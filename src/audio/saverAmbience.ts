// Sonido del salvapantallas: ambientes hechos con ruido filtrado (sin archivos).
// Mar, viento, lluvia y fuego. Muy poca CPU: un ruido en bucle y unos filtros que se mueven despacio.
import { createContext, lfo, noiseSource, rampTo } from './synth'

export type AmbienceSound = 'mar' | 'viento' | 'lluvia' | 'fuego'

export interface AmbiencePlayer {
  setLevel(level: number, seconds: number): void
  dispose(): void
}

export function playAmbience(kind: AmbienceSound): AmbiencePlayer | null {
  const ctx = createContext()
  if (!ctx) return null
  const master = ctx.createGain()
  master.gain.value = 0
  master.connect(ctx.destination)
  const stops: (() => void)[] = []
  const layer = (type: BiquadFilterType, freq: number, q: number, gain: number) => {
    const src = noiseSource(ctx)
    const f = ctx.createBiquadFilter()
    f.type = type
    f.frequency.value = freq
    f.Q.value = q
    const g = ctx.createGain()
    g.gain.value = gain
    src.connect(f).connect(g).connect(master)
    src.start(0, Math.random() * 3)
    stops.push(() => src.stop())
    return { f, g }
  }
  const slow = (param: AudioParam, base: number, depth: number, hz: number) => {
    const o = lfo(ctx, param, base, depth, hz)
    stops.push(() => o.stop())
  }
  let timer = 0

  if (kind === 'mar') {
    // Olas: un rugido grave que sube y baja cada ~9 s, con espuma aguda encima
    const body = layer('lowpass', 500, 0.3, 0.6)
    slow(body.f.frequency, 520, 360, 1 / 9)
    slow(body.g.gain, 0.5, 0.45, 1 / 9)
    const foam = layer('highpass', 2600, 0.4, 0.08)
    slow(foam.g.gain, 0.08, 0.07, 1 / 9)
  } else if (kind === 'viento') {
    const gust = layer('bandpass', 600, 0.9, 0.5)
    slow(gust.f.frequency, 620, 380, 1 / 13)
    slow(gust.g.gain, 0.45, 0.3, 1 / 7)
    const air = layer('lowpass', 350, 0.2, 0.25)
    slow(air.g.gain, 0.25, 0.12, 1 / 19)
  } else if (kind === 'lluvia') {
    layer('bandpass', 3200, 0.5, 0.35)
    layer('lowpass', 500, 0.3, 0.2)
    const hiss = layer('highpass', 6000, 0.3, 0.06)
    slow(hiss.g.gain, 0.06, 0.02, 1 / 11)
  } else {
    // Fuego: rumor grave y chasquidos sueltos
    const rumble = layer('lowpass', 260, 0.4, 0.45)
    slow(rumble.g.gain, 0.42, 0.12, 1 / 5)
    const crackle = () => {
      if (ctx.state !== 'running') return
      const now = ctx.currentTime
      const n = Math.floor(Math.random() * 4)
      for (let i = 0; i < n; i++) {
        const t = now + Math.random() * 0.45
        const src = noiseSource(ctx)
        const f = ctx.createBiquadFilter()
        f.type = 'bandpass'
        f.frequency.value = 1800 + Math.random() * 3200
        f.Q.value = 1.5
        const g = ctx.createGain()
        const peak = 0.15 + Math.random() * 0.45
        g.gain.setValueAtTime(0, t)
        g.gain.linearRampToValueAtTime(peak, t + 0.002)
        g.gain.exponentialRampToValueAtTime(0.0001, t + 0.03 + Math.random() * 0.05)
        src.connect(f).connect(g).connect(master)
        src.start(t, Math.random() * 3)
        src.stop(t + 0.12)
      }
    }
    timer = window.setInterval(crackle, 450)
  }

  return {
    setLevel(level, seconds) {
      if (level > 0 && ctx.state !== 'running') ctx.resume().catch(() => {})
      rampTo(master.gain, 0.7 * level * level, seconds, ctx)
    },
    dispose() {
      clearInterval(timer)
      stops.forEach((s) => { try { s() } catch { /* ya parado */ } })
      ctx.close().catch(() => {})
    },
  }
}

/** Qué ambiente va con cada colección de paisajes */
export function soundForCollection(collection: string | null, style: string): AmbienceSound | null {
  if (style === 'reloj') return 'fuego'
  if (style !== 'paisajes') return null
  switch (collection) {
    case 'costas': return 'mar'
    case 'agua': case 'ciudades': return 'lluvia'
    case 'montanas': case 'nubes': case 'bosques': return 'viento'
    default: return 'viento'
  }
}
