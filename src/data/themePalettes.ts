// Paletas de los temas de CouchDesk y cuentas de contraste.
// Sin importaciones: lo usa también scripts/check-themes.ts (node) para comprobar el contraste.
// Capa 1 = colores de marca; capa 2 = papeles (fondo, superficie, texto, acento…) que cambia cada tema.

export type Mode = 'light' | 'dark'
export type FontSet = 'editorial' | 'moderna' | 'tecnica'
export type Corners = 'suave' | 'redondo' | 'recto'

/** Colores de marca (BRAND.md) */
export const BRAND = {
  tinta: '#1F1A17', lino: '#F3ECE1', brasa: '#E8643A', arena: '#E6DCCB', taupe: '#8C7F72', noche: '#14110F',
} as const

/** Papeles de color de un tema. Los "rgb" son tripletes "r g b" para usar con transparencia */
export interface Palette {
  mode: Mode
  bg: string; bg2: string; surface: string; surface2: string
  text: string; text2: string; muted: string
  accent: string; accentInk: string; focus: string
  success: string; successBg: string; warn: string; warnBg: string
  error: string; errorBg: string; info: string; infoBg: string
  /** Opacidad de las líneas finas (color del texto) */
  hair: number
  /** Brillo del borde superior de las piezas */
  highlight: string
  /** Color de las sombras (triplete) */
  shadow: string
  /** Fondo animado: degradado y tres tintes (tripletes) y la luz de las olas */
  bdTop: string; bdMid: string; bdBottom: string
  bdTint1: string; bdTint2: string; bdTint3: string; bdLight: string
  /** Opacidad del grano de papel */
  grain: number
}

const salon: Palette = {
  mode: 'light',
  bg: '#F3ECE1', bg2: '#EBE2D3', surface: '#FBF7F0', surface2: '#F5EEE3',
  text: BRAND.tinta, text2: '#3D342B', muted: '#6F6358',
  // Foco: Brasa un poco más honda para llegar a 3:1 sobre Lino (la Brasa pura da 2,8:1)
  accent: BRAND.brasa, accentInk: BRAND.tinta, focus: '#D9562E',
  success: '#3B6340', successBg: '#E3ECDD', warn: '#7A5710', warnBg: '#F4E8CF',
  error: '#983829', errorBg: '#F6E1DB', info: '#2B5B7C', infoBg: '#E0EBF2',
  hair: 0.09, highlight: 'rgba(255, 255, 255, 0.9)', shadow: '80 56 30',
  bdTop: '#F6F0E6', bdMid: '#F1E9DC', bdBottom: '#EBE0CF',
  bdTint1: '232 196 148', bdTint2: '236 168 132', bdTint3: '196 184 150', bdLight: '252 249 243',
  grain: 0.05,
}

const noche: Palette = {
  mode: 'dark',
  bg: BRAND.noche, bg2: '#1A1613', surface: '#201B18', surface2: '#2A2420',
  text: BRAND.lino, text2: '#DDD3C5', muted: '#A89B8C',
  accent: BRAND.brasa, accentInk: BRAND.tinta, focus: BRAND.brasa,
  success: '#9CC79A', successBg: '#1E2A1F', warn: '#E6BE6E', warnBg: '#2C2416',
  error: '#F29A86', errorBg: '#331D18', info: '#93BFDB', infoBg: '#18232B',
  hair: 0.11, highlight: 'rgba(255, 255, 255, 0.06)', shadow: '0 0 0',
  bdTop: '#14110F', bdMid: '#181411', bdBottom: '#1E1814',
  bdTint1: '232 100 58', bdTint2: '170 110 76', bdTint3: '96 76 60', bdLight: '44 37 32',
  grain: 0.035,
}

const cine: Palette = {
  mode: 'dark',
  bg: '#0A0807', bg2: '#0F0C0A', surface: '#171311', surface2: '#221C19',
  text: '#FBF6EE', text2: '#E9E0D3', muted: '#C2B5A5',
  accent: '#F0703F', accentInk: BRAND.noche, focus: '#F0703F',
  success: '#A6D6A2', successBg: '#14201A', warn: '#F0CB80', warnBg: '#241C0C',
  error: '#FFA591', errorBg: '#2A1410', info: '#A2CCEA', infoBg: '#0F1B24',
  hair: 0.15, highlight: 'rgba(255, 255, 255, 0.05)', shadow: '0 0 0',
  bdTop: '#0A0807', bdMid: '#0C0A08', bdBottom: '#110D0B',
  bdTint1: '150 70 40', bdTint2: '80 46 30', bdTint3: '46 36 30', bdLight: '30 24 21',
  grain: 0.025,
}

const bosque: Palette = {
  mode: 'dark',
  bg: '#121C17', bg2: '#16221C', surface: '#1C2A22', surface2: '#25362C',
  text: '#F1EBDC', text2: '#DAD3C1', muted: '#A9B5A2',
  accent: '#E3A35C', accentInk: BRAND.noche, focus: '#E3A35C',
  success: '#A4D39A', successBg: '#1E3125', warn: '#EBC57E', warnBg: '#2D2A19',
  error: '#F49B87', errorBg: '#35211B', info: '#9CC8D4', infoBg: '#182B2F',
  hair: 0.11, highlight: 'rgba(255, 255, 255, 0.06)', shadow: '0 8 4',
  bdTop: '#121C17', bdMid: '#15211A', bdBottom: '#1B2A20',
  bdTint1: '120 168 116', bdTint2: '227 163 92', bdTint3: '70 112 84', bdLight: '38 56 44',
  grain: 0.035,
}

const costa: Palette = {
  mode: 'light',
  bg: '#F2ECE0', bg2: '#E8E2D3', surface: '#FBF8F1', surface2: '#F2EEE5',
  text: '#14262E', text2: '#2C4049', muted: '#4D646E',
  accent: '#2F6F8F', accentInk: '#FBF8F1', focus: '#2F6F8F',
  success: '#2F6544', successBg: '#DFEDE2', warn: '#77560F', warnBg: '#F3E7CB',
  error: '#963A2E', errorBg: '#F5E0DA', info: '#245E80', infoBg: '#DDEAF1',
  hair: 0.09, highlight: 'rgba(255, 255, 255, 0.9)', shadow: '30 60 80',
  bdTop: '#F4EFE5', bdMid: '#ECEDE6', bdBottom: '#DDE8EA',
  bdTint1: '150 196 212', bdTint2: '232 200 150', bdTint3: '118 170 182', bdLight: '252 251 247',
  grain: 0.045,
}

const altoContraste: Palette = {
  mode: 'dark',
  bg: '#000000', bg2: '#0A0A0A', surface: '#121212', surface2: '#1C1C1C',
  text: '#FFFFFF', text2: '#F4F1EC', muted: '#D9D2C7',
  accent: '#FF9670', accentInk: '#000000', focus: '#FFD24A',
  success: '#A3EBA3', successBg: '#0C1A0C', warn: '#FFD889', warnBg: '#1E1704',
  error: '#FFA696', errorBg: '#260C08', info: '#A6D8FF', infoBg: '#08161F',
  hair: 0.5, highlight: 'rgba(255, 255, 255, 0)', shadow: '0 0 0',
  bdTop: '#000000', bdMid: '#000000', bdBottom: '#000000',
  bdTint1: '0 0 0', bdTint2: '0 0 0', bdTint3: '0 0 0', bdLight: '0 0 0',
  grain: 0,
}

export const PALETTES = { salon, noche, cine, bosque, costa, 'alto-contraste': altoContraste } as const
export type BuiltinId = keyof typeof PALETTES
export const BUILTIN_IDS = Object.keys(PALETTES) as BuiltinId[]

// ---------------- Contraste (WCAG 2.x) ----------------

export function hexToRgb(hex: string): [number, number, number] {
  let h = hex.trim().replace('#', '')
  if (h.length === 3) h = h.split('').map((c) => c + c).join('')
  const n = parseInt(h.slice(0, 6), 16)
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255]
}

export const isHex = (s: string) => /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.test(s.trim())
export const normHex = (s: string) => {
  const h = s.trim().replace('#', '')
  return `#${(h.length === 3 ? h.split('').map((c) => c + c).join('') : h).toUpperCase()}`
}
const triplet = (hex: string) => hexToRgb(hex).join(' ')

export function luminance(hex: string) {
  const [r, g, b] = hexToRgb(hex).map((v) => {
    const c = v / 255
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
  })
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

export function contrast(a: string, b: string) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x)
  return (hi + 0.05) / (lo + 0.05)
}

export interface ContrastCheck { pair: string; ratio: number; min: number; ok: boolean }

/** Parejas que deben leerse bien. `aaa`: texto a 7:1 (tema de alto contraste) */
export function checkPalette(p: Palette, aaa = false): ContrastCheck[] {
  const text = aaa ? 7 : 4.5
  const pairs: [string, string, string, number][] = [
    ['text/bg', p.text, p.bg, text], ['text/surface', p.text, p.surface, text], ['text/surface-2', p.text, p.surface2, text],
    ['text-2/bg', p.text2, p.bg, text], ['text-2/surface', p.text2, p.surface, text],
    ['muted/bg', p.muted, p.bg, text], ['muted/surface', p.muted, p.surface, text], ['muted/surface-2', p.muted, p.surface2, text],
    ['muted/bg-2', p.muted, p.bg2, text],
    ['accent-ink/accent', p.accentInk, p.accent, aaa ? 7 : 4.5],
    ['success', p.success, p.successBg, text], ['warn', p.warn, p.warnBg, text],
    ['error', p.error, p.errorBg, text], ['info', p.info, p.infoBg, text],
    ['focus/bg (UI)', p.focus, p.bg, 3], ['focus/surface (UI)', p.focus, p.surface, 3],
  ]
  return pairs.map(([pair, a, b, min]) => {
    const ratio = contrast(a, b)
    return { pair, ratio: Math.round(ratio * 100) / 100, min, ok: ratio >= min }
  })
}

// ---------------- Variables CSS ----------------

export const FONTS: Record<FontSet, { title: string; tracking: string }> = {
  editorial: { title: "'Instrument Serif', 'Newsreader', Georgia, serif", tracking: '-0.035em' },
  moderna: { title: "'Geist Variable', 'Plus Jakarta Sans', sans-serif", tracking: '-0.05em' },
  tecnica: { title: "'Geist Mono Variable', 'SF Mono', monospace", tracking: '-0.06em' },
}

export const CORNERS: Record<Corners, [number, number]> = { suave: [22, 14], redondo: [30, 20], recto: [10, 6] }

/** Texto que mejor se lee sobre un color de acento */
export function inkFor(accent: string) {
  return contrast(BRAND.tinta, accent) >= contrast('#FBF8F1', accent) ? BRAND.tinta : '#FBF8F1'
}

/** Paleta de un tema propio: la base (o la neutra del modo elegido) con otro acento */
export function derivePalette(base: BuiltinId, mode: Mode, accent: string | null): Palette {
  let p: Palette = PALETTES[base]
  if (p.mode !== mode) p = mode === 'light' ? salon : noche
  if (!accent || !isHex(accent)) return p
  const a = normHex(accent)
  return { ...p, accent: a, focus: towards(a, p.text, p.bg, 3), accentInk: inkFor(a) }
}

/** Mezcla `color` con `to` poco a poco hasta llegar al contraste pedido sobre `bg` */
export function towards(color: string, to: string, bg: string, min: number) {
  const a = hexToRgb(color)
  const b = hexToRgb(to)
  for (let k = 0; k <= 1.0001; k += 0.1) {
    const hex = '#' + a.map((v, i) => Math.round(v + (b[i] - v) * k).toString(16).padStart(2, '0')).join('').toUpperCase()
    if (contrast(hex, bg) >= min) return hex
  }
  return to
}

/** Todas las variables de un tema, listas para :root */
export function varsFor(p: Palette, font: FontSet, corners: Corners): Record<string, string> {
  const [r, rs] = CORNERS[corners]
  const f = FONTS[font]
  return {
    'color-scheme': p.mode,
    '--bg': p.bg, '--bg-2': p.bg2, '--surface': p.surface, '--surface-2': p.surface2,
    '--text': p.text, '--text-2': p.text2, '--muted': p.muted,
    '--accent': p.accent, '--accent-ink': p.accentInk, '--focus': p.focus,
    '--success': p.success, '--success-bg': p.successBg, '--warn': p.warn, '--warn-bg': p.warnBg,
    '--error': p.error, '--error-bg': p.errorBg, '--info': p.info, '--info-bg': p.infoBg,
    '--ink-rgb': triplet(p.text), '--bg-rgb': triplet(p.bg), '--surface-rgb': triplet(p.surface),
    '--accent-rgb': triplet(p.accent), '--focus-rgb': triplet(p.focus),
    '--hair-alpha': String(p.hair), '--highlight': p.highlight, '--shadow-rgb': p.shadow,
    '--bd-top': p.bdTop, '--bd-mid': p.bdMid, '--bd-bottom': p.bdBottom,
    '--bd-tint-1': p.bdTint1, '--bd-tint-2': p.bdTint2, '--bd-tint-3': p.bdTint3, '--bd-light': p.bdLight,
    '--grain': String(p.grain),
    '--serif': f.title, '--title-tracking': f.tracking,
    '--radius': `${r}px`, '--radius-sm': `${rs}px`,
  }
}
