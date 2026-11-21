import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import {
  SpeakerHigh, SpeakerSlash, SunDim, BatteryHigh, WifiHigh, CheckCircle, CaretRight,
  Power, ArrowClockwise, Moon, WifiSlash, Rocket,
} from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { Stepper } from '../components/Stepper'
import { APP_VERSION } from '../version'
import { call } from '../native'

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
  const navigate = useNavigate()
  const { data: vol } = useNative('get_volume', mocks.volume)
  const { data: bat } = useNative('get_battery', mocks.battery)
  const { data: wifi } = useNative('get_wifi', mocks.wifi)
  const items = [
    { path: '/ajustes/sonido', label: 'Sonido', icon: SpeakerHigh, value: vol ? (vol.muted ? 'Silencio' : `${vol.level}%`) : '' },
    { path: '/ajustes/pantalla', label: 'Pantalla', icon: SunDim, value: 'Brillo' },
    { path: '/ajustes/energia', label: 'Energía', icon: BatteryHigh, value: bat?.percent != null ? `${bat.percent}%${bat.pluggedIn ? ' · cargando' : ''}` : '' },
    { path: '/ajustes/red', label: 'Wi-Fi', icon: WifiHigh, value: wifi ? (wifi.connected ? wifi.ssid ?? '' : 'Desconectado') : '' },
    { path: '/ajustes/arranque', label: 'Arranque', icon: Rocket, value: '' },
  ]
  return (
    <Page title="Ajustes">
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
      <p className="app-version">Consola · {APP_VERSION}</p>
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
  return (
    <Page title="Sonido">
      <Stepper id="vol" label="Volumen" value={data?.level ?? null} onChange={setLevel} isDefault />
      <div className="row">
        <span className="row-label">Silenciar</span>
        <button className="option" style={{ width: 'auto', marginTop: 0 }} data-nav="mute" onClick={toggleMute}>
          {data?.muted ? <SpeakerSlash size={20} weight="light" /> : <SpeakerHigh size={20} weight="light" />}
          {data?.muted ? 'Activar sonido' : 'Silenciar'}
        </button>
      </div>
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
  return (
    <Page title="Pantalla">
      {data === null && !error ? (
        <p className="hero-meta">Leyendo el brillo…</p>
      ) : (
        <Stepper id="bri" label="Brillo" value={data} onChange={setLevel} isDefault />
      )}
      {error && <p className="notice error">Esta pantalla no permite cambiar el brillo desde aquí.</p>}
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

  return (
    <Page title="Energía">
      <div className="row">
        <span className="row-label">Batería</span>
        <span className="row-value">
          {bat?.hasBattery ? `${bat.percent ?? '—'}% · ${bat.pluggedIn ? 'enchufado' : 'con batería'}` : 'Sin batería'}
        </span>
      </div>

      <h3 className="label" style={{ marginTop: 28 }}>Modo de energía</h3>
      {plans?.map((p, i) => (
        <button key={p.guid} className="option" data-nav={`plan-${p.guid}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => choose(p.guid)}>
          {p.name}
          {p.active && <span className="option-check"><CheckCircle size={18} weight="fill" /> Activo</span>}
        </button>
      ))}

      <h3 className="label" style={{ marginTop: 28 }}>Apagar</h3>
      {([['suspender', 'Suspender', Moon], ['reiniciar', 'Reiniciar', ArrowClockwise], ['apagar', 'Apagar', Power]] as const).map(([a, label, Icon]) => (
        <button key={a} className="option" data-nav={`pw-${a}`} onClick={() => act(a)}>
          <span style={{ display: 'inline-flex', gap: 12, alignItems: 'center' }}><Icon size={20} weight="light" /> {label}</span>
          {confirm === a && <span className="option-check" style={{ color: 'var(--red)' }}>Pulsa A otra vez para confirmar</span>}
        </button>
      ))}
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

// ---------------- Arranque ----------------

interface ShellStatus { enabled: boolean; exe: string; runningAsShell: boolean }

export function StartupSettings() {
  const { data, setData, error, setError } = useNative<ShellStatus>('shell_status', { enabled: false, exe: 'C:\\Users\\CASA\\AppData\\Local\\Consola\\consola.exe', runningAsShell: false })
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

  return (
    <Page title="Arranque">
      <div className="row">
        <span className="row-label">
          Arrancar directamente en la consola
          <span className="row-sub">Al encender el PC verás la consola en lugar del escritorio de Windows.</span>
        </span>
        <button className="option" style={{ width: 'auto', marginTop: 0 }} data-nav="toggle" data-nav-default="" onClick={toggle} disabled={devBuild && !data?.enabled}>
          {data?.enabled ? <><CheckCircle size={18} weight="fill" /> Activado</> : confirm ? 'Pulsa A otra vez para activar' : 'Desactivado'}
        </button>
      </div>
      {devBuild && <p className="notice error">Instala la consola con su instalador antes de activar esto (ahora se está usando una versión de pruebas).</p>}
      <p className="notice info">
        Cómo volver al escritorio normal: Escritorio → Ir al escritorio (solo por hoy), o desactiva esta opción.
        Si algo fallara, pulsa Ctrl + Alt + Supr → Administrador de tareas → Ejecutar nueva tarea → escribe <span className="mono">explorer</span>.
      </p>
      <p className="hero-meta" style={{ fontSize: 13, marginTop: 12 }}>
        Solo afecta a tu usuario. Los programas de inicio (RustDesk, WARP, audio…) los arranca la consola. Programa: <span className="mono">{data?.exe}</span>
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

  if (data && !data.hasAdapter) return <Page title="Wi-Fi"><p className="hero-meta">Este PC no tiene Wi-Fi.</p></Page>

  return (
    <Page title="Wi-Fi">
      <div className="row">
        <span className="row-label">
          {data?.connected ? data.ssid : 'Sin conexión Wi-Fi'}
          <span className="row-sub">{data?.connected ? `Señal ${data.signal ?? '—'}%` : 'Puedes seguir conectado por cable'}</span>
        </span>
        {data?.connected && (
          <button className="option" style={{ width: 'auto', marginTop: 0 }} data-nav="wifi-off" onClick={disconnect}>
            <WifiSlash size={20} weight="light" /> Desconectar
          </button>
        )}
      </div>

      <h3 className="label" style={{ marginTop: 28 }}>Redes guardadas</h3>
      {data?.saved.length === 0 && <p className="hero-meta">No hay redes guardadas en este PC.</p>}
      {data?.saved.map((name, i) => (
        <button key={name} className="option" data-nav={`net-${name}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => connect(name)}>
          <span style={{ display: 'inline-flex', gap: 12, alignItems: 'center' }}><WifiHigh size={20} weight="light" /> {name}</span>
          {data.ssid === name ? <span className="option-check"><CheckCircle size={18} weight="fill" /> Conectado</span> : busy === name ? 'Conectando…' : null}
        </button>
      ))}
      <p className="notice info">Para añadir una red nueva, de momento usa el escritorio de Windows una vez. Luego saldrá aquí.</p>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}
