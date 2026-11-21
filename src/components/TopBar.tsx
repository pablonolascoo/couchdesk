import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { WifiHigh, WifiSlash, BatteryHigh, BatteryMedium, BatteryLow, BatteryCharging } from '@phosphor-icons/react'
import { Qr } from './Qr'
import { useRemote } from '../data/remote'
import { call } from '../native'
import { useT, fmt } from '../i18n'
import './TopBar.css'

interface Battery { hasBattery: boolean; percent: number | null; pluggedIn: boolean }

export function TopBar({ title }: { title: string }) {
  const t = useT()
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

  const time = fmt.time(now)
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
          {online ? <WifiHigh size={18} aria-label={t('shell.online')} /> : <WifiSlash size={18} aria-label={t('shell.offline')} />}
          {battery?.hasBattery && (
            <span className="battery"><BatIcon size={20} aria-hidden /> {battery.percent ?? '—'}%</span>
          )}
          <span className="status-sep" aria-hidden />
          <span className="clock" title={fmt.date(now)}>{time}</span>
        </div>
        <button className="qr" data-nav="qr" onClick={() => navigate('/mando-movil')} aria-label={t('shell.remote.aria')}>
          {remote?.url ? <Qr text={remote.url} size={40} /> : null}
          <span>{t('shell.remote.title')}<small>{remote?.clients.length ? t('shell.remote.connected', { n: remote.clients.length }) : t('shell.remote.scan')}</small></span>
        </button>
      </div>
    </header>
  )
}
