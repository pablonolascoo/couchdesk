import { useEffect, useState } from 'react'
import { inputMode, padKind, type InputMode } from '../ui/sound'
import { t, useT } from '../i18n'
import './HintBar.css'

// Mismo gesto, nombre distinto según lo que tengas en la mano
const keyboardName = (): Record<string, string> => ({ A: t('shell.key.enter'), B: 'Esc', X: t('shell.key.space'), Y: 'Tab', '☰': t('shell.key.home') })
const playstationName: Record<string, string> = { A: '✕', B: '○', X: '□', Y: '△', '☰': 'OPTIONS' }

// Leyenda de botones, siempre abajo: A/B con mando de Xbox, ✕/○ con PlayStation, Intro/Esc con teclado
export function HintBar({ hints }: { hints: [string, string][] }) {
  const t = useT()
  const [mode, setMode] = useState<InputMode>(inputMode)
  const [ps, setPs] = useState(padKind() === 'ps')
  useEffect(() => {
    const on = () => { setMode(inputMode()); setPs(padKind() === 'ps') }
    window.addEventListener('inputmode', on)
    return () => window.removeEventListener('inputmode', on)
  }, [])
  const kb = mode === 'keyboard'
  const keys = kb ? keyboardName() : null
  const name = (key: string) => (keys ? keys[key] : ps ? playstationName[key] : undefined) ?? key
  return (
    <footer className="hintbar">
      {hints.map(([key, label]) => {
        const text = name(key)
        return <span key={key}><kbd className={text.length > 1 ? 'kb' : ''}>{text}</kbd> {label}</span>
      })}
      {kb && <span><kbd className="kb">Tab</kbd> {t('shell.hint.next')}</span>}
    </footer>
  )
}
