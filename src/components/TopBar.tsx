import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { WifiHigh, WifiSlash, BatteryHigh, BatteryMedium, BatteryLow, BatteryCharging } from '@phosphor-icons/react'
import { Qr } from './Qr'
import { useRemote } from '../data/remote'
import { call } from '../native'
import './TopBar.css'

interface Battery { hasBattery: boolean; percent: number | null; pluggedIn: boolean }

export function TopBar({ title }: { title: string }) {
  const [now, setNow] = useState(() => new Date())
  const [battery, setBattery] = useState<Battery | null>(null)
  const [online, setOnline] = useState(navigator.onLine)
  const navigate = useNavigate()
  const remote = useRemote()

  useEffect(() => {
    const tick = () => {
      setNow(new Date())
      call<Battery>('get_battery', {}, { hasBattery: true, percent: 84, pluggedIn: false }).then(setBattery).catch(() => {})
    }
    tick()
    const id = setInterval(tick, 30_000)
    const on = () => setOnline(navigator.onLine)
    window.addEventListener('online', on)
    window.addEventListener('offline', on)
    return () => { clearInterval(id); window.removeEventListener('online', on); window.removeEventListener('offline', on) }
  }, [])

  const time = now.toLocaleTimeString('es-ES', { hour: '2-digit', minute: '2-digit' })
  const pct = battery?.percent ?? 0
  const BatIcon = battery?.pluggedIn ? BatteryCharging : pct > 60 ? BatteryHigh : pct > 25 ? BatteryMedium : BatteryLow

  return (
    <header className="topbar">
      <div className="topbar-brand">
        <span className="brand-mark" aria-hidden><span /></span>
        <h1 className="topbar-title"><span key={title}>{title}</span></h1>
      </div>
      <div className="topbar-right">
        <div className="topbar-status">
          {online ? <WifiHigh size={18} aria-label="Conectado a internet" /> : <WifiSlash size={18} aria-label="Sin internet" />}
          {battery?.hasBattery && (
            <span className="battery"><BatIcon size={20} aria-hidden /> {battery.percent ?? '—'}%</span>
          )}
          <span className="status-sep" aria-hidden />
          <span className="clock">{time}</span>
        </div>
        <button className="qr" data-nav="qr" onClick={() => navigate('/mando-movil')} aria-label="Conectar el móvil como mando">
          {remote?.url ? <Qr text={remote.url} size={40} /> : null}
          <span>Mando móvil<small>{remote?.clients.length ? `${remote.clients.length} conectado${remote.clients.length > 1 ? 's' : ''}` : 'Escanea'}</small></span>
        </button>
      </div>
    </header>
  )
}
