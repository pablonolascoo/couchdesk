import { useEffect, useState } from 'react'
import { ArrowUpRight, ArrowCounterClockwise, Desktop as DesktopIcon } from '@phosphor-icons/react'
import { TopBar } from '../components/TopBar'
import { HintBar } from '../components/HintBar'
import { call, exitToDesktop, isDesktopApp } from '../native'
import './Desktop.css'

export function Desktop() {
  const [consoleBoots, setConsoleBoots] = useState<boolean | null>(null)
  const [confirm, setConfirm] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    call<{ enabled: boolean }>('shell_status', {}, { enabled: true }).then((s) => setConsoleBoots(s.enabled)).catch(() => setConsoleBoots(false))
  }, [])

  const goNormal = () => {
    if (consoleBoots && !confirm) return setConfirm(true) // pulsar dos veces para confirmar
    exitToDesktop(true).catch((e) => setError(String(e)))
  }

  return (
    <div className="screen">
      <TopBar title="Escritorio" />
      <section className="desk">
        <span className="eyebrow reveal">Windows</span>
        <h2 className="hero-title reveal" style={{ ['--i' as string]: 1 }}>¿Cómo quieres volver?</h2>
        {!isDesktopApp && <p className="hero-meta">Esto solo funciona en la consola instalada.</p>}

        <div className="desk-options">
          <button className="desk-card reveal" style={{ ['--i' as string]: 2 }} data-nav="now" data-nav-default="" onClick={() => exitToDesktop().catch((e) => setError(String(e)))} disabled={!isDesktopApp}>
            <span className="desk-icon"><DesktopIcon size={28} /></span>
            <span className="desk-title">Ir al escritorio solo ahora</span>
            <span className="desk-text">
              La consola se cierra y ves el escritorio de Windows.
              {consoleBoots ? ' Al volver a encender, aparece la consola otra vez.' : ''}
            </span>
            <span className="desk-go"><ArrowUpRight size={18} /></span>
          </button>

          <button className={`desk-card${confirm ? ' desk-warn' : ''} reveal`} style={{ ['--i' as string]: 3 }} data-nav="normal" onClick={goNormal} disabled={!isDesktopApp}>
            <span className="desk-icon"><ArrowCounterClockwise size={28} /></span>
            <span className="desk-title">Volver al modo normal de Windows</span>
            <span className="desk-text">
              {confirm
                ? 'Pulsa otra vez para confirmar.'
                : consoleBoots === false
                  ? 'Windows ya arranca en su escritorio de siempre. Solo se cerrará la consola.'
                  : 'Al encender, Windows abrirá su escritorio normal. La consola seguirá en el menú Inicio.'}
            </span>
            <span className="desk-go"><ArrowUpRight size={18} /></span>
          </button>
        </div>

        <p className="desk-foot reveal" style={{ ['--i' as string]: 4 }}>
          Para volver a usar la consola al encender: ábrela desde el menú Inicio → Ajustes → Arranque.
        </p>
        {error && <p className="notice error">{error}</p>}
      </section>
      <HintBar hints={[['A', 'Seleccionar'], ['B', 'Atrás']]} />
    </div>
  )
}
