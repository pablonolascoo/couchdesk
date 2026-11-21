import { useEffect, useState } from 'react'
import { sound } from '../ui/sound'
import './BootSplash.css'

// Pantalla de arranque: solo la primera vez que se abre la consola
let shown = false
const WORD = 'Consola'

export function BootSplash() {
  const [phase, setPhase] = useState<'in' | 'out' | 'done'>(shown ? 'done' : 'in')

  useEffect(() => {
    if (phase === 'done') return
    if (!shown) sound.boot()
    shown = true
    const out = setTimeout(() => setPhase('out'), 2100)
    const done = setTimeout(() => setPhase('done'), 3000)
    return () => { clearTimeout(out); clearTimeout(done) }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  if (phase === 'done') return null
  return (
    <div className={`boot ${phase === 'out' ? 'boot-out' : ''}`} aria-hidden>
      <div className="boot-mark">
        <span className="boot-ring" />
        <span className="boot-ring boot-ring-2" />
        <span className="boot-dot" />
      </div>
      <h1 className="boot-word">
        {WORD.split('').map((c, i) => (
          <span key={i} className="boot-char" style={{ animationDelay: `${450 + i * 55}ms` }}>{c}</span>
        ))}
      </h1>
      <span className="boot-line"><span /></span>
    </div>
  )
}
