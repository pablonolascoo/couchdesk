// Temas de CouchDesk: colores, letra, esquinas, fondo y música del menú.
// - Los integrados no se pueden borrar ni cambiar; se pueden duplicar.
// - Los propios se guardan en localStorage "couchdesk.theme.v1" junto con las variables ya
//   calculadas del tema activo: index.html las pone antes de pintar (sin destellos).
// - Las otras ventanas (asistente, salvapantallas) se enteran por el evento "storage".
import { useSyncExternalStore } from 'react'
import { getBackdrop, parseYouTube, setBackdrop, type BackdropChoice } from './backdrop'
import {
  PALETTES, BUILTIN_IDS, derivePalette, isHex, normHex, varsFor,
  type BuiltinId, type Corners, type FontSet, type Mode, type Palette,
} from './themePalettes'

export type { BuiltinId, Corners, FontSet, Mode }

export type MusicPreset = 'calido' | 'nocturno' | 'cine' | 'bosque' | 'costa'
export const MUSIC_PRESETS: MusicPreset[] = ['calido', 'nocturno', 'cine', 'bosque', 'costa']

export type MusicChoice =
  | { kind: 'preset'; preset: MusicPreset }
  | { kind: 'url'; url: string }
  | { kind: 'youtube'; url: string; videoId: string | null; playlistId: string | null }
  | { kind: 'off' }

export interface Theme {
  id: string
  /** Nombre propio (los integrados usan su clave de traducción) */
  name: string
  builtIn?: boolean
  base: BuiltinId
  mode: Mode
  /** null = el del tema base */
  accent: string | null
  font: FontSet
  corners: Corners
  /** null = no cambia el fondo al elegir el tema */
  backdrop: BackdropChoice | null
  music: MusicChoice
}

const builtin = (id: BuiltinId, backdrop: BackdropChoice, preset: MusicPreset): Theme => ({
  id, name: `themes.builtin.${id}`, builtIn: true, base: id, mode: PALETTES[id].mode,
  accent: null, font: 'editorial', corners: id === 'alto-contraste' ? 'recto' : 'suave', backdrop, music: { kind: 'preset', preset },
})

export const BUILTINS: Theme[] = [
  builtin('salon', { kind: 'animated', style: 'ondas' }, 'calido'),
  builtin('noche', { kind: 'animated', style: 'aurora' }, 'nocturno'),
  builtin('cine', { kind: 'animated', style: 'plano' }, 'cine'),
  builtin('bosque', { kind: 'animated', style: 'particulas' }, 'bosque'),
  builtin('costa', { kind: 'animated', style: 'ondas' }, 'costa'),
  builtin('alto-contraste', { kind: 'animated', style: 'plano' }, 'cine'),
]

export const DEFAULT_ID: BuiltinId = 'salon'

// ---------------- Almacén ----------------

const KEY = 'couchdesk.theme.v1'

interface Stored { active: string; custom: Theme[]; applied?: string; vars?: Record<string, string> }
interface State { active: string; custom: Theme[] }

function read(): State {
  try {
    const s = JSON.parse(localStorage.getItem(KEY) ?? 'null') as Stored | null
    if (s && typeof s.active === 'string' && Array.isArray(s.custom)) {
      return { active: s.active, custom: s.custom.map(sanitize).filter((t): t is Theme => !!t) }
    }
  } catch { /* sin almacenamiento o dañado: valores por defecto */ }
  return { active: DEFAULT_ID, custom: [] }
}

let state = read()
const listeners = new Set<() => void>()
const subscribe = (fn: () => void) => { listeners.add(fn); return () => { listeners.delete(fn) } }
const emit = () => listeners.forEach((l) => l())

export const allThemes = () => [...BUILTINS, ...state.custom]
export const findTheme = (id: string) => allThemes().find((t) => t.id === id) ?? null
export const activeTheme = () => findTheme(state.active) ?? BUILTINS[0]

export function paletteOf(t: Theme): Palette {
  return t.builtIn ? PALETTES[t.base] : derivePalette(t.base, t.mode, t.accent)
}
export const themeVars = (t: Theme) => varsFor(paletteOf(t), t.font, t.corners)

/** Guarda el estado y las variables ya calculadas (las lee index.html antes de pintar) */
export function persistTheme() { save() }

function save() {
  const t = activeTheme()
  const stored: Stored = { active: state.active, custom: state.custom, applied: t.builtIn ? t.id : 'propio', vars: themeVars(t) }
  try { localStorage.setItem(KEY, JSON.stringify(stored)) } catch { /* solo dura esta sesión */ }
}

/** Escribe las variables del tema en :root (cambia toda la interfaz al momento) */
export function applyToDocument(t: Theme = activeTheme()) {
  const root = document.documentElement
  for (const [k, v] of Object.entries(themeVars(t))) root.style.setProperty(k, v)
  root.dataset.theme = t.builtIn ? t.id : 'propio'
  root.dataset.mode = paletteOf(t).mode
}

function commit(next: State) {
  state = next
  save()
  applyToDocument()
  emit()
}

/** Usar un tema: colores al momento y su fondo (si tiene) */
export function setActiveTheme(id: string) {
  const t = findTheme(id)
  if (!t) return
  commit({ ...state, active: id })
  if (t.backdrop && JSON.stringify(getBackdrop().choice) !== JSON.stringify(t.backdrop)) setBackdrop({ choice: t.backdrop })
}

const newId = () => `t-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`

export function saveTheme(t: Theme): Theme {
  const clean = sanitize({ ...t, builtIn: false })
  if (!clean) return t
  const exists = state.custom.some((c) => c.id === clean.id)
  commit({ ...state, custom: exists ? state.custom.map((c) => (c.id === clean.id ? clean : c)) : [...state.custom, clean] })
  return clean
}

export function deleteTheme(id: string) {
  commit({ active: state.active === id ? DEFAULT_ID : state.active, custom: state.custom.filter((c) => c.id !== id) })
}

/** Copia editable de cualquier tema */
export function duplicateOf(t: Theme, name: string): Theme {
  return { ...t, id: newId(), name, builtIn: false }
}

export function blankTheme(name: string, base: BuiltinId = 'salon'): Theme {
  return { ...BUILTINS.find((b) => b.id === base)!, id: newId(), name, builtIn: false }
}

// Otras ventanas: si cambia el tema aquí, allí también
if (typeof window !== 'undefined') {
  window.addEventListener('storage', (e) => {
    if (e.key !== KEY) return
    state = read()
    applyToDocument()
    emit()
  })
}

export function useThemes() {
  const s = useSyncExternalStore(subscribe, () => state)
  return { active: s.active, custom: s.custom, all: [...BUILTINS, ...s.custom] }
}

// ---------------- Validar y compartir ----------------

const FONT_SETS: FontSet[] = ['editorial', 'moderna', 'tecnica']
const CORNER_SET: Corners[] = ['suave', 'redondo', 'recto']

function sanitizeMusic(m: unknown): MusicChoice {
  const x = m as Partial<Record<string, unknown>> | null
  if (x?.kind === 'off') return { kind: 'off' }
  if (x?.kind === 'preset' && MUSIC_PRESETS.includes(x.preset as MusicPreset)) return { kind: 'preset', preset: x.preset as MusicPreset }
  if (x?.kind === 'url' && typeof x.url === 'string' && /^https?:\/\//i.test(x.url)) return { kind: 'url', url: x.url.slice(0, 500) }
  if (x?.kind === 'youtube' && typeof x.url === 'string') {
    const yt = parseYouTube(x.url)
    if (yt) return { kind: 'youtube', url: x.url.slice(0, 500), ...yt }
  }
  return { kind: 'preset', preset: 'calido' }
}

function sanitizeBackdrop(b: unknown): BackdropChoice | null {
  const x = b as Partial<Record<string, unknown>> | null
  if (!x || typeof x !== 'object') return null
  if (x.kind === 'animated' && ['ondas', 'aurora', 'particulas', 'plano'].includes(x.style as string)) return x as BackdropChoice
  if (x.kind === 'youtube' && typeof x.url === 'string') {
    const yt = parseYouTube(x.url)
    return yt ? { kind: 'youtube', url: x.url, ...yt } : null
  }
  // Imágenes, vídeos y modelos de enlace se comparten; los archivos del PC solo sirven en este PC
  if ((x.kind === 'image' || x.kind === 'video' || x.kind === 'model') && typeof x.src === 'string') {
    return { kind: x.kind, src: x.src, name: typeof x.name === 'string' ? x.name : '' }
  }
  return null
}

export function sanitize(t: unknown): Theme | null {
  const x = t as Partial<Theme> | null
  if (!x || typeof x !== 'object') return null
  const base = BUILTIN_IDS.includes(x.base as BuiltinId) ? (x.base as BuiltinId) : 'salon'
  const name = typeof x.name === 'string' && x.name.trim() ? x.name.trim().slice(0, 40) : 'Tema'
  return {
    id: typeof x.id === 'string' && x.id ? x.id.slice(0, 40) : newId(),
    name,
    builtIn: false,
    base,
    mode: x.mode === 'dark' || x.mode === 'light' ? x.mode : PALETTES[base].mode,
    accent: typeof x.accent === 'string' && isHex(x.accent) ? normHex(x.accent) : null,
    font: FONT_SETS.includes(x.font as FontSet) ? (x.font as FontSet) : 'editorial',
    corners: CORNER_SET.includes(x.corners as Corners) ? (x.corners as Corners) : 'suave',
    backdrop: sanitizeBackdrop(x.backdrop),
    music: sanitizeMusic(x.music),
  }
}

const PREFIX = 'COUCHDESK-TEMA-1:'

/** Código para compartir: texto corto que se puede pegar en otra CouchDesk */
export function exportCode(t: Theme): string {
  const { id: _id, builtIn: _b, ...rest } = t // eslint-disable-line @typescript-eslint/no-unused-vars
  const json = JSON.stringify(rest)
  const b64 = btoa(String.fromCharCode(...new TextEncoder().encode(json))).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
  return PREFIX + b64
}

/** Lee un código o un JSON pegado. Devuelve un tema nuevo (sin guardar) o null */
export function importCode(text: string): Theme | null {
  const raw = text.trim()
  try {
    let json = raw
    if (raw.startsWith(PREFIX)) {
      const b64 = raw.slice(PREFIX.length).replace(/-/g, '+').replace(/_/g, '/')
      const bin = atob(b64 + '='.repeat((4 - (b64.length % 4)) % 4))
      json = new TextDecoder().decode(Uint8Array.from(bin, (c) => c.charCodeAt(0)))
    }
    const t = sanitize({ ...JSON.parse(json), id: newId() })
    return t
  } catch {
    return null
  }
}

// ---------------- Borrador del editor ----------------
// Sobrevive a ir a Ajustes → Fondo y volver (el editor está en varias pantallas)

let draft: Theme | null = null
export const getDraft = () => draft
export function setDraft(t: Theme | null) { draft = t; emit() }
export function useDraft() {
  return useSyncExternalStore(subscribe, () => draft)
}
