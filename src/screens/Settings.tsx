import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import {
  SpeakerHigh, SpeakerSlash, SunDim, BatteryHigh, WifiHigh, CheckCircle, CaretRight,
  Power, ArrowClockwise, Moon, WifiSlash, Rocket, ImageSquare,
} from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { Stepper } from '../components/Stepper'
import { APP_VERSION } from '../version'
import { call } from '../native'
import { GameController, MoonStars, Globe } from '@phosphor-icons/react'
import { useT, LANGUAGES, getLanguage, setLanguage, pickLanguage, usesSystemLanguage } from '../i18n'
import { usePadsLabel } from './Controllers'
import { Palette, Gauge } from '@phosphor-icons/react'
import { t as tr } from '../i18n'
import { themeLabel, MenuMusicLink } from './Themes'
import { WindowsChecklist } from './WindowsChecklist'

/** Carga un valor nativo y permite recargarlo */
function useNative<T>(cmd: string, mock: T) {
  const [data, setData] = useState<T | null>(null)
  const [error, setError] = useState<string | null>(null)
  const reload = useCallback(() => {
    call<T>(cmd, {}, mock).then(setData).catch((e) => setError(String(e)))
  }, [cmd]) // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(reload, [reload])
  return { data, setData, error, setError, reload }
}

interface Volume { level: number; muted: boolean }
interface Battery { hasBattery: boolean; percent: number | null; pluggedIn: boolean }
interface PowerPlan { guid: string; name: string; active: boolean }
interface Wifi { hasAdapter: boolean; connected: boolean; ssid: string | null; signal: number | null; saved: string[] }

const mocks = {
  volume: { level: 60, muted: false } as Volume,
  brightness: 80 as number | null,
  battery: { hasBattery: true, percent: 58, pluggedIn: true } as Battery,
  plans: [
    { guid: '381b4222-f694-41f0-9685-ff5bb260df2e', name: 'Equilibrado', active: false },
    { guid: 'e1477891-417f-46d8-99a9-3ae9d087202f', name: 'Juego - Maximo rendimiento', active: true },
  ] as PowerPlan[],
  wifi: { hasAdapter: true, connected: false, ssid: null, signal: null, saved: ['DIGIFIBRA-URxC'] } as Wifi,
}

// ---------------- Menú ----------------

export function SettingsMenu() {
  const t = useT()
  const navigate = useNavigate()
  const { data: vol } = useNative('get_volume', mocks.volume)
  const { data: bat } = useNative('get_battery', mocks.battery)
  const { data: wifi } = useNative('get_wifi', mocks.wifi)
  const items = [
    { path: '/ajustes/sonido', label: t('settings.sound'), icon: SpeakerHigh, value: vol ? (vol.muted ? t('settings.sound.muted') : `${vol.level}%`) : '' },
    { path: '/ajustes/pantalla', label: t('settings.display'), icon: SunDim, value: t('settings.display.brightness') },
    { path: '/ajustes/energia', label: t('settings.power'), icon: BatteryHigh, value: bat?.percent != null ? (bat.pluggedIn ? t('settings.power.charging', { percent: bat.percent }) : `${bat.percent}%`) : '' },
    { path: '/ajustes/mandos', label: t('controllers.title'), icon: GameController, value: usePadsLabel() }, // hook a nivel de componente (orden fijo)
    { path: '/ajustes/red', label: t('settings.wifi'), icon: WifiHigh, value: wifi ? (wifi.connected ? wifi.ssid ?? '' : t('settings.wifi.off')) : '' },
    { path: '/ajustes/idioma', label: t('settings.language'), icon: Globe, value: LANGUAGES.find((l) => l.code === getLanguage())?.name ?? '' },
    { path: '/ajustes/arranque', label: t('settings.startup'), icon: Rocket, value: '' },
    { path: '/ajustes/temas', label: tr('themes.title'), icon: Palette, value: themeLabel() },
    { path: '/ajustes/fondo', label: t('settings.background'), icon: ImageSquare, value: '' },
    { path: '/ajustes/salvapantallas', label: t('settings.screensaver'), icon: MoonStars, value: '' },
    { path: '/ajustes/rendimiento', label: t('performance.title'), icon: Gauge, value: '' },
  ]
  return (
    <Page title={t('settings.title')}>
      <div className="menu-list">
        {items.map(({ path, label, icon: Icon, value }, i) => (
          <button key={path} className="menu-item" data-nav={`set-${path}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => navigate(path)}>
            <Icon size={24} weight="light" />
            <span className="row-label">{label}</span>
            <span className="menu-item-value">{value}</span>
            <CaretRight size={18} weight="light" />
          </button>
        ))}
      </div>
      <p className="app-version">CouchDesk · {APP_VERSION}</p>
    </Page>
  )
}

// ---------------- Sonido ----------------

export function SoundSettings() {
  const { data, setData, error, setError } = useNative('get_volume', mocks.volume)
  const setLevel = (level: number) => {
    setData((d) => (d ? { ...d, level } : d))
    call('set_volume', { level }).catch((e) => setError(String(e)))
  }
  const toggleMute = () => {
    const muted = !data?.muted
    setData((d) => (d ? { ...d, muted } : d))
    call('set_mute', { muted }).catch((e) => setError(String(e)))
  }
  const t = useT()
  return (
    <Page title={t('settings.sound')}>
      <Stepper id="vol" label={t('settings.sound.volume')} value={data?.level ?? null} onChange={setLevel} isDefault />
      <div className="row">
        <span className="row-label">{t('settings.sound.mute')}</span>
        <button className="option" style={{ width: 'auto', marginTop: 0 }} data-nav="mute" onClick={toggleMute}>
          {data?.muted ? <SpeakerSlash size={20} weight="light" /> : <SpeakerHigh size={20} weight="light" />}
          {data?.muted ? t('settings.sound.unmute') : t('settings.sound.mute')}
        </button>
      </div>
      <MenuMusicLink />
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

// ---------------- Pantalla ----------------

export function DisplaySettings() {
  const { data, setData, error, setError } = useNative('get_brightness', mocks.brightness)
  const setLevel = (level: number) => {
    setData(level)
    call('set_brightness', { level }).catch((e) => setError(String(e)))
  }
  const t = useT()
  return (
    <Page title={t('settings.display')}>
      {data === null && !error ? (
        <p className="hero-meta">{t('settings.display.reading')}</p>
      ) : (
        <Stepper id="bri" label={t('settings.display.brightness')} value={data} onChange={setLevel} isDefault />
      )}
      {error && <p className="notice error">{t('settings.display.unsupported')}</p>}
    </Page>
  )
}

// ---------------- Energía ----------------

export function PowerSettings() {
  const { data: bat } = useNative('get_battery', mocks.battery)
  const { data: plans, reload, error, setError } = useNative('get_power_plans', mocks.plans)
  const [confirm, setConfirm] = useState<string | null>(null)

  const choose = (guid: string) => call('set_power_plan', { guid }).then(reload).catch((e) => setError(String(e)))
  const act = (action: string) => {
    if (confirm !== action) return setConfirm(action) // pulsar dos veces = confirmar
    call('power_action', { action }).catch((e) => setError(String(e)))
  }

  const t = useT()
  return (
    <Page title={t('settings.power')}>
      <div className="row">
        <span className="row-label">{t('settings.power.battery')}</span>
        <span className="row-value">
          {bat?.hasBattery ? t(bat.pluggedIn ? 'settings.power.pluggedIn' : 'settings.power.onBattery', { percent: bat.percent ?? '—' }) : t('settings.power.noBattery')}
        </span>
      </div>

      <h3 className="label" style={{ marginTop: 28 }}>{t('settings.power.mode')}</h3>
      {plans?.map((p, i) => (
        <button key={p.guid} className="option" data-nav={`plan-${p.guid}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => choose(p.guid)}>
          {p.name}
          {p.active && <span className="option-check"><CheckCircle size={18} weight="fill" /> {t('settings.power.active')}</span>}
        </button>
      ))}

      <h3 className="label" style={{ marginTop: 28 }}>{t('settings.power.off')}</h3>
      {([['suspender', t('settings.power.sleep'), Moon], ['reiniciar', t('settings.power.restart'), ArrowClockwise], ['apagar', t('settings.power.shutdown'), Power]] as const).map(([a, label, Icon]) => (
        <button key={a} className="option" data-nav={`pw-${a}`} onClick={() => act(a)}>
          <span style={{ display: 'inline-flex', gap: 12, alignItems: 'center' }}><Icon size={20} weight="light" /> {label}</span>
          {confirm === a && <span className="option-check" style={{ color: 'var(--red)' }}>{t('settings.power.confirm')}</span>}
        </button>
      ))}
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

// ---------------- Arranque ----------------

interface ShellStatus { enabled: boolean; exe: string; runningAsShell: boolean }

export function StartupSettings() {
  const { data, setData, error, setError } = useNative<ShellStatus>('shell_status', { enabled: false, exe: 'C:\\Users\\CASA\\AppData\\Local\\CouchDesk\\couchdesk.exe', runningAsShell: false })
  const [confirm, setConfirm] = useState(false)
  const devBuild = data?.exe.toLowerCase().includes('\\target\\') ?? false

  const toggle = async () => {
    if (!data) return
    if (!data.enabled && !confirm) return setConfirm(true) // pulsar dos veces para activar
    try {
      setData(await call<ShellStatus>('shell_set', { enabled: !data.enabled }, { ...data, enabled: !data.enabled }))
      setConfirm(false)
    } catch (e) { setError(String(e)) }
  }

  const t = useT()
  return (
    <Page title={t('settings.startup')}>
      <div className="row">
        <span className="row-label">
          {t('settings.startup.toggle')}
          <span className="row-sub">{t('settings.startup.toggleSub')}</span>
        </span>
        <button className="option" style={{ width: 'auto', marginTop: 0 }} data-nav="toggle" data-nav-default="" onClick={toggle} disabled={devBuild && !data?.enabled}>
          {data?.enabled ? <><CheckCircle size={18} weight="fill" /> {t('settings.startup.on')}</> : confirm ? t('settings.startup.confirm') : t('settings.startup.off')}
        </button>
      </div>
      <WindowsChecklist />
      {devBuild && <p className="notice error">{t('settings.startup.devBuild')}</p>}
      <p className="notice info">
        {t('settings.startup.howBack')}{' '}
        {t('settings.startup.rescue')} <span className="mono">explorer</span>.
      </p>
      <p className="hero-meta" style={{ fontSize: 13, marginTop: 12 }}>
        {t('settings.startup.foot')} <span className="mono">{data?.exe}</span>
      </p>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

// ---------------- Wi-Fi ----------------

export function NetworkSettings() {
  const { data, reload, error, setError } = useNative('get_wifi', mocks.wifi)
  const [busy, setBusy] = useState<string | null>(null)

  const connect = async (name: string) => {
    setBusy(name); setError(null)
    try {
      await call('connect_wifi', { name })
      setTimeout(() => { reload(); setBusy(null) }, 4000) // Windows tarda unos segundos en conectar
    } catch (e) { setError(String(e)); setBusy(null) }
  }
  const disconnect = () => call('disconnect_wifi').then(() => setTimeout(reload, 1500)).catch((e) => setError(String(e)))

  const t = useT()
  if (data && !data.hasAdapter) return <Page title={t('settings.wifi')}><p className="hero-meta">{t('settings.wifi.noAdapter')}</p></Page>

  return (
    <Page title={t('settings.wifi')}>
      <div className="row">
        <span className="row-label">
          {data?.connected ? data.ssid : t('settings.wifi.notConnected')}
          <span className="row-sub">{data?.connected ? t('settings.wifi.signal', { signal: data.signal ?? '—' }) : t('settings.wifi.cable')}</span>
        </span>
        {data?.connected && (
          <button className="option" style={{ width: 'auto', marginTop: 0 }} data-nav="wifi-off" onClick={disconnect}>
            <WifiSlash size={20} weight="light" /> {t('settings.wifi.disconnect')}
          </button>
        )}
      </div>

      <h3 className="label" style={{ marginTop: 28 }}>{t('settings.wifi.saved')}</h3>
      {data?.saved.length === 0 && <p className="hero-meta">{t('settings.wifi.noSaved')}</p>}
      {data?.saved.map((name, i) => (
        <button key={name} className="option" data-nav={`net-${name}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => connect(name)}>
          <span style={{ display: 'inline-flex', gap: 12, alignItems: 'center' }}><WifiHigh size={20} weight="light" /> {name}</span>
          {data.ssid === name ? <span className="option-check"><CheckCircle size={18} weight="fill" /> {t('settings.wifi.connected')}</span> : busy === name ? t('settings.wifi.connecting') : null}
        </button>
      ))}
      <p className="notice info">{t('settings.wifi.addHelp')}</p>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

// ---------------- Idioma ----------------

export function LanguageSettings() {
  const t = useT() // se repinta al cambiar: el cambio se ve al momento
  const system = usesSystemLanguage()
  const current = getLanguage()
  const systemCode = pickLanguage(navigator.languages ?? [navigator.language])
  const systemName = LANGUAGES.find((l) => l.code === systemCode)?.name ?? systemCode
  const options = [
    { id: 'system', label: t('settings.language.system', { name: systemName }), active: system, pick: () => setLanguage(null) },
    ...LANGUAGES.map((l) => ({ id: l.code, label: l.name, active: !system && l.code === current, pick: () => setLanguage(l.code) })),
  ]
  return (
    <Page title={t('settings.language')}>
      {options.map((o, i) => (
        <button key={o.id} className="option" lang={o.id === 'system' ? undefined : o.id} data-nav={`lang-${o.id}`} data-nav-default={i === 0 ? '' : undefined} aria-pressed={o.active} onClick={o.pick}>
          {o.label}
          {o.active && <span className="option-check"><CheckCircle size={18} weight="fill" /></span>}
        </button>
      ))}
      <p className="hero-meta" style={{ fontSize: 13, marginTop: 20 }}>{t('settings.language.help')}</p>
    </Page>
  )
}
