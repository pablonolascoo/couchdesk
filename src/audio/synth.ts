// Piezas de sonido generado (sin archivos): ruido, reverberación y modulación lenta.
// Las usan la música del menú y el sonido del salvapantallas.

/** Contexto de audio pensado para música (búfer más grande = menos CPU) */
export function createContext(): AudioContext | null {
  try { return new AudioContext({ latencyHint: 'playback' }) } catch { return null }
}

const noiseCache = new WeakMap<BaseAudioContext, AudioBuffer>()

/** 4 s de ruido (rosa aproximado: más suave que el blanco), en bucle sin costuras */
export function noiseBuffer(ctx: BaseAudioContext): AudioBuffer {
  const cached = noiseCache.get(ctx)
  if (cached) return cached
  const len = ctx.sampleRate * 4
  const buf = ctx.createBuffer(2, len, ctx.sampleRate)
  for (let ch = 0; ch < 2; ch++) {
    const d = buf.getChannelData(ch)
    let b0 = 0, b1 = 0, b2 = 0
    for (let i = 0; i < len; i++) {
      const w = Math.random() * 2 - 1
      b0 = 0.99765 * b0 + w * 0.099046
      b1 = 0.963 * b1 + w * 0.2965164
      b2 = 0.57 * b2 + w * 1.0526913
      d[i] = (b0 + b1 + b2 + w * 0.1848) * 0.18
    }
    // Fundido en los bordes para que el bucle no haga clic
    const fade = Math.floor(ctx.sampleRate * 0.05)
    for (let i = 0; i < fade; i++) {
      const k = i / fade
      d[i] *= k
      d[len - 1 - i] *= k
    }
  }
  noiseCache.set(ctx, buf)
  return buf
}

export function noiseSource(ctx: BaseAudioContext): AudioBufferSourceNode {
  const src = ctx.createBufferSource()
  src.buffer = noiseBuffer(ctx)
  src.loop = true
  return src
}

/** Sala generada: ruido que se apaga poco a poco (reverberación suave) */
export function reverb(ctx: BaseAudioContext, seconds: number, decay = 3): ConvolverNode {
  const len = Math.floor(ctx.sampleRate * seconds)
  const buf = ctx.createBuffer(2, len, ctx.sampleRate)
  for (let ch = 0; ch < 2; ch++) {
    const d = buf.getChannelData(ch)
    for (let i = 0; i < len; i++) d[i] = (Math.random() * 2 - 1) * (1 - i / len) ** decay
  }
  const conv = ctx.createConvolver()
  conv.buffer = buf
  return conv
}

/** Oscilador lento que mueve un parámetro: base ± depth a `hz` vueltas por segundo */
export function lfo(ctx: BaseAudioContext, param: AudioParam, base: number, depth: number, hz: number, type: OscillatorType = 'sine') {
  param.value = base
  const osc = ctx.createOscillator()
  osc.type = type
  osc.frequency.value = hz
  const g = ctx.createGain()
  g.gain.value = depth
  osc.connect(g).connect(param)
  osc.start()
  return osc
}

export const midiToHz = (m: number) => 440 * 2 ** ((m - 69) / 12)

/** Sube o baja un volumen sin clics */
export function rampTo(param: AudioParam, value: number, seconds: number, ctx: BaseAudioContext) {
  const t = ctx.currentTime
  param.cancelScheduledValues(t)
  param.setValueAtTime(param.value, t)
  param.linearRampToValueAtTime(value, t + Math.max(0.02, seconds))
}
