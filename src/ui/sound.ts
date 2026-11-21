// Sonidos de interfaz sintetizados (sin archivos): suaves, cortos y cálidos, como en una consola

let ctx: AudioContext | null = null
let muted = false
try { muted = localStorage.getItem('consola.sonidos') === 'no' } catch { /* sin almacenamiento */ }

function audio() {
  if (!ctx) {
    try { ctx = new AudioContext() } catch { return null }
  }
  if (ctx.state === 'suspended') ctx.resume().catch(() => {})
  return ctx
}

function tone(freq: number, to: number, ms: number, gain: number, delay = 0, type: OscillatorType = 'sine') {
  const a = audio()
  if (!a || muted) return
  const t = a.currentTime + delay / 1000
  const osc = a.createOscillator()
  const g = a.createGain()
  osc.type = type
  osc.frequency.setValueAtTime(freq, t)
  osc.frequency.exponentialRampToValueAtTime(to, t + ms / 1000)
  g.gain.setValueAtTime(0, t)
  g.gain.linearRampToValueAtTime(gain, t + 0.006)
  g.gain.exponentialRampToValueAtTime(0.0001, t + ms / 1000)
  osc.connect(g).connect(a.destination)
  osc.start(t)
  osc.stop(t + ms / 1000 + 0.02)
}

export const sound = {
  move: () => tone(1900, 1500, 45, 0.025),
  select: () => { tone(740, 990, 90, 0.05); tone(1480, 1980, 120, 0.02, 30) },
  back: () => tone(620, 410, 110, 0.04),
  boot: () => {
    // Acorde que se abre poco a poco
    ;[392, 494, 587, 784].forEach((f, i) => tone(f, f * 1.002, 1400, 0.028, i * 90, 'triangle'))
  },
  get muted() { return muted },
  setMuted(v: boolean) {
    muted = v
    try { localStorage.setItem('consola.sonidos', v ? 'no' : 'si') } catch { /* nada */ }
  },
}

// Qué se está usando: mando, teclado o ratón. Con mando o teclado se esconde el puntero
// y la leyenda de abajo enseña las teclas que tocan.
export type InputMode = 'pad' | 'keyboard' | 'mouse'

export function inputMode(): InputMode {
  return (document.documentElement.dataset.input as InputMode) ?? 'pad'
}

// Qué mando es: de PlayStation (✕ ○ □ △) o de Xbox (A B X Y)
export function padKind(): 'ps' | 'xbox' {
  return document.documentElement.dataset.pad === 'ps' ? 'ps' : 'xbox'
}

export function setPadKind(id: string) {
  const kind = /054c|playstation|dualsense|dualshock/i.test(id) ? 'ps' : 'xbox'
  const html = document.documentElement
  if (html.dataset.pad === kind) return
  html.dataset.pad = kind
  window.dispatchEvent(new Event('inputmode'))
}

export function trackInputMode() {
  const html = document.documentElement
  const set = (m: InputMode) => {
    if (html.dataset.input === m) return
    html.dataset.input = m
    window.dispatchEvent(new Event('inputmode'))
  }
  const keyboard = () => set('keyboard')
  const pad = () => set('pad')
  // Movimientos mínimos del ratón (vibraciones) no cuentan
  let lx = -1, ly = -1
  const mouse = (e: MouseEvent) => {
    if (lx >= 0 && Math.abs(e.screenX - lx) + Math.abs(e.screenY - ly) > 6) set('mouse')
    lx = e.screenX; ly = e.screenY
  }
  window.addEventListener('keydown', keyboard)
  window.addEventListener('padinput', pad)
  window.addEventListener('mousemove', mouse)
  html.dataset.input = 'pad'
  return () => {
    window.removeEventListener('keydown', keyboard)
    window.removeEventListener('padinput', pad)
    window.removeEventListener('mousemove', mouse)
  }
}
