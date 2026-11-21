import { useState, type CSSProperties } from 'react'
import { ArrowUpRight, Cloud as CloudIcon, GameController } from '@phosphor-icons/react'
import { BrandLogo } from '../ui/brands'
import './Cloud.css'
import { TopBar } from '../components/TopBar'
import { HintBar } from '../components/HintBar'
import { call, isDesktopApp } from '../native'
import './Desktop.css'

// Jugar sin instalar: Xbox Cloud Gaming y tu PS5 por la red de casa
export function Cloud() {
  const [busy, setBusy] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const run = (id: string, cmd: string, msg: string) => {
    setError(null)
    setBusy(id)
    call(cmd, {}).catch((e) => setError(`${msg}: ${e}`)).finally(() => setBusy(null))
  }
  const i = (n: number) => ({ '--i': n }) as CSSProperties

  return (
    <div className="screen">
      <TopBar title="En la nube" />
      <section className="desk">
        <span className="eyebrow reveal">Jugar sin instalar</span>
        <h2 className="hero-title reveal" style={i(1)}>¿A qué juegas hoy?</h2>
        {!isDesktopApp && <p className="hero-meta">Esto solo funciona en la consola instalada.</p>}

        <div className="desk-options">
          <button className="desk-card cloud-card xbox reveal" style={i(2)} data-nav="xcloud" data-nav-default="" disabled={!isDesktopApp} onClick={() => run('xcloud', 'cloud_open', 'No se pudo abrir')}>
            <span className="cloud-art" aria-hidden><GameController size={240} weight="fill" /></span>
            <span className="desk-icon"><CloudIcon size={28} weight="fill" /></span>
            <span className="desk-title">Xbox Cloud Gaming</span>
            <span className="desk-text">
              {busy === 'xcloud' ? 'Abriendo…' : 'Juegos de Game Pass al momento. La primera vez inicia sesión con tu cuenta de Microsoft.'}
            </span>
            <span className="desk-go"><ArrowUpRight size={18} /></span>
          </button>

          <button className="desk-card cloud-card ps reveal" style={i(3)} data-nav="ps5" disabled={!isDesktopApp || busy === 'ps5'} onClick={() => run('ps5', 'open_remote_play', 'No se pudo abrir PS5 Remote Play')}>
            <span className="cloud-art" aria-hidden><BrandLogo name="playstation" size={230} color="currentColor" /></span>
            <span className="desk-icon"><BrandLogo name="playstation" size={26} color="currentColor" /></span>
            <span className="desk-title">PS5 Remote Play</span>
            <span className="desk-text">
              {busy === 'ps5' ? 'Preparándolo… (la primera vez se descarga)' : 'Juega a tu PS5 desde aquí. La primera vez vincúlala con tu cuenta de PlayStation.'}
            </span>
            <span className="desk-go"><ArrowUpRight size={18} /></span>
          </button>
        </div>

        <p className="desk-foot reveal" style={i(4)}>
          Para volver a la consola desde un juego en la nube: mantén LB + RB + View (o L1 + R1 + SHARE), o pulsa el botón PS.
        </p>
        {error && <p className="notice error">{error}</p>}
      </section>
      <HintBar hints={[['A', 'Jugar'], ['B', 'Atrás']]} />
    </div>
  )
}
