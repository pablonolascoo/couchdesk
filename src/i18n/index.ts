// Idiomas de CouchDesk.
// - Cada idioma es una carpeta en ./locales con archivos JSON planos (clave → texto), uno por zona
//   de la app (common.json, settings.json…). Las claves llevan delante su zona: "settings.title".
//   Para traducir, copia la carpeta es/, cambia los textos y añade el idioma a LANGUAGES.
//   Guía: TRANSLATING.md en el repositorio.
// - El idioma sale del sistema la primera vez; en Ajustes → Idioma se puede elegir otro.
// - Textos con variables: "Hola, {nombre}" → t('clave', { nombre: 'Ana' }).
// - Plurales sencillos: "{n} mando|{n} mandos" → elige por n === 1.
import { useSyncExternalStore } from 'react'
import { call, isDesktopApp } from '../native'

export interface Language { code: string; name: string; english: string }

/** Idiomas disponibles (el nombre, en su propio idioma) */
export const LANGUAGES: Language[] = [
  { code: 'es', name: 'Español', english: 'Spanish' },
  { code: 'en', name: 'English', english: 'English' },
  { code: 'pt-BR', name: 'Português (Brasil)', english: 'Portuguese (Brazil)' },
  { code: 'fr', name: 'Français', english: 'French' },
  { code: 'de', name: 'Deutsch', english: 'German' },
  { code: 'it', name: 'Italiano', english: 'Italian' },
]

type Dict = Record<string, string>
const merge = (mods: { default: Dict }[]) => Object.assign({}, ...mods.map((m) => m.default)) as Dict
// El español va siempre dentro (es la base); los demás se cargan solo cuando hacen falta
const es = merge(Object.values(import.meta.glob<{ default: Dict }>('./locales/es/*.json', { eager: true })))
const loaders = import.meta.glob<{ default: Dict }>('./locales/*/*.json')
const dicts: Record<string, Dict> = { es }

const KEY = 'couchdesk.lang'
const listeners = new Set<() => void>()
let current = 'es'

function saved(): string | null {
  try { return localStorage.getItem(KEY) } catch { return null }
}

/** El mejor idioma disponible para una lista como navigator.languages */
export function pickLanguage(wanted: readonly string[]): string {
  const codes = LANGUAGES.map((l) => l.code)
  for (const w of wanted) {
    const exact = codes.find((c) => c.toLowerCase() === w.toLowerCase())
    if (exact) return exact
    const base = w.split('-')[0].toLowerCase()
    const near = codes.find((c) => c.split('-')[0].toLowerCase() === base)
    if (near) return near
  }
  return 'en'
}

async function load(code: string) {
  if (dicts[code]) return
  const files = Object.entries(loaders).filter(([path]) => path.startsWith(`./locales/${code}/`))
  if (files.length) dicts[code] = merge(await Promise.all(files.map(([, load]) => load())))
}

/** Cambia el idioma. `null` = el del sistema */
export async function setLanguage(code: string | null) {
  try { if (code) localStorage.setItem(KEY, code); else localStorage.removeItem(KEY) } catch { /* sin almacenamiento */ }
  const next = code ?? pickLanguage(navigator.languages ?? [navigator.language])
  await load(next)
  current = next
  document.documentElement.lang = next
  // La parte nativa (errores, avisos, Claude) habla el mismo idioma
  if (isDesktopApp) call('set_locale', { code: next }).catch(() => { /* versión antigua sin el comando */ })
  listeners.forEach((l) => l())
}

export const getLanguage = () => current
export const usesSystemLanguage = () => saved() === null

/** Llamar una vez antes de pintar la app */
export async function initI18n() {
  await setLanguage(saved())
}

function format(text: string, vars?: Record<string, string | number>) {
  if (!vars) return text
  if (text.includes('|') && typeof vars.n === 'number') {
    const [one, many] = text.split('|')
    text = vars.n === 1 ? one : many
  }
  return text.replace(/\{(\w+)\}/g, (_, k) => (k in vars ? String(vars[k]) : `{${k}}`))
}

/** Texto traducido. Si falta en el idioma actual, usa el español; si tampoco está, la clave */
export function t(key: string, vars?: Record<string, string | number>): string {
  const text = dicts[current]?.[key] ?? dicts.es[key] ?? key
  return format(text, vars)
}

/** Avisa cuando cambia el idioma (para código fuera de React). Devuelve la función para dejar de escuchar */
export const subscribe = (fn: () => void) => { listeners.add(fn); return () => { listeners.delete(fn) } }

/** En componentes: const t = useT() → se vuelve a pintar al cambiar de idioma */
export function useT() {
  useSyncExternalStore(subscribe, getLanguage)
  return t
}

/** Formatos de fecha, hora y números del idioma actual */
export const fmt = {
  time: (d: Date) => d.toLocaleTimeString(current, { hour: '2-digit', minute: '2-digit' }),
  date: (d: Date) => d.toLocaleDateString(current, { weekday: 'long', day: 'numeric', month: 'long' }),
  number: (n: number) => n.toLocaleString(current),
  /** 21 nov 2026 */
  dateShort: (d: Date) => d.toLocaleDateString(current, { day: 'numeric', month: 'short', year: 'numeric' }),
  /** 21 nov 2026, 14:05 */
  dateTime: (d: Date) => d.toLocaleString(current, { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' }),
  /** "hace 3 minutos", "ayer"… */
  relative: (d: Date) => {
    const s = (d.getTime() - Date.now()) / 1000
    const rtf = new Intl.RelativeTimeFormat(current, { numeric: 'auto' })
    const abs = Math.abs(s)
    if (abs < 60) return rtf.format(Math.round(s), 'second')
    if (abs < 3600) return rtf.format(Math.round(s / 60), 'minute')
    if (abs < 86400) return rtf.format(Math.round(s / 3600), 'hour')
    if (abs < 86400 * 30) return rtf.format(Math.round(s / 86400), 'day')
    if (abs < 86400 * 365) return rtf.format(Math.round(s / 86400 / 30), 'month')
    return rtf.format(Math.round(s / 86400 / 365), 'year')
  },
  /** Lista natural: "a, b y c" */
  list: (items: string[]) => new Intl.ListFormat(current, { style: 'long', type: 'conjunction' }).format(items),
  /** 1,5 MB */
  bytes: (n: number) => {
    const units = ['B', 'KB', 'MB', 'GB', 'TB']
    let i = 0
    while (n >= 1024 && i < units.length - 1) { n /= 1024; i++ }
    return `${n.toLocaleString(current, { maximumFractionDigits: i ? 1 : 0 })} ${units[i]}`
  },
}
