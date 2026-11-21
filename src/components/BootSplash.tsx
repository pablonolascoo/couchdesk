import { useEffect, useState } from 'react'
import { sound } from '../ui/sound'
import './BootSplash.css'

// Arranque de CouchDesk (solo la primera vez que se abre la ventana):
// la C se dibuja, la luz se enciende y respira, sube el nombre y todo se disuelve en el Inicio.
// Cualquier tecla, clic o botón del mando lo salta.

let shown = false
/** La marca no se traduce */
const WORD = 'CouchDesk'
const HOLD_MS = 2050
const OUT_MS = 720

/** La pantalla de debajo entra al disolverse el arranque (ver index.css: data-booting) */
function handoff() {
  const root = document.documentElement
  if (!('booting' in root.dataset)) return
  delete root.dataset.booting
  const route = document.querySelector<HTMLElement>('.route')
  route?.animate(
    [
      { opacity: 0, transform: 'scale(1.02)', filter: 'blur(8px)' },
      { opacity: 1, transform: 'none', filter: 'blur(0px)' },
    ],
    { duration: 1100, easing: 'cubic-bezier(0.16, 1, 0.3, 1)' },
  )
}

export function BootSplash() {
  const [phase, setPhase] = useState<'in' | 'out' | 'done'>(shown ? 'done' : 'in')

  useEffect(() => {
    if (phase === 'done') { handoff(); return }
    if (!shown) sound.boot()
    shown = true
    let doneTimer = 0
    const leave = () => {
      setPhase((p) => (p === 'in' ? 'out' : p))
      handoff()
      clearTimeout(doneTimer)
      doneTimer = window.setTimeout(() => setPhase('done'), OUT_MS)
    }
    const outTimer = window.setTimeout(leave, HOLD_MS)
    const skip = () => { clearTimeout(outTimer); leave() }
    const opts = { once: true, capture: true } as const
    window.addEventListener('keydown', skip, opts)
    window.addEventListener('pointerdown', skip, opts)
    window.addEventListener('padinput', skip, opts)
    return () => {
      clearTimeout(outTimer)
      clearTimeout(doneTimer)
      window.removeEventListener('keydown', skip, opts)
      window.removeEventListener('pointerdown', skip, opts)
      window.removeEventListener('padinput', skip, opts)
    }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  if (phase === 'done') return null
  return (
    <div className={`boot ${phase === 'out' ? 'boot-out' : ''}`} aria-hidden>
      <svg className="boot-mark" viewBox="0 0 256 256" width="112" height="112">
        {/* La C se dibuja siguiendo su línea central y luego se cambia por la forma rellena */}
        <path className="boot-c-draw" d="M193.68 74.05A85 85 0 1 0 193.68 181.95" pathLength={1} />
        <path className="boot-c-fill" d="M205.95 63.97A104 104 0 1 0 205.95 192.03A19 19 0 0 0 176.01 168.63A66 66 0 1 1 176.01 87.37A19 19 0 0 0 205.95 63.97Z" />
        {/* La luz: un brillo que se abre y el punto que se enciende y respira */}
        <circle className="boot-bloom" cx="211" cy="128" r="20" />
        <circle className="boot-dot" cx="211" cy="128" r="20" />
      </svg>
      <h1 className="boot-word" aria-label={WORD}>
        {WORD.split('').map((c, i) => (
          <span key={i} className="boot-char-mask">
            <span className="boot-char" style={{ animationDelay: `${880 + i * 50}ms` }}>{c}</span>
          </span>
        ))}
      </h1>
    </div>
  )
}
