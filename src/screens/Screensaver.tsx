import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Sun, Moon, Cloud, CloudSun, CloudMoon, CloudFog, CloudRain, CloudSnow, CloudLightning, type Icon } from '@phosphor-icons/react'
import { VideoLoop } from '../components/VideoLoop'
import { SaverSound, setSaverClip } from '../components/SaverSound'
import { landscapes, type Landscape } from '../data/ambience'
import { loadSettings, weatherNow, type SaverSettings, type Weather } from '../data/screensaver'
import { call, isDesktopApp } from '../native'
import { fmt, useT } from '../i18n'
import './Screensaver.css'

/** Cada cuánto cambia el paisaje (con fundido lento) */
const CHANGE_MS = 90_000
const FADE_MS = 3_000
const WEATHER_MS = 20 * 60_000

export const weatherIcons: Record<string, Icon> = { Sun, Moon, Cloud, CloudSun, CloudMoon, CloudFog, CloudRain, CloudSnow, CloudLightning }

const shuffle = <T,>(xs: T[]) => [...xs].sort(() => Math.random() - 0.5)

/**
 * Página de la ventana "salvapantallas". La enseña y la esconde la parte nativa
 * (y avisa con el evento "salvapantallas"); mientras está oculta no hay nada montado.
 * En el navegador (/salvapantallas?prueba=1) se ve directamente, para probarla.
 */
export function Screensaver() {
  const [active, setActive] = useState(!isDesktopApp)
  const [settings, setSettings] = useState<SaverSettings | null>(null)

  // Esta página vive sola en su ventana: fondo negro desde el principio
  useEffect(() => {
    document.body.style.background = '#000'
    return () => { document.body.style.background = '' }
  }, [])

  useEffect(() => {
    if (!isDesktopApp) return
    let stop: (() => void) | undefined
    let alive = true
    import('@tauri-apps/api/event').then(({ listen }) =>
      listen<{ visible: boolean }>('salvapantallas', (e) => setActive(e.payload.visible)).then((u) => {
        if (alive) stop = u
        else u()
      }),
    )
    // Por si se enseñó antes de que la página escuchara
    call<{ visible: boolean }>('screensaver_status').then((s) => setActive(s.visible)).catch(() => {})
    return () => { alive = false; stop?.() }
  }, [])

  useEffect(() => {
    if (active) loadSettings().then(setSettings)
    else setSettings(null)
  }, [active])

  const exit = useCallback(() => {
    if (isDesktopApp) call('screensaver_hide').catch(() => {})
    else if (window.history.length > 1) window.history.back()
    else window.location.assign('/')
  }, [])

  if (!active || !settings) return <div className="ss ss-off" />
  return <Scene settings={settings} onExit={exit} />
}

function Scene({ settings, onExit }: { settings: SaverSettings; onExit: () => void }) {
  const style = settings.style
  const lowPower = style === 'bajo-consumo'
  useT() // reloj y fecha en el idioma elegido
  const [now, setNow] = useState(() => new Date())
  const [nudge, setNudge] = useState({ x: 0, y: 0 })
  const [weather, setWeather] = useState<Weather | null>(null)

  // Reloj: una sola actualización por minuto, en el cambio de minuto. Y un pequeño
  // desplazamiento cada vez para no marcar la pantalla (en bajo consumo, más amplio).
  useEffect(() => {
    let t: number
    const tick = () => {
      const d = new Date()
      setNow(d)
      const r = () => Math.random() * 2 - 1
      setNudge(lowPower
        ? { x: r() * window.innerWidth * 0.18, y: r() * window.innerHeight * 0.16 }
        : { x: r() * 14, y: r() * 10 })
      t = window.setTimeout(tick, 60_000 - (d.getSeconds() * 1000 + d.getMilliseconds()) + 50)
    }
    const d = new Date()
    t = window.setTimeout(tick, 60_000 - (d.getSeconds() * 1000 + d.getMilliseconds()) + 50)
    return () => clearTimeout(t)
  }, [lowPower])

  // El tiempo (la parte nativa lo guarda 20 minutos)
  useEffect(() => {
    if (!settings.showWeather || lowPower) return
    const load = () => weatherNow().then(setWeather).catch(() => setWeather(null))
    load()
    const t = window.setInterval(load, WEATHER_MS)
    return () => clearInterval(t)
  }, [settings.showWeather, lowPower])

  // Cualquier tecla, clic o movimiento claro del ratón sale (la parte nativa también lo vigila,
  // aunque el foco esté en un juego). Al principio, un momento de gracia.
  useEffect(() => {
    const start = performance.now()
    let origin: { x: number; y: number } | null = null
    const ready = () => performance.now() - start > 1000
    const leave = () => { if (ready()) onExit() }
    const onMove = (e: MouseEvent) => {
      if (!ready()) return
      if (!origin) { origin = { x: e.clientX, y: e.clientY }; return }
      if (Math.abs(e.clientX - origin.x) > 12 || Math.abs(e.clientY - origin.y) > 12) onExit()
    }
    window.addEventListener('keydown', leave)
    window.addEventListener('mousedown', leave)
    window.addEventListener('wheel', leave)
    window.addEventListener('mousemove', onMove)
    return () => {
      window.removeEventListener('keydown', leave)
      window.removeEventListener('mousedown', leave)
      window.removeEventListener('wheel', leave)
      window.removeEventListener('mousemove', onMove)
    }
  }, [onExit])

  const time = fmt.time(now)
  const date = fmt.date(now)
  const WIcon = weather ? weatherIcons[weather.icon] ?? Cloud : null

  return (
    <div className={`ss ss-${style}`}>
      {style === 'paisajes' && <Landscapes collections={settings.collections} sound={!!settings.sound} />}
      <SaverSound settings={settings} />
      {!lowPower && <div className="ss-shade" aria-hidden />}

      <div className="ss-center" style={{ transform: `translate(${nudge.x}px, ${nudge.y}px)` }}>
        <div className="ss-time">{time}</div>
        <div className="ss-date">{date}</div>
      </div>

      {weather && WIcon && (
        <div className="ss-weather">
          <WIcon size={30} weight="light" />
          <span className="ss-temp">{Math.round(weather.temperature)}°</span>
          <span className="ss-place">{weather.place} · {weather.label}</span>
        </div>
      )}
    </div>
  )
}

/** Paisajes en movimiento: cambia cada CHANGE_MS con un fundido; salta los que fallan */
function Landscapes({ collections, sound = false }: { collections: string[]; sound?: boolean }) {
  const list = useMemo(() => {
    const chosen = landscapes.filter((l) => collections.includes(l.collection))
    return shuffle(chosen.length ? chosen : landscapes)
  }, [collections])
  const [layers, setLayers] = useState<{ key: number; clip: Landscape }[]>(() => [{ key: 0, clip: list[0] }])
  const [failed, setFailed] = useState(0)
  const counter = useRef(0)

  const next = () => {
    counter.current += 1
    const clip = list[counter.current % list.length]
    setLayers((ls) => [...ls.slice(-1), { key: counter.current, clip }])
  }

  useEffect(() => {
    const t = window.setInterval(next, CHANGE_MS)
    return () => clearInterval(t)
  }, [list]) // eslint-disable-line react-hooks/exhaustive-deps

  // Tras el fundido, fuera la capa de abajo (deja de descargar y reproducir)
  useEffect(() => {
    if (layers.length < 2) return
    const t = window.setTimeout(() => setLayers((ls) => ls.slice(-1)), FADE_MS + 400)
    return () => clearTimeout(t)
  }, [layers])

  const current = layers[layers.length - 1]
  useEffect(() => { setSaverClip(current.clip); return () => setSaverClip(null) }, [current.clip])
  const t = useT()
  // Sin conexión: si fallan todos, queda el fondo cálido del estilo "Solo reloj"
  if (failed >= list.length) return null

  return (
    <>
      {layers.map((l) => (
        <div key={l.key} className="ss-layer" style={{ ['--change' as string]: `${CHANGE_MS + FADE_MS}ms` }}>
          <VideoLoop src={l.clip.url} rate={l.clip.rate} muted={!(sound && l.clip.source === 'youtube')} onError={() => { setFailed((f) => f + 1); next() }} />
        </div>
      ))}
      <p className="ss-credit">{t('screensaver.credit', { name: current.clip.credit })}</p>
    </>
  )
}
