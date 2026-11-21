import { useEffect, useRef, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { SpeakerHigh, SpeakerSlash, Clock, ArrowLeft, FilmSlate, WifiSlash, ArrowClockwise } from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { VideoLoop } from '../components/VideoLoop'
import { ambience, ambienceCredits, type Clip } from '../data/ambience'
import { inhibitSaver } from '../data/screensaver'
import { fmt, useT } from '../i18n'
import './Ambience.css'

const pref = (k: string, def: boolean) => {
  try { const v = localStorage.getItem(`consola.ambiente.${k}`); return v === null ? def : v === '1' } catch { return def }
}
const setPref = (k: string, v: boolean) => { try { localStorage.setItem(`consola.ambiente.${k}`, v ? '1' : '0') } catch { /* nada */ } }

/** ¿Hay internet? (se actualiza solo) */
function useOnline() {
  const [online, setOnline] = useState(navigator.onLine)
  useEffect(() => {
    const on = () => setOnline(navigator.onLine)
    window.addEventListener('online', on)
    window.addEventListener('offline', on)
    return () => { window.removeEventListener('online', on); window.removeEventListener('offline', on) }
  }, [])
  return online
}

// ---------------- /ambiente ----------------

export function AmbienceGrid() {
  const navigate = useNavigate()
  const online = useOnline()
  const t = useT()

  if (!online) {
    return (
      <Page title={t('ambience.title')}>
        <div className="empty-state">
          <span className="empty-icon"><WifiSlash size={26} weight="light" /></span>
          <h2>{t('ambience.offline.title')}</h2>
          <p>{t('ambience.offline.text')}</p>
          <button className="pill" data-nav="amb-retry" data-nav-default="" onClick={() => window.location.reload()}>
            {t('ambience.retry')}
            <span className="pill-icon"><ArrowClockwise size={16} weight="light" /></span>
          </button>
        </div>
      </Page>
    )
  }

  return (
    <Page wide title={t('ambience.title')} hints={[['A', t('ambience.hint.view')], ['B', t('common.back')]]}>
      <p className="amb-lead">{t('ambience.lead')}</p>
      <div className="amb-grid">
        {ambience.map((a, i) => (
          <button
            key={a.id}
            className="amb-tile"
            data-nav={`amb-${a.id}`}
            data-nav-default={i === 0 ? '' : undefined}
            onClick={() => navigate(`/ambiente/${a.id}`)}
          >
            <span className="amb-thumb">
              {/* Portadas de Pexels primero: limpias, sin rótulos de YouTube */}
              {a.fallback?.poster ?? a.poster
                ? <img src={a.fallback?.poster ?? a.poster} alt="" loading="lazy" draggable={false} />
                : <FilmSlate size={34} weight="light" />}
            </span>
            <span className="amb-name">{a.title}</span>
            <span className="amb-cat">{a.category}</span>
          </button>
        ))}
      </div>
      <p className="amb-credits">{ambienceCredits()}</p>
    </Page>
  )
}

// ---------------- /ambiente/:id ----------------

/** Hora que cambia una vez por minuto */
function useMinuteClock() {
  const [now, setNow] = useState(() => new Date())
  useEffect(() => {
    let t: number
    const schedule = () => {
      const d = new Date()
      t = window.setTimeout(() => { setNow(new Date()); schedule() }, 60_000 - (d.getSeconds() * 1000 + d.getMilliseconds()) + 50)
    }
    schedule()
    return () => clearTimeout(t)
  }, [])
  return now
}

function AmbienceClock() {
  const now = useMinuteClock()
  useT()
  return <div className="amb-clock">{fmt.time(now)}</div>
}

export function AmbiencePlayer() {
  const { id } = useParams()
  const navigate = useNavigate()
  const t = useT()
  const entry = ambience.find((a) => a.id === id)
  const [useFallback, setUseFallback] = useState(false)
  const [failed, setFailed] = useState(false)
  const [sound, setSound] = useState(() => pref('sonido', false))
  const [clock, setClock] = useState(() => pref('reloj', false))
  const [ui, setUi] = useState(true)
  const hideTimer = useRef<number>(undefined)

  // Mientras se ve, el salvapantallas no lo tapa
  useEffect(() => {
    inhibitSaver(true)
    return () => inhibitSaver(false)
  }, [])

  // La capa con el nombre y los botones se va sola; vuelve con cualquier tecla, ratón o mando
  useEffect(() => {
    const wake = () => {
      setUi(true)
      clearTimeout(hideTimer.current)
      hideTimer.current = window.setTimeout(() => setUi(false), 4000)
    }
    wake()
    const evs = ['keydown', 'mousemove', 'mousedown', 'padkey', 'navfocus'] as const
    evs.forEach((e) => window.addEventListener(e, wake, true))
    return () => { clearTimeout(hideTimer.current); evs.forEach((e) => window.removeEventListener(e, wake, true)) }
  }, [])

  if (!entry) {
    return (
      <Page title={t('ambience.title')}>
        <div className="empty-state">
          <span className="empty-icon"><FilmSlate size={26} weight="light" /></span>
          <h2>{t('ambience.missing.title')}</h2>
          <p>{t('ambience.missing.text')}</p>
          <button className="pill" data-nav="amb-all" data-nav-default="" onClick={() => navigate('/ambiente', { replace: true })}>
            {t('ambience.seeAll')}
            <span className="pill-icon"><ArrowLeft size={16} weight="light" /></span>
          </button>
        </div>
      </Page>
    )
  }

  const clip: Clip = useFallback && entry.fallback ? entry.fallback : entry
  const onError = () => {
    if (entry.fallback && !useFallback) setUseFallback(true)
    else setFailed(true)
  }
  const toggleSound = () => { setSound((v) => { setPref('sonido', !v); return !v }) }
  const toggleClock = () => { setClock((v) => { setPref('reloj', !v); return !v }) }

  return (
    <div className={`amb-player ${ui ? 'show-ui' : ''}`}>
      <VideoLoop key={clip.url} src={clip.url} poster={clip.poster} muted={!sound} onError={onError} />
      <div className="amb-shade" aria-hidden />

      {clock && <AmbienceClock />}

      {failed && (
        <div className="amb-error">
          <WifiSlash size={26} weight="light" />
          <p>{t('ambience.error')}</p>
        </div>
      )}

      <div className="amb-top">
        <span className="amb-eyebrow">{entry.category}</span>
        <h1 className="amb-title">{entry.title}</h1>
      </div>

      <div className="amb-bar">
        <button className="amb-btn" data-nav="amb-sound" data-nav-default="" onClick={toggleSound} aria-pressed={sound}>
          {sound ? <SpeakerHigh size={20} weight="light" /> : <SpeakerSlash size={20} weight="light" />}
          {sound ? t('ambience.soundOff') : t('ambience.soundOn')}
        </button>
        <button className="amb-btn" data-nav="amb-clock" onClick={toggleClock} aria-pressed={clock}>
          <Clock size={20} weight="light" />
          {clock ? t('ambience.clockHide') : t('ambience.clockShow')}
        </button>
        <button className="amb-btn" data-nav="amb-back" onClick={() => (window.history.state?.idx > 0 ? navigate(-1) : navigate('/ambiente'))}>
          <ArrowLeft size={20} weight="light" />
          {t('ambience.back')}
        </button>
        <span className="amb-credit">{clip.source === 'pexels' ? t('ambience.creditPexels', { name: clip.credit }) : `${clip.credit} · YouTube`}</span>
      </div>
    </div>
  )
}
