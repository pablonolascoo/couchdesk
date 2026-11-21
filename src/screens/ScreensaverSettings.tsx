import { useEffect, useRef, useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { CheckCircle, Mountains, MoonStars, Clock, MapPin, NavigationArrow, Play, MagnifyingGlass, CaretRight } from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { collections, type CollectionId } from '../data/ambience'
import {
  loadSettings, saveSettings, locateApprox, searchPlaces, previewSaver,
  type SaverSettings, type SaverStyle, type Place,
} from '../data/screensaver'
import { useT } from '../i18n'
import './ScreensaverSettings.css'
import { SaverExtras } from './SaverExtras'

const AFTER = [1, 2, 5, 10, 15, 30]
const OFF = [0, 15, 30, 60]
// Textos en screensaver.style.<id> y screensaver.style.<id>.sub
const STYLES: { id: SaverStyle; icon: typeof Mountains }[] = [
  { id: 'paisajes', icon: Mountains },
  { id: 'bajo-consumo', icon: MoonStars },
  { id: 'reloj', icon: Clock },
]

/** Ajustes guardados al momento, con cada cambio */
function useSaverSettings() {
  const [s, setS] = useState<SaverSettings | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => { loadSettings().then(setS).catch((e) => setError(String(e))) }, [])
  const update = (patch: Partial<SaverSettings>) => {
    if (!s) return
    const next = { ...s, ...patch }
    setS(next)
    setError(null)
    saveSettings(next).catch((e) => setError(String(e)))
  }
  return { s, update, error, setError }
}

function Toggle({ id, on, onChange, isDefault }: { id: string; on: boolean; onChange: (v: boolean) => void; isDefault?: boolean }) {
  const t = useT()
  return (
    <button className={`option sv-toggle ${on ? 'is-on' : ''}`} data-nav={id} data-nav-default={isDefault ? '' : undefined} onClick={() => onChange(!on)} aria-pressed={on}>
      {on ? <><CheckCircle size={18} weight="fill" /> {t('screensaver.on')}</> : t('screensaver.off')}
    </button>
  )
}

function Chips<T extends string | number>({ id, options, value, label, onPick }: {
  id: string; options: T[]; value: (o: T) => boolean; label: (o: T) => string; onPick: (o: T) => void
}) {
  return (
    <div className="sv-chips" role="group">
      {options.map((o) => (
        <button key={String(o)} className={`sv-chip ${value(o) ? 'is-on' : ''}`} data-nav={`${id}-${o}`} aria-pressed={value(o)} onClick={() => onPick(o)}>
          {value(o) && <CheckCircle size={16} weight="fill" />}
          {label(o)}
        </button>
      ))}
    </div>
  )
}

// ---------------- /ajustes/salvapantallas ----------------

export function ScreensaverSettings() {
  const navigate = useNavigate()
  const t = useT()
  const { s, update, error, setError } = useSaverSettings()
  const [approx, setApprox] = useState<string | null>(null)
  const [locating, setLocating] = useState(false)

  const pickApprox = async () => {
    update({ locationMode: 'auto' })
    setLocating(true)
    try {
      const p = await locateApprox()
      setApprox(p.detail ? `${p.name}, ${p.detail}` : p.name)
    } catch {
      setError(t('screensaver.locateError'))
    } finally { setLocating(false) }
  }

  if (!s) return <Page title={t('screensaver.title')}><p className="hero-meta">{error ?? t('screensaver.loading')}</p></Page>

  const toggleCollection = (c: CollectionId) => {
    const has = s.collections.includes(c)
    if (has && s.collections.length === 1) return // al menos una
    update({ collections: has ? s.collections.filter((x) => x !== c) : [...s.collections, c] })
  }

  return (
    <Page title={t('screensaver.title')}>
      <div className="row">
        <span className="row-label">
          {t('screensaver.title')}
          <span className="row-sub">{t('screensaver.enabled.sub')}</span>
        </span>
        <Toggle id="sv-on" on={s.enabled} onChange={(enabled) => update({ enabled })} isDefault />
      </div>

      <h3 className="label sv-label">{t('screensaver.after')}</h3>
      <Chips id="sv-after" options={AFTER} value={(m) => s.afterMin === m} label={(m) => t('screensaver.minutes', { m })} onPick={(afterMin) => update({ afterMin })} />

      <h3 className="label sv-label">{t('screensaver.style')}</h3>
      {STYLES.map(({ id, icon: Icon }) => (
        <button key={id} className="option sv-style" data-nav={`sv-style-${id}`} aria-pressed={s.style === id} onClick={() => update({ style: id })}>
          <span className="sv-style-main">
            <Icon size={24} weight="light" />
            <span className="row-label">{t(`screensaver.style.${id}`)}<span className="row-sub">{t(`screensaver.style.${id}.sub`)}</span></span>
          </span>
          {s.style === id && <span className="option-check"><CheckCircle size={18} weight="fill" /> {t('screensaver.chosen')}</span>}
        </button>
      ))}

      {s.style === 'paisajes' && (
        <>
          <h3 className="label sv-label">{t('screensaver.landscapes')}</h3>
          <Chips id="sv-col" options={collections.map((c) => c.id)} value={(c) => s.collections.includes(c)}
            label={(c) => collections.find((x) => x.id === c)?.label ?? c} onPick={toggleCollection} />
        </>
      )}

      <h3 className="label sv-label">{t('screensaver.weather')}</h3>
      <div className="row">
        <span className="row-label">{t('screensaver.weather.show')}<span className="row-sub">{t('screensaver.weather.show.sub')}</span></span>
        <Toggle id="sv-weather" on={s.showWeather} onChange={(showWeather) => update({ showWeather })} />
      </div>
      {s.showWeather && (
        <>
          <button className="option sv-style" data-nav="sv-loc-auto" aria-pressed={s.locationMode === 'auto'} onClick={pickApprox}>
            <span className="sv-style-main">
              <NavigationArrow size={22} weight="light" />
              <span className="row-label">
                {t('screensaver.loc.auto')}
                <span className="row-sub">{locating ? t('screensaver.loc.calculating') : s.locationMode === 'auto' && approx ? approx : t('screensaver.loc.autoSub')}</span>
              </span>
            </span>
            {s.locationMode === 'auto' && <span className="option-check"><CheckCircle size={18} weight="fill" /> {t('screensaver.chosen')}</span>}
          </button>
          <button className="option sv-style" data-nav="sv-loc-city" aria-pressed={s.locationMode === 'ciudad'}
            onClick={() => { update({ locationMode: 'ciudad' }); navigate('/ajustes/salvapantallas/ciudad') }}>
            <span className="sv-style-main">
              <MapPin size={22} weight="light" />
              <span className="row-label">
                {t('screensaver.loc.city')}
                <span className="row-sub">{s.place ? `${s.place.name}${s.place.detail ? `, ${s.place.detail}` : ''}` : t('screensaver.loc.none')}</span>
              </span>
            </span>
            {s.locationMode === 'ciudad' && s.place ? <span className="option-check"><CheckCircle size={18} weight="fill" /> {t('screensaver.chosen')}</span> : <CaretRight size={18} weight="light" />}
          </button>
        </>
      )}

      <h3 className="label sv-label">{t('screensaver.games')}</h3>
      <div className="row">
        <span className="row-label">{t('screensaver.skip')}<span className="row-sub">{t('screensaver.skip.sub')}</span></span>
        <Toggle id="sv-skip" on={s.skipFullscreen} onChange={(skipFullscreen) => update({ skipFullscreen })} />
      </div>
      <div className="sv-sub">
        <span className="row-label">{t('screensaver.monitorOff')}</span>
        <Chips id="sv-off" options={OFF} value={(m) => s.monitorOffMin === m} label={(m) => (m === 0 ? t('screensaver.never') : t('screensaver.minutes', { m }))} onPick={(monitorOffMin) => update({ monitorOffMin })} />
      </div>

      <SaverExtras s={s} update={update} />

      <div className="sv-actions">
        <button className="pill pill-dark" data-nav="sv-test" onClick={() => previewSaver().catch((e) => setError(String(e)))}>
          {t('screensaver.test')}
          <span className="pill-icon"><Play size={14} weight="fill" /></span>
        </button>
      </div>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

// ---------------- /ajustes/salvapantallas/ciudad ----------------

export function ScreensaverCity() {
  const navigate = useNavigate()
  const t = useT()
  const { s, update, error, setError } = useSaverSettings()
  const [q, setQ] = useState('')
  const [results, setResults] = useState<Place[] | null>(null)
  const [busy, setBusy] = useState(false)
  const timer = useRef<number>(undefined)

  const search = async (text: string) => {
    if (text.trim().length < 2) { setResults(null); return }
    setBusy(true); setError(null)
    try { setResults(await searchPlaces(text)) } catch (e) { setError(String(e)); setResults(null) } finally { setBusy(false) }
  }
  const onType = (text: string) => {
    setQ(text)
    clearTimeout(timer.current)
    timer.current = window.setTimeout(() => search(text), 450)
  }
  const onSubmit = (e: FormEvent) => { e.preventDefault(); clearTimeout(timer.current); search(q) }
  const choose = (place: Place) => { update({ locationMode: 'ciudad', place }); navigate(-1) }

  return (
    <Page title={t('screensaver.loc.city')}>
      <form className="sv-search" onSubmit={onSubmit}>
        <MagnifyingGlass size={22} weight="light" />
        <input
          className="sv-input"
          data-nav="city-q"
          data-nav-default=""
          value={q}
          onChange={(e) => onType(e.target.value)}
          placeholder={t('screensaver.city.placeholder')}
          aria-label={t('screensaver.city.label')}
          autoComplete="off"
          spellCheck={false}
        />
        <button className="pill pill-dark" type="submit" data-nav="city-go">{t('screensaver.city.search')}</button>
      </form>
      {s?.place && <p className="hero-meta sv-now">{t('screensaver.city.now', { place: `${s.place.name}${s.place.detail ? `, ${s.place.detail}` : ''}` })}</p>}

      {busy && <p className="hero-meta sv-now">{t('screensaver.city.searching')}</p>}
      {!busy && results?.length === 0 && (
        <p className="notice info">{t('screensaver.city.none')}</p>
      )}
      {results?.map((p, i) => (
        <button key={`${p.latitude},${p.longitude}`} className="option" data-nav={`city-${i}`} onClick={() => choose(p)}>
          <span className="sv-style-main">
            <MapPin size={22} weight="light" />
            <span className="row-label">{p.name}<span className="row-sub">{p.detail}</span></span>
          </span>
          <CaretRight size={18} weight="light" />
        </button>
      ))}
      {error && <p className="notice error">{error.includes('conexión') || !navigator.onLine ? t('screensaver.city.offline') : error}</p>}
    </Page>
  )
}
