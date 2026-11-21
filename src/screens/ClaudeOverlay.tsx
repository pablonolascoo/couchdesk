import { useEffect, useRef } from 'react'
import { Plus, X, Sparkle } from '@phosphor-icons/react'
import { useSpatialNav } from '../nav/useSpatialNav'
import { call, isDesktopApp } from '../native'
import { useT } from '../i18n'
import {
  suggestions, useClaudeChat, useVoice, useHoldToTalk, currentConversation, setCurrentConversation, stopSpeaking,
} from '../data/claude'
import { Bubbles, AskBox, VoiceError } from './Claude'
import './Claude.css'
import './ClaudeOverlay.css'

const hide = () => { stopSpeaking(); call('assistant_hide').catch(() => {}) }

/** /asistente-flotante: Claude en una ventanita encima de juegos y programas (ventana "asistente") */
export function ClaudeOverlay() {
  const t = useT()
  useSpatialNav()
  const chat = useClaudeChat()
  const voice = useVoice((heard) => { if (heard.trim()) chat.ask(heard); else voice.setError(t('claude.voice.notHeard')) })
  useHoldToTalk(voice.start, voice.stop)
  const open = useRef(chat.open)
  open.current = chat.open

  // Al aparecer: seguir la última conversación (puede haber cambiado en la ventana principal)
  useEffect(() => {
    open.current(currentConversation())
    if (!isDesktopApp) return
    let off: (() => void) | undefined
    import('@tauri-apps/api/event').then(({ listen }) =>
      listen('assistant-shown', () => open.current(currentConversation())).then((u) => { off = u }),
    )
    return () => off?.()
  }, [])

  // Esc o B: esconder (antes que la navegación, que intentaría ir "atrás")
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      e.preventDefault()
      e.stopImmediatePropagation()
      hide()
    }
    document.addEventListener('keydown', onKey, true)
    let raf = 0
    let was = true // si la ventana aparece con B pulsado, esperar a que se suelte
    const tick = () => {
      const pad = navigator.getGamepads?.().find((p) => p)
      const b = !!pad && document.hasFocus() && (pad.buttons[1]?.pressed ?? false)
      if (b && !was) hide()
      was = b
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => { document.removeEventListener('keydown', onKey, true); cancelAnimationFrame(raf) }
  }, [])

  const fresh = () => { setCurrentConversation(null); chat.open(null) }

  return (
    <div className="overlay">
      <header className="overlay-head">
        <span className="overlay-title"><Sparkle size={18} weight="fill" /> Claude</span>
        <button className="c-chip" data-nav="ov-new" onClick={fresh}><Plus size={16} weight="light" /> {t('claude.overlay.new')}</button>
        <button className="overlay-close" data-nav="ov-close" aria-label={t('claude.overlay.close')} onClick={hide}><X size={18} weight="light" /></button>
      </header>

      <Bubbles msgs={chat.msgs} busy={chat.busy} empty={t('claude.overlay.empty')} />

      {chat.msgs.length === 0 && (
        <div className="overlay-chips">
          {suggestions().slice(0, 4).map((s, i) => (
            <button key={s} className="c-chip" data-nav={`ov-sug-${i}`} onClick={() => chat.ask(s)} disabled={chat.busy}>{s}</button>
          ))}
        </div>
      )}

      <AskBox chat={chat} voice={voice} idPrefix="ov-" />
      <VoiceError voice={voice} idPrefix="ov-" />

      <footer className="overlay-foot">
        <span><kbd>X</kbd> {t('claude.hint.talk')}</span>
        <span><kbd>B</kbd> {t('claude.overlay.close')}</span>
        <span className="overlay-tip">{t('claude.overlay.tip')}</span>
      </footer>
    </div>
  )
}
