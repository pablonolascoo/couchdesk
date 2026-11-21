import { useEffect, useState } from 'react'
import { GameController, Mouse, Keyboard } from '@phosphor-icons/react'
import { padKind } from '../ui/sound'
import './PadModeToast.css'

const modes = [
  { icon: GameController, text: 'Mando: controla la consola' },
  { icon: Mouse, text: 'Mando como ratón' },
  { icon: Keyboard, text: 'Mando como teclado' },
]

// Aviso arriba al cambiar de modo con xTend; en ratón/teclado se queda visible
export function PadModeToast() {
  const [mode, setMode] = useState<number | null>(null)
  useEffect(() => {
    let timer = 0
    const on = (e: Event) => {
      const m = (e as CustomEvent<number>).detail
      setMode(m)
      clearTimeout(timer)
      if (m === 0) timer = window.setTimeout(() => setMode(null), 2500)
    }
    window.addEventListener('padmode', on)
    return () => window.removeEventListener('padmode', on)
  }, [])
  if (mode === null) return null
  const { icon: Icon, text } = modes[mode]
  const keys = padKind() === 'ps' ? 'OPTIONS + SHARE' : 'START + BACK'
  return (
    <div className={`pad-toast ${mode === 0 ? '' : 'sticky'}`} role="status">
      <Icon size={18} weight="light" /> {text}{mode !== 0 && ` (${keys} para cambiar)`}
    </div>
  )
}
