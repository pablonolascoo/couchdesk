import { useEffect, useState } from 'react'
import { GameController, Mouse, Keyboard } from '@phosphor-icons/react'
import { padKind } from '../ui/sound'
import { useT } from '../i18n'
import './PadModeToast.css'

const modes = [
  { icon: GameController, text: 'shell.padMode.nav' },
  { icon: Mouse, text: 'shell.padMode.mouse' },
  { icon: Keyboard, text: 'shell.padMode.keyboard' },
]

// Aviso arriba al cambiar de modo con xTend; en ratón/teclado se queda visible
export function PadModeToast() {
  const t = useT()
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
      <Icon size={18} weight="light" /> {t(text)}{mode !== 0 && ` ${t('shell.padMode.switch', { keys })}`}
    </div>
  )
}
