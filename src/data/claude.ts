// Claude en la consola: datos, eventos y voz, compartidos por /claude y el asistente flotante
import { useCallback, useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { call, isDesktopApp } from '../native'
import { t, fmt, getLanguage } from '../i18n'

export type Role = 'user' | 'claude' | 'tool' | 'error'
export interface Msg { role: Role; text: string }
export interface ConvSummary { id: string; title: string; updatedAt: number; count: number }
export interface Conversation { id: string; title: string; updatedAt: number; messages: Msg[] }
export interface Status { installed: boolean; loggedIn: boolean | null }
export interface Connector { name: string; url: string; description: string; enabled: boolean }
export interface CatalogItem { name: string; title: string; url: string; description: string }
export interface Plugin { id: string; version?: string; enabled?: boolean; managed?: boolean; name?: string; description?: string; installs?: number }

/** Ideas para empezar (en el idioma actual: lo que se pulse se envía tal cual a Claude) */
export const suggestions = () => [
  t('claude.sug.volume'),
  t('claude.sug.library'),
  t('claude.sug.youtube'),
  t('claude.sug.brightness'),
  t('claude.sug.emulators'),
  t('claude.sug.battery'),
]

/** Texto de un conector conocido del catálogo, traducido por su id; si no lo conocemos, el que trae */
export function connectorDescription(name: string, fallback: string) {
  const key = `claude.connector.${name}.desc`
  const text = t(key)
  return text === key ? fallback : text
}

// ---------------- Datos de prueba (solo en el navegador, sin la app) ----------------

const now = Date.now()
export const mocks = {
  status: { installed: true, loggedIn: true } as Status,
  get conversations() {
    return [
      { id: 'demo-1', title: t('claude.sug.volume'), updatedAt: now - 60_000, count: 3 },
      { id: 'demo-2', title: t('claude.sug.library'), updatedAt: now - 3_600_000, count: 3 },
      { id: 'demo-3', title: t('claude.sug.youtube'), updatedAt: now - 86_400_000 * 2, count: 3 },
    ] as ConvSummary[]
  },
  get conversation() {
    return {
      id: 'demo-1', title: t('claude.sug.volume'), updatedAt: now,
      messages: [
        { role: 'user', text: t('claude.sug.volume') },
        { role: 'tool', text: t('claude.mock.tool') },
        { role: 'claude', text: t('claude.mock.reply') },
      ],
    } as Conversation
  },
  get connectors() {
    return {
      mine: [{ name: 'deepwiki', url: 'https://mcp.deepwiki.com/mcp', description: t('claude.connector.deepwiki.desc'), enabled: true }] as Connector[],
      catalog: [
        { name: 'deepwiki', title: 'DeepWiki', url: 'https://mcp.deepwiki.com/mcp', description: t('claude.connector.deepwiki.desc') },
        { name: 'context7', title: 'Context7', url: 'https://mcp.context7.com/mcp', description: t('claude.connector.context7.desc') },
        { name: 'microsoft-learn', title: 'Microsoft Learn', url: 'https://learn.microsoft.com/api/mcp', description: t('claude.connector.microsoft-learn.desc') },
        { name: 'huggingface', title: 'Hugging Face', url: 'https://huggingface.co/mcp', description: t('claude.connector.huggingface.desc') },
      ] as CatalogItem[],
    }
  },
  get plugins() {
    return {
      installed: [
        { id: 'pdf-viewer@synced', version: '0.2.0', enabled: true, managed: true },
        { id: 'mi-extension@claude-plugins-official', version: '1.0.0', enabled: true, managed: false },
      ] as Plugin[],
      available: [
        { id: 'figma@claude-plugins-official', name: 'figma', description: t('claude.mock.figma'), installs: 12000 },
        { id: 'github@claude-plugins-official', name: 'github', description: t('claude.mock.github'), installs: 9000 },
      ] as Plugin[],
    }
  },
}

// ---------------- Conversación actual (compartida entre ventanas) ----------------

const CURRENT = 'consola.claude.actual'
export function currentConversation(): string | null {
  try { return localStorage.getItem(CURRENT) } catch { return null }
}
export function setCurrentConversation(id: string | null) {
  try { if (id) localStorage.setItem(CURRENT, id); else localStorage.removeItem(CURRENT) } catch { /* sin almacenamiento */ }
}

export const ago = (ms: number) => {
  if (Date.now() - ms < 60_000) return t('claude.ago.now')
  const text = fmt.relative(new Date(ms))
  return text.charAt(0).toLocaleUpperCase(getLanguage()) + text.slice(1)
}

// ---------------- Leer en voz alta ----------------

const SPEAK = 'consola.claude.leer'
export function speakEnabled() {
  try { return localStorage.getItem(SPEAK) === 'si' } catch { return false }
}
export function useSpeakSetting() {
  const [on, setOn] = useState(speakEnabled)
  const toggle = () => {
    const next = !on
    setOn(next)
    try { localStorage.setItem(SPEAK, next ? 'si' : 'no') } catch { /* sin almacenamiento */ }
    if (!next) stopSpeaking()
  }
  return [on, toggle] as const
}
export function speak(text: string) {
  if (!('speechSynthesis' in window) || !speakEnabled()) return
  const clean = text.replace(/[*_#`>|]/g, '').replace(/\[(.*?)\]\(.*?\)/g, '$1').trim()
  if (!clean) return
  const u = new SpeechSynthesisUtterance(clean)
  const voices = speechSynthesis.getVoices()
  const lang = getLanguage()
  const base = lang.split('-')[0]
  const voice = voices.find((v) => v.lang.toLowerCase() === lang.toLowerCase())
    ?? (lang === 'es' ? voices.find((v) => v.lang === 'es-ES') : undefined)
    ?? voices.find((v) => v.lang.toLowerCase().startsWith(base))
  if (voice) u.voice = voice
  u.lang = voice?.lang ?? lang
  speechSynthesis.speak(u)
}
export function stopSpeaking() {
  if ('speechSynthesis' in window) speechSynthesis.cancel()
}

// ---------------- Chat ----------------

const newTag = () => Math.random().toString(36).slice(2) + Date.now().toString(36)

/**
 * Conversación con Claude. `id` null = nueva. Los eventos llevan la etiqueta de esta pantalla,
 * así la ventana flotante y la principal no se mezclan.
 */
export function useClaudeChat(onSession?: (id: string) => void) {
  const [id, setId] = useState<string | null>(null)
  const [msgs, setMsgs] = useState<Msg[]>([])
  const [busy, setBusy] = useState(false)
  const tag = useRef(newTag())
  const sessionCb = useRef(onSession)
  sessionCb.current = onSession

  const push = (m: Msg) => setMsgs((prev) => [...prev, m])

  useEffect(() => {
    if (!isDesktopApp) return
    let off: (() => void) | undefined
    import('@tauri-apps/api/event').then(({ listen }) =>
      listen<{ tag: string; kind: string; text: string }>('claude-event', ({ payload: p }) => {
        if (p.tag !== tag.current) return
        if (p.kind === 'session') { setId(p.text); setCurrentConversation(p.text); sessionCb.current?.(p.text) }
        if (p.kind === 'text' && p.text.trim()) { push({ role: 'claude', text: p.text }); speak(p.text) }
        if (p.kind === 'tool') push({ role: 'tool', text: p.text })
        if (p.kind === 'error') push({ role: 'error', text: p.text })
        if (p.kind === 'done') setBusy(false)
      }).then((u) => { off = u }),
    )
    return () => off?.()
  }, [])

  /** Carga una conversación guardada (o empieza una nueva con null) */
  const open = useCallback(async (conv: string | null) => {
    setId(conv)
    if (!conv) { setMsgs([]); return }
    try {
      const c = await call<Conversation>('claude_conversation', { id: conv }, { ...mocks.conversation, id: conv })
      setMsgs(c.messages)
      setCurrentConversation(conv)
    } catch {
      setId(null)
      setMsgs([])
      setCurrentConversation(null)
    }
  }, [])

  const ask = useCallback(async (q: string) => {
    const text = q.trim()
    if (!text || busy) return
    stopSpeaking()
    push({ role: 'user', text })
    setBusy(true)
    try {
      await call('claude_ask', { prompt: text, conversation: id, tag: tag.current })
      if (!isDesktopApp) {
        setTimeout(() => {
          const sid = id ?? 'demo-nueva'
          setId(sid)
          sessionCb.current?.(sid)
          push({ role: 'tool', text: t('claude.mock.tool') })
          push({ role: 'claude', text: t('claude.mock.reply') })
          setBusy(false)
        }, 700)
      }
    } catch (e) {
      push({ role: 'error', text: String(e) })
      setBusy(false)
    }
  }, [busy, id])

  return { id, msgs, busy, ask, open }
}

/** ¿Claude Code instalado y con sesión? Cambiar `key` lo vuelve a comprobar */
export function useClaudeStatus(key = 0) {
  const [status, setStatus] = useState<Status | null>(null)
  useEffect(() => { call<Status>('claude_status', {}, mocks.status).then(setStatus).catch(() => setStatus({ installed: false, loggedIn: null })) }, [key])
  return status
}

// ---------------- Voz ----------------

/** Qué página de Configuración arregla el error de voz */
export function voiceSettingsPage(error: string) {
  // Los textos vienen de Rust en el idioma actual (src-tauri/src/i18n.rs: voice.mic_blocked, voice.no_language)
  if (/›\s*(micr[oó]fono|microphone|microfone|mikrofon|microfono)\.?\s*$/i.test(error)) return 'privacy-microphone'
  if (/paquete de voz|speech pack|pacote de fala|pack vocal|sprachpaket|pacchetto vocale/i.test(error)) return 'speech'
  return 'privacy-speech'
}

let windowLabel: Promise<string> | null = null
const myLabel = () => {
  windowLabel ??= import('@tauri-apps/api/window').then((m) => m.getCurrentWindow().label)
  return windowLabel
}

/** Hablar manteniendo pulsado. Al soltar llama a `onFinal` con lo que se ha entendido. */
export function useVoice(onFinal: (text: string) => void) {
  const [listening, setListening] = useState(false)
  const [partial, setPartial] = useState('')
  const [error, setError] = useState<string | null>(null)
  const final = useRef(onFinal)
  final.current = onFinal
  const active = useRef(false)

  useEffect(() => {
    if (!isDesktopApp) return
    const offs: (() => void)[] = []
    let alive = true
    Promise.all([import('@tauri-apps/api/event'), myLabel()]).then(([{ listen }, label]) => {
      const on = (name: string, fn: (text: string) => void) =>
        listen<{ window: string; text: string }>(name, ({ payload }) => { if (payload.window === label) fn(payload.text) })
          .then((u) => { if (alive) offs.push(u); else u() })
      on('voice-partial', setPartial)
      on('voice-final', (text) => { setListening(false); setPartial(''); final.current(text) })
      on('voice-error', (text) => { setListening(false); setPartial(''); setError(text) })
    })
    return () => { alive = false; offs.forEach((f) => f()) }
  }, [])

  const start = useCallback(async () => {
    if (active.current) return
    active.current = true
    stopSpeaking()
    setError(null)
    setPartial('')
    setListening(true)
    if (!isDesktopApp) { setPartial(t('claude.mock.partial')); return }
    try {
      await call('voice_start')
      // Si soltaron mientras arrancaba, parar ya
      if (!active.current) call('voice_stop').catch(() => {})
    } catch (e) {
      active.current = false
      setListening(false)
      setError(String(e))
    }
  }, [])

  const stop = useCallback(() => {
    if (!active.current) return
    active.current = false
    if (!isDesktopApp) { setListening(false); setPartial(''); final.current(t('claude.sug.volume')); return }
    call('voice_stop').catch(() => {})
  }, [])

  const openSettings = () => { if (error) call('voice_open_settings', { page: voiceSettingsPage(error) }).catch(() => {}) }

  return { listening, partial, error, setError, start, stop, openSettings }
}

/**
 * Mantener X en el mando (o Espacio en el teclado, fuera de un campo de texto) = hablar.
 * Solo con la ventana delante.
 */
export function useHoldToTalk(start: () => void, stop: () => void) {
  const cb = useRef({ start, stop })
  cb.current = { start, stop }
  useEffect(() => {
    let raf = 0
    let held = false
    const tick = () => {
      const pad = navigator.getGamepads?.().find((p) => p)
      const x = !!pad && document.hasFocus() && (pad.buttons[2]?.pressed ?? false)
      if (x && !held) { held = true; cb.current.start() }
      else if (!x && held) { held = false; cb.current.stop() }
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    const typing = (t: EventTarget | null) => t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement
    let key = false
    const down = (e: KeyboardEvent) => {
      if (e.key !== ' ' || typing(e.target) || e.ctrlKey || e.altKey || e.metaKey) return
      e.preventDefault()
      e.stopImmediatePropagation()
      if (!key && !e.repeat) { key = true; cb.current.start() }
    }
    const up = (e: KeyboardEvent) => {
      if (e.key !== ' ' || !key) return
      e.preventDefault()
      e.stopImmediatePropagation()
      key = false
      cb.current.stop()
    }
    // Fase de captura: antes que la navegación (que usa Espacio para "aceptar")
    document.addEventListener('keydown', down, true)
    document.addEventListener('keyup', up, true)
    return () => {
      cancelAnimationFrame(raf)
      document.removeEventListener('keydown', down, true)
      document.removeEventListener('keyup', up, true)
      if (held || key) cb.current.stop()
    }
  }, [])
}

// ---------------- Claude maneja la consola ----------------

/** En la ventana principal: Claude pide abrir una pantalla (herramienta abrir_pantalla) */
export function useClaudeNavigate() {
  const navigate = useNavigate()
  useEffect(() => {
    if (!isDesktopApp) return
    let off: (() => void) | undefined
    let alive = true
    Promise.all([import('@tauri-apps/api/event'), myLabel()]).then(([{ listen }, label]) => {
      if (label !== 'main' || !alive) return
      listen<string>('claude-navigate', ({ payload }) => { if (payload.startsWith('/')) navigate(payload) })
        .then((u) => { if (alive) off = u; else u() })
    })
    return () => { alive = false; off?.() }
  }, [navigate])
}
