// Perfiles de mando: qué botón hace cada acción de la consola y cómo responde el stick.
//
// Guardado en localStorage, clave "couchdesk.pads.v1":
//   { version: 1, profiles: Profile[] (solo los del usuario), defaultId: string,
//     perPad: { [gamepad.id]: profileId } }
// Los perfiles de serie (Estándar, Nintendo/Japón, Zurdo) viven en el código y no se guardan.
//
// Eventos de ventana que emite useSpatialNav (para que cualquier pantalla los use):
//   'padpage'   detail -1 | +1   LB / RB (o lo que diga el perfil). Solo si la navegación no está en pausa.
//   'padmenu'   sin detail        START sin BACK (START + BACK es el cambio de modo de xTend).
//   'padkey'    detail PadKey     cada acción con el perfil ya aplicado (también en pausa; la usa el teclado).
//   'padbutton' detail PadButton  cada botón crudo al pulsarlo y al soltarlo (para reasignar botones).
//   'padinput'  sin detail        hubo algo de mando (para esconder el puntero).
//   'padmode'   detail 0|1|2      START + BACK: mando → ratón → teclado.
//
// Pausa de la navegación: lockNav('motivo') / unlockNav('motivo'). Mientras haya algún motivo,
// useSpatialNav no mueve el foco ni pulsa nada; solo emite 'padkey' y 'padbutton'.

import { t } from '../i18n'

export type Action = 'accept' | 'back' | 'home' | 'menu' | 'pagePrev' | 'pageNext'
export type Speed = 'slow' | 'normal' | 'fast'
export type Zone = 'small' | 'normal' | 'large'

export interface Profile {
  id: string
  name: string
  builtIn?: boolean
  buttons: Record<Action, number>
  swapAB: boolean
  repeat: Speed
  deadzone: Zone
  invertY: boolean
  leftHanded: boolean
}

export interface PadConfig {
  version: 1
  profiles: Profile[]
  defaultId: string
  perPad: Record<string, string>
}

export type PadKey = 'up' | 'down' | 'left' | 'right' | 'x' | 'y' | Action
export interface PadButton { pad: number; id: string; button: number; pressed: boolean }

// El texto se traduce al leerlo (getter), así sigue el idioma actual
const act = (action: Action) => ({ action, get label() { return t(`controllers.action.${action}`) } })
export const ACTIONS: { action: Action; readonly label: string }[] = [
  act('accept'), act('back'), act('home'), act('menu'), act('pagePrev'), act('pageNext'),
]

const STANDARD: Record<Action, number> = { accept: 0, back: 1, home: 16, menu: 9, pagePrev: 4, pageNext: 5 }
const base = { buttons: STANDARD, swapAB: false, repeat: 'normal', deadzone: 'normal', invertY: false, leftHanded: false } as const

export const PRESETS: Profile[] = [
  { ...base, id: 'estandar', get name() { return t('controllers.preset.estandar') }, builtIn: true },
  { ...base, id: 'nintendo', get name() { return t('controllers.preset.nintendo') }, builtIn: true, swapAB: true },
  { ...base, id: 'zurdo', get name() { return t('controllers.preset.zurdo') }, builtIn: true, leftHanded: true },
]

export const EMPTY: PadConfig = { version: 1, profiles: [], defaultId: 'estandar', perPad: {} }

// ---------------- Funciones puras (sin efectos: mismo dato, mismo resultado) ----------------

export const allProfiles = (c: PadConfig): Profile[] => [...PRESETS, ...c.profiles]

export const findProfile = (c: PadConfig, id: string): Profile | undefined => allProfiles(c).find((p) => p.id === id)

/** Perfil que usa un mando (por su gamepad.id); si no tiene, el de por defecto */
export function profileFor(c: PadConfig, padId?: string): Profile {
  const id = (padId && c.perPad[padId]) || c.defaultId
  return findProfile(c, id) ?? PRESETS[0]
}

/** Botones finales: aplica "intercambiar A y B" */
export function effectiveButtons(p: Profile): Record<Action, number> {
  const b = { ...p.buttons }
  if (p.swapAB) [b.accept, b.back] = [b.back, b.accept]
  return b
}

/** Cuánto hay que empujar el stick para que cuente (0..1) */
export const threshold = (z: Zone) => ({ small: 0.4, normal: 0.6, large: 0.8 })[z]

/** Al mantener: espera antes de repetir y ritmo de repetición (ms) */
export const repeatTiming = (s: Speed) => ({ slow: { delay: 520, rate: 220 }, normal: { delay: 350, rate: 120 }, fast: { delay: 240, rate: 60 } })[s]

/** Nombre libre para un perfil nuevo: "Mi perfil", "Mi perfil 2"… */
export function freeName(c: PadConfig, wanted: string): string {
  const taken = new Set(allProfiles(c).map((p) => p.name))
  if (!taken.has(wanted)) return wanted
  let n = 2
  while (taken.has(`${wanted} ${n}`)) n++
  return `${wanted} ${n}`
}

/** Copia un perfil (de serie o propio) como perfil nuevo del usuario */
export function duplicate(c: PadConfig, fromId: string, newId: string, name?: string): PadConfig {
  const from = findProfile(c, fromId) ?? PRESETS[0]
  const copy: Profile = { ...from, buttons: { ...from.buttons }, id: newId, builtIn: undefined, name: freeName(c, name ?? t('controllers.copyName', { name: from.name })) }
  return { ...c, profiles: [...c.profiles, copy] }
}

/** Cambia un perfil del usuario (los de serie no se tocan) */
export function updateProfile(c: PadConfig, id: string, patch: Partial<Omit<Profile, 'id' | 'builtIn'>>): PadConfig {
  return { ...c, profiles: c.profiles.map((p) => (p.id === id ? { ...p, ...patch, buttons: { ...p.buttons, ...patch.buttons } } : p)) }
}

export function setButton(c: PadConfig, id: string, action: Action, button: number): PadConfig {
  const p = c.profiles.find((x) => x.id === id)
  if (!p) return c
  return updateProfile(c, id, { buttons: { ...p.buttons, [action]: button } })
}

/** Borra un perfil del usuario; quien lo usaba pasa al Estándar */
export function removeProfile(c: PadConfig, id: string): PadConfig {
  const perPad = Object.fromEntries(Object.entries(c.perPad).filter(([, v]) => v !== id))
  return { ...c, profiles: c.profiles.filter((p) => p.id !== id), perPad, defaultId: c.defaultId === id ? 'estandar' : c.defaultId }
}

export const setDefault = (c: PadConfig, id: string): PadConfig => ({ ...c, defaultId: id })

/** Perfil de un mando concreto; `null` = usar el de por defecto */
export function assign(c: PadConfig, padId: string, id: string | null): PadConfig {
  const perPad = { ...c.perPad }
  if (id) perPad[padId] = id
  else delete perPad[padId]
  return { ...c, perPad }
}

/** Lee lo guardado aceptando datos viejos o rotos */
export function parseConfig(raw: string | null): PadConfig {
  if (!raw) return EMPTY
  try {
    const d = JSON.parse(raw) as Partial<PadConfig>
    if (d?.version !== 1) return EMPTY
    const profiles = (Array.isArray(d.profiles) ? d.profiles : [])
      .filter((p) => p && typeof p.id === 'string' && typeof p.name === 'string')
      .map((p) => ({ ...PRESETS[0], ...p, builtIn: undefined, buttons: { ...STANDARD, ...p.buttons } }))
    const c: PadConfig = { version: 1, profiles, defaultId: typeof d.defaultId === 'string' ? d.defaultId : 'estandar', perPad: d.perPad && typeof d.perPad === 'object' ? d.perPad : {} }
    if (!findProfile(c, c.defaultId)) c.defaultId = 'estandar'
    return c
  } catch {
    return EMPTY
  }
}

// ---------------- Estado guardado ----------------

const KEY = 'couchdesk.pads.v1'
let config: PadConfig = (() => {
  try { return parseConfig(localStorage.getItem(KEY)) } catch { return EMPTY }
})()
const listeners = new Set<(c: PadConfig) => void>()

export function getConfig(): PadConfig {
  return config
}

export function setConfig(next: PadConfig | ((c: PadConfig) => PadConfig)) {
  config = typeof next === 'function' ? next(config) : next
  try { localStorage.setItem(KEY, JSON.stringify(config)) } catch { /* sin almacenamiento: vale para esta sesión */ }
  listeners.forEach((fn) => fn(config))
}

export function subscribe(fn: (c: PadConfig) => void): () => void {
  listeners.add(fn)
  return () => { listeners.delete(fn) }
}

export const newId = () => `p${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`

// ---------------- Pausa de la navegación ----------------

const locks = new Set<string>()
export const lockNav = (why: string) => { locks.add(why) }
export const unlockNav = (why: string) => { locks.delete(why) }
export const isNavLocked = () => locks.size > 0

/** true mientras useSpatialNav mueve el foco (para distinguirlo de un foco puesto por la pantalla) */
export const navState = { focusing: false }

/** Pide abrir el teclado en pantalla para este campo */
export function requestKeyboard(el: HTMLInputElement | HTMLTextAreaElement) {
  window.dispatchEvent(new CustomEvent('kbopen', { detail: el }))
}

export function isTextField(el: Element | null): el is HTMLInputElement | HTMLTextAreaElement {
  if (el instanceof HTMLTextAreaElement) return !el.readOnly && !el.disabled
  if (!(el instanceof HTMLInputElement) || el.readOnly || el.disabled) return false
  return ['text', 'search', 'url', 'email', 'password', 'tel', 'number', ''].includes(el.type)
}

// ---------------- Nombres de botones ----------------

// Los que empiezan por "@" son palabras: se traducen al pedirlos
const XBOX = ['A', 'B', 'X', 'Y', 'LB', 'RB', 'LT', 'RT', 'View', '@menu', 'L3', 'R3', '@up', '@down', '@left', '@right', 'Xbox']
const PS = ['✕', '○', '□', '△', 'L1', 'R1', 'L2', 'R2', 'Share', 'Options', 'L3', 'R3', '@up', '@down', '@left', '@right', 'PS']

export function buttonName(i: number, ps = false): string {
  const name = (ps ? PS : XBOX)[i]
  if (name === undefined) return t('controllers.btn.number', { n: i + 1 })
  return name.startsWith('@') ? t(`controllers.btn.${name.slice(1)}`) : name
}
