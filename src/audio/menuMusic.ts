// Música del menú, como en PS4/PS5: suena solo con CouchDesk delante y en las pantallas de menú.
// - Generada al momento con Web Audio (sin archivos ni licencias): acordes lentos que cambian,
//   notas sueltas de campana sobre una escala pentatónica y una sala con eco suave. Nunca se repite igual.
// - O un enlace propio (MP3/OGG) o un vídeo o lista de YouTube.
// - Entra y sale con fundidos de 1,5 s. En silencio, el audio se suspende (no gasta CPU).
// - Empieza tras la primera pulsación (los navegadores no dejan sonar antes).
import { useSyncExternalStore } from 'react'
import type { MusicChoice, MusicPreset } from '../data/themes'
import { createContext, lfo, midiToHz, noiseSource, rampTo, reverb } from './synth'

export const FADE_S = 1.5

// ---------------- Ajustes (encendida y volumen) ----------------

export interface MusicSettings { enabled: boolean; volume: number }
const KEY = 'couchdesk.music.v1'
const DEFAULTS: MusicSettings = { enabled: true, volume: 40 }

function readSettings(): MusicSettings {
  try {
    const s = JSON.parse(localStorage.getItem(KEY) ?? 'null') as Partial<MusicSettings> | null
    if (s) return { enabled: s.enabled !== false, volume: Math.max(0, Math.min(100, Number(s.volume ?? DEFAULTS.volume))) }
  } catch { /* por defecto */ }
  return DEFAULTS
}

let settings = readSettings()
const listeners = new Set<() => void>()
const subscribe = (fn: () => void) => { listeners.add(fn); return () => { listeners.delete(fn) } }

export function setMusicSettings(patch: Partial<MusicSettings>) {
  settings = { ...settings, ...patch }
  try { localStorage.setItem(KEY, JSON.stringify(settings)) } catch { /* solo esta sesión */ }
  listeners.forEach((l) => l())
  sync()
}
export const getMusicSettings = () => settings
export const useMusicSettings = () => useSyncExternalStore(subscribe, () => settings)

// ---------------- Recetas de cada tema ----------------

type Bell = 'rhodes' | 'glass' | 'pluck' | 'flute'

interface Recipe {
  /** Nota base (MIDI) y escala (semitonos) */
  root: number
  scale: number[]
  /** Acordes: semitonos sobre la nota base */
  chords: number[][]
  chordSeconds: number
  pad: { type: OscillatorType; cutoff: number; gain: number; detune: number }
  bell: { kind: Bell; gain: number; every: [number, number]; octave: number }
  noise?: { kind: 'wind' | 'waves'; gain: number }
  room: { seconds: number; wet: number }
}

const PENTA_MAJOR = [0, 2, 4, 7, 9]
const PENTA_MINOR = [0, 3, 5, 7, 10]

export const RECIPES: Record<MusicPreset, Recipe> = {
  // Salón: cálido, piano eléctrico suave en Re
  calido: {
    root: 50, scale: PENTA_MAJOR, chordSeconds: 11,
    chords: [[0, 7, 16, 23], [9, 16, 19, 24], [5, 12, 16, 21], [7, 14, 19, 23]],
    pad: { type: 'triangle', cutoff: 1100, gain: 0.05, detune: 7 },
    bell: { kind: 'rhodes', gain: 0.07, every: [2.4, 6], octave: 2 },
    room: { seconds: 3.2, wet: 0.45 },
  },
  // Noche: colchón oscuro en La menor, notas graves y escasas
  nocturno: {
    root: 45, scale: PENTA_MINOR, chordSeconds: 14,
    chords: [[0, 7, 15, 19], [-4, 3, 8, 15], [-7, 0, 8, 12], [-2, 5, 10, 14]],
    pad: { type: 'sawtooth', cutoff: 520, gain: 0.035, detune: 9 },
    bell: { kind: 'glass', gain: 0.045, every: [4, 9], octave: 2 },
    room: { seconds: 4.2, wet: 0.55 },
  },
  // Cine: casi nada; un colchón limpio y alguna nota alta muy de vez en cuando
  cine: {
    root: 48, scale: [0, 2, 7, 9], chordSeconds: 16,
    chords: [[0, 7, 14, 19], [5, 12, 16, 21], [-3, 4, 12, 16]],
    pad: { type: 'sine', cutoff: 900, gain: 0.06, detune: 4 },
    bell: { kind: 'glass', gain: 0.03, every: [7, 14], octave: 3 },
    room: { seconds: 4.8, wet: 0.6 },
  },
  // Bosque: aireado, flauta suave y viento
  bosque: {
    root: 53, scale: [0, 2, 4, 6, 7, 9, 11], chordSeconds: 12,
    chords: [[0, 7, 16, 18], [2, 9, 14, 21], [-5, 7, 11, 14], [4, 11, 16, 19]],
    pad: { type: 'triangle', cutoff: 1400, gain: 0.035, detune: 6 },
    bell: { kind: 'flute', gain: 0.05, every: [3, 7], octave: 2 },
    noise: { kind: 'wind', gain: 0.05 },
    room: { seconds: 3.6, wet: 0.5 },
  },
  // Costa: cristal en Mi y olas que vienen y van
  costa: {
    root: 52, scale: PENTA_MAJOR, chordSeconds: 12,
    chords: [[0, 7, 11, 16], [-3, 4, 9, 12], [5, 9, 12, 16], [7, 11, 14, 19]],
    pad: { type: 'sine', cutoff: 1300, gain: 0.05, detune: 5 },
    bell: { kind: 'glass', gain: 0.05, every: [2.8, 6.5], octave: 2 },
    noise: { kind: 'waves', gain: 0.09 },
    room: { seconds: 3.4, wet: 0.5 },
  },
}

// ---------------- Reproductores ----------------

interface Player {
  /** 0..1, con fundido */
  setLevel(level: number, seconds: number): void
  dispose(): void
}

const rand = (a: number, b: number) => a + Math.random() * (b - a)
const pick = <T,>(xs: T[]) => xs[Math.floor(Math.random() * xs.length)]

/** Música generada. Programa los sucesos un poco por delante; si el audio se suspende, todo se detiene */
function generative(preset: MusicPreset): Player | null {
  const ctx = createContext()
  if (!ctx) return null
  const r = RECIPES[preset]
  const master = ctx.createGain()
  master.gain.value = 0
  master.connect(ctx.destination)
  const dry = ctx.createGain()
  dry.gain.value = 1 - r.room.wet * 0.5
  const room = reverb(ctx, r.room.seconds, 2.6)
  const wet = ctx.createGain()
  wet.gain.value = r.room.wet
  dry.connect(master)
  room.connect(wet).connect(master)
  const bus = ctx.createGain()
  bus.connect(dry)
  bus.connect(room)

  // Colchón: un filtro que respira muy despacio
  const padFilter = ctx.createBiquadFilter()
  padFilter.type = 'lowpass'
  padFilter.Q.value = 0.4
  padFilter.connect(bus)
  const lfos: OscillatorNode[] = [lfo(ctx, padFilter.frequency, r.pad.cutoff, r.pad.cutoff * 0.35, 1 / 23)]

  // Ruido: viento o olas
  let noise: AudioBufferSourceNode | null = null
  if (r.noise) {
    noise = noiseSource(ctx)
    const f = ctx.createBiquadFilter()
    const g = ctx.createGain()
    if (r.noise.kind === 'wind') {
      f.type = 'bandpass'
      f.Q.value = 0.8
      lfos.push(lfo(ctx, f.frequency, 650, 380, 1 / 17))
      lfos.push(lfo(ctx, g.gain, r.noise.gain, r.noise.gain * 0.7, 1 / 11))
    } else {
      f.type = 'lowpass'
      f.Q.value = 0.3
      lfos.push(lfo(ctx, f.frequency, 700, 400, 1 / 9))
      lfos.push(lfo(ctx, g.gain, r.noise.gain, r.noise.gain * 0.95, 1 / 9))
    }
    noise.connect(f).connect(g).connect(dry)
    g.connect(room)
    noise.start()
  }

  let chordIndex = 0
  let nextChord = ctx.currentTime + 0.1
  let nextBell = ctx.currentTime + rand(1.5, 3)
  let lastNote = -1

  const padNote = (midi: number, t: number, len: number) => {
    const g = ctx.createGain()
    g.gain.setValueAtTime(0, t)
    g.gain.linearRampToValueAtTime(r.pad.gain, t + len * 0.35)
    g.gain.setValueAtTime(r.pad.gain, t + len * 0.75)
    g.gain.linearRampToValueAtTime(0, t + len * 1.35)
    g.connect(padFilter)
    for (const d of [-r.pad.detune, r.pad.detune]) {
      const o = ctx.createOscillator()
      o.type = r.pad.type
      o.frequency.value = midiToHz(midi)
      o.detune.value = d
      o.connect(g)
      o.start(t)
      o.stop(t + len * 1.4)
    }
  }

  const bell = (t: number) => {
    // Notas de la escala, sin repetir la anterior, cerca de la última (melodía tranquila)
    const notes = [0, 1].flatMap((oct) => r.scale.map((s) => r.root + 12 * (r.bell.octave + oct) + s))
    let n = pick(notes)
    if (lastNote > 0) {
      const near = notes.filter((x) => x !== lastNote && Math.abs(x - lastNote) <= 7)
      if (near.length) n = pick(near)
    }
    lastNote = n
    const f = midiToHz(n)
    const pan = ctx.createStereoPanner()
    pan.pan.value = rand(-0.5, 0.5)
    const amp = ctx.createGain()
    amp.connect(pan).connect(bus)
    const peak = r.bell.gain * rand(0.6, 1)
    const car = ctx.createOscillator()
    car.frequency.value = f
    let end: number
    if (r.bell.kind === 'flute') {
      car.type = 'sine'
      amp.gain.setValueAtTime(0, t)
      amp.gain.linearRampToValueAtTime(peak, t + 0.35)
      amp.gain.exponentialRampToValueAtTime(0.0001, t + 2.6)
      const vib = ctx.createOscillator()
      vib.frequency.value = 5
      const vg = ctx.createGain()
      vg.gain.value = f * 0.004
      vib.connect(vg).connect(car.frequency)
      vib.start(t)
      vib.stop(t + 2.7)
      end = t + 2.7
    } else if (r.bell.kind === 'pluck') {
      car.type = 'triangle'
      amp.gain.setValueAtTime(0, t)
      amp.gain.linearRampToValueAtTime(peak, t + 0.008)
      amp.gain.exponentialRampToValueAtTime(0.0001, t + 1.6)
      end = t + 1.7
    } else {
      // Rhodes y cristal: FM con dos senos; el brillo se apaga antes que el tono
      car.type = 'sine'
      const mod = ctx.createOscillator()
      const ratio = r.bell.kind === 'rhodes' ? 1 : 3.5
      mod.frequency.value = f * ratio
      const idx = ctx.createGain()
      const depth = f * (r.bell.kind === 'rhodes' ? 1.6 : 0.9)
      idx.gain.setValueAtTime(depth, t)
      idx.gain.exponentialRampToValueAtTime(depth * 0.05, t + 1.2)
      mod.connect(idx).connect(car.frequency)
      const decay = r.bell.kind === 'rhodes' ? 3.2 : 4
      amp.gain.setValueAtTime(0, t)
      amp.gain.linearRampToValueAtTime(peak, t + 0.006)
      amp.gain.exponentialRampToValueAtTime(0.0001, t + decay)
      end = t + decay + 0.1
      mod.start(t)
      mod.stop(end)
    }
    car.connect(amp)
    car.start(t)
    car.stop(end)
  }

  const tick = () => {
    if (ctx.state !== 'running') return
    const horizon = ctx.currentTime + 1.5
    while (nextChord < horizon) {
      const chord = r.chords[chordIndex++ % r.chords.length]
      for (const s of chord) padNote(r.root + s, nextChord, r.chordSeconds)
      nextChord += r.chordSeconds
    }
    while (nextBell < horizon) {
      bell(nextBell)
      // A veces dos notas seguidas, como una frase corta
      nextBell += Math.random() < 0.25 ? rand(0.35, 0.7) : rand(...r.bell.every)
    }
  }
  const timer = window.setInterval(tick, 400)
  let suspendTimer = 0

  return {
    setLevel(level, seconds) {
      clearTimeout(suspendTimer)
      if (level > 0 && ctx.state !== 'running') ctx.resume().then(tick).catch(() => {})
      // Curva de volumen perceptiva y techo bajo: música de fondo
      rampTo(master.gain, 0.55 * level * level, seconds, ctx)
      if (level === 0) suspendTimer = window.setTimeout(() => { ctx.suspend().catch(() => {}) }, seconds * 1000 + 300)
    },
    dispose() {
      clearInterval(timer)
      clearTimeout(suspendTimer)
      lfos.forEach((o) => { try { o.stop() } catch { /* ya parado */ } })
      try { noise?.stop() } catch { /* ya parado */ }
      ctx.close().catch(() => {})
    },
  }
}

/** Fundido de un volumen que solo se puede cambiar a pasos (audio HTML y YouTube) */
function stepper(apply: (v: number) => void, start = 0) {
  let current = start
  let timer = 0
  return (target: number, seconds: number, done?: () => void) => {
    clearInterval(timer)
    const from = current
    const t0 = performance.now()
    timer = window.setInterval(() => {
      const k = Math.min(1, (performance.now() - t0) / (seconds * 1000))
      current = from + (target - from) * k
      apply(current)
      if (k >= 1) { clearInterval(timer); done?.() }
    }, 50)
  }
}

/** Enlace propio a un MP3/OGG (o una radio por internet) */
function fromUrl(url: string): Player {
  const el = new Audio()
  el.src = url
  el.loop = true
  el.preload = 'none'
  el.volume = 0
  el.dataset.couchdeskMusic = ''
  const fade = stepper((v) => { el.volume = Math.max(0, Math.min(1, v)) })
  return {
    setLevel(level, seconds) {
      if (level > 0 && el.paused) el.play().catch(() => {})
      fade(level * level * 0.8, seconds, () => { if (level === 0) el.pause() })
    },
    dispose() { el.pause(); el.removeAttribute('src'); el.load() },
  }
}

/** Vídeo o lista de YouTube: un reproductor oculto, con sonido */
function fromYouTube(videoId: string | null, playlistId: string | null): Player {
  const p = new URLSearchParams({
    autoplay: '0', mute: '0', loop: '1', controls: '0', playsinline: '1', disablekb: '1', fs: '0', rel: '0', enablejsapi: '1',
    origin: window.location.origin,
  })
  if (playlistId) { p.set('list', playlistId); p.set('listType', 'playlist') } else if (videoId) p.set('playlist', videoId)
  const frame = document.createElement('iframe')
  frame.src = `https://www.youtube-nocookie.com/embed/${videoId ?? 'videoseries'}?${p}`
  frame.allow = 'autoplay; encrypted-media'
  frame.tabIndex = -1
  frame.title = 'music'
  frame.setAttribute('aria-hidden', 'true')
  Object.assign(frame.style, { position: 'fixed', width: '2px', height: '2px', left: '-10px', top: '-10px', opacity: '0', pointerEvents: 'none', border: '0' })
  document.body.appendChild(frame)
  const send = (func: string, args: unknown[] = []) => frame.contentWindow?.postMessage(JSON.stringify({ event: 'command', func, args }), '*')
  let playing = false
  let wanted = 0
  frame.addEventListener('load', () => {
    frame.contentWindow?.postMessage(JSON.stringify({ event: 'listening', id: 'musica', channel: 'widget' }), '*')
    send('setVolume', [0])
    if (wanted > 0) { send('playVideo'); playing = true }
  })
  const fade = stepper((v) => send('setVolume', [Math.round(v * 100)]))
  return {
    setLevel(level, seconds) {
      wanted = level
      if (level > 0 && !playing) { send('playVideo'); playing = true }
      fade(level * level * 0.8, seconds, () => { if (level === 0) { send('pauseVideo'); playing = false } })
    },
    dispose() { frame.remove() },
  }
}

function create(choice: MusicChoice): Player | null {
  if (choice.kind === 'preset') return generative(choice.preset)
  if (choice.kind === 'url') return fromUrl(choice.url)
  if (choice.kind === 'youtube') return fromYouTube(choice.videoId, choice.playlistId)
  return null
}

// ---------------- Mando central ----------------

let source: MusicChoice = { kind: 'off' }
let audible = false
let unlocked = false
let player: Player | null = null
let playerKey = ''

const keyOf = (c: MusicChoice) => JSON.stringify(c)

function sync() {
  const want = settings.enabled && audible && unlocked && source.kind !== 'off'
  const key = keyOf(source)
  if (player && playerKey !== key) {
    // Otra música: la de antes se va con un fundido y luego se libera
    const old = player
    old.setLevel(0, FADE_S)
    window.setTimeout(() => old.dispose(), FADE_S * 1000 + 200)
    player = null
  }
  if (want && !player) {
    player = create(source)
    playerKey = key
  }
  player?.setLevel(want ? settings.volume / 100 : 0, FADE_S)
}

/** Qué música toca (la del tema activo, o la que se está probando en el editor) */
export function setMusicSource(choice: MusicChoice) {
  if (keyOf(choice) === keyOf(source)) return
  source = choice
  sync()
}

/** ¿Se puede oír ahora? (ventana delante, pantalla de menú, nada más sonando) */
export function setMusicAudible(on: boolean) {
  if (on === audible) return
  audible = on
  sync()
}

/** Primera pulsación: a partir de aquí ya se puede sonar */
export function unlockMusic() {
  if (unlocked) return
  unlocked = true
  sync()
}
