import { useEffect, useLayoutEffect, useState } from 'react'
import { useLocation, useNavigate, useNavigationType } from 'react-router-dom'
import { activeTheme, applyToDocument, persistTheme, useDraft, useThemes } from '../data/themes'
import { setMusicAudible, setMusicSource, unlockMusic } from '../audio/menuMusic'
import { call, isDesktopApp } from '../native'

/**
 * Capa invisible de la ventana principal (se monta una vez en App):
 * - aplica el tema activo,
 * - decide cuándo suena la música del menú,
 * - marca si se va hacia delante o hacia atrás (para la transición entre pantallas),
 * - hunde un poco el elemento con foco al pulsar A.
 */
export function ThemeMusic() {
  useThemeSync()
  useRouteDirection()
  usePressFeedback()
  useMenuMusic()
  useLaunchRoute()
  return null
}

function useThemeSync() {
  const { active } = useThemes()
  // También renueva las variables guardadas (por si cambió la paleta en una actualización)
  useLayoutEffect(() => { applyToDocument(); persistTheme() }, [active])
}

/** Volver (B, Esc, atrás) = "back"; el resto = "forward". Se fija antes de pintar la pantalla nueva */
function useRouteDirection() {
  const location = useLocation()
  const type = useNavigationType()
  useLayoutEffect(() => {
    document.documentElement.dataset.navDir = type === 'POP' ? 'back' : 'forward'
  }, [location.key, type])
}

function usePressFeedback() {
  useEffect(() => {
    let timer = 0
    const press = () => {
      const el = document.querySelector<HTMLElement>('[data-nav][data-focused]')
      if (!el) return
      el.setAttribute('data-pressed', '')
      clearTimeout(timer)
      timer = window.setTimeout(() => el.removeAttribute('data-pressed'), 140)
    }
    const onPad = (e: Event) => { if ((e as CustomEvent).detail === 'accept') press() }
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Enter' && !e.repeat) press() }
    window.addEventListener('padkey', onPad)
    window.addEventListener('keydown', onKey)
    return () => { window.removeEventListener('padkey', onPad); window.removeEventListener('keydown', onKey); clearTimeout(timer) }
  }, [])
}

/** Pantallas donde la música del menú no suena (tienen su propio sonido) */
const QUIET = [/^\/ambiente\/.+/, /^\/navegador/, /^\/nube/]

/** ¿Hay un vídeo o audio con sonido en la página? */
function pageHasSound() {
  return Array.from(document.querySelectorAll<HTMLMediaElement>('video, audio')).some((m) => !m.paused && !m.muted && m.volume > 0)
}

function useMenuMusic() {
  const location = useLocation()
  const { active } = useThemes()
  const draft = useDraft()
  const [focused, setFocused] = useState(() => !document.hidden && document.hasFocus())
  const [mediaPlaying, setMediaPlaying] = useState(false)
  const [pageSound, setPageSound] = useState(false)
  const [saver, setSaver] = useState(false)

  // Qué suena: la del tema, o la que se prueba en el editor de temas
  const editing = /^\/ajustes\/temas\/(nuevo|[^/]+\/editar)/.test(location.pathname) && draft
  const music = editing ? draft.music : activeTheme().music
  useEffect(() => { setMusicSource(music) }, [JSON.stringify(music), active]) // eslint-disable-line react-hooks/exhaustive-deps

  // Primera pulsación: ya se puede sonar
  useEffect(() => {
    const go = () => unlockMusic()
    const opts = { once: true, capture: true } as const
    window.addEventListener('keydown', go, opts)
    window.addEventListener('pointerdown', go, opts)
    window.addEventListener('padinput', go, opts)
    return () => {
      window.removeEventListener('keydown', go, opts)
      window.removeEventListener('pointerdown', go, opts)
      window.removeEventListener('padinput', go, opts)
    }
  }, [])

  // Ventana delante y con foco (un juego, el navegador o la nube se lo quitan)
  useEffect(() => {
    const update = () => setFocused(!document.hidden && document.hasFocus())
    document.addEventListener('visibilitychange', update)
    window.addEventListener('focus', update)
    window.addEventListener('blur', update)
    return () => {
      document.removeEventListener('visibilitychange', update)
      window.removeEventListener('focus', update)
      window.removeEventListener('blur', update)
    }
  }, [])

  // Vídeos con sonido dentro de la consola
  useEffect(() => {
    const update = () => setPageSound(pageHasSound())
    const events = ['play', 'pause', 'ended', 'volumechange', 'emptied']
    events.forEach((e) => document.addEventListener(e, update, true))
    return () => events.forEach((e) => document.removeEventListener(e, update, true))
  }, [])

  // Spotify u otro reproductor sonando en el PC (solo en la app; se mira cada pocos segundos)
  useEffect(() => {
    if (!isDesktopApp || !focused) return
    let alive = true
    const check = () => call<{ playing: boolean; app: string; appId: string } | null>('media_now_playing')
      .then((t) => { if (alive) setMediaPlaying(!!t?.playing && !/webview|consola|couchdesk/i.test(`${t.app} ${t.appId}`)) })
      .catch(() => {})
    check()
    const timer = window.setInterval(check, 5000)
    return () => { alive = false; clearInterval(timer) }
  }, [focused])

  // El salvapantallas está encima (no quita el foco, pero tiene su propio ambiente)
  useEffect(() => {
    if (!isDesktopApp) return
    let off: (() => void) | undefined
    let alive = true
    import('@tauri-apps/api/event').then(({ listen }) =>
      listen<{ visible: boolean }>('salvapantallas', (e) => setSaver(e.payload.visible)).then((u) => { if (alive) off = u; else u() }),
    )
    return () => { alive = false; off?.() }
  }, [])

  const quietRoute = QUIET.some((r) => r.test(location.pathname))
  const audible = focused && !quietRoute && !mediaPlaying && !pageSound && !saver
  useEffect(() => { setMusicAudible(audible) }, [audible])
  useEffect(() => () => setMusicAudible(false), [])
}

/** Si la consola se abrió con "/c" (configurar salvapantallas), ir a esa pantalla */
function useLaunchRoute() {
  const navigate = useNavigate()
  useEffect(() => {
    if (!isDesktopApp) return
    call<string | null>('launch_route_take').then((r) => { if (r && r.startsWith('/')) navigate(r) }).catch(() => {})
  }, [navigate])
}
