// Ajustes de rendimiento de la propia app (los de Windows viven en src-tauri/src/perf.rs).
// Guardado en localStorage "couchdesk.perf.v1": { reduceMotion: boolean }.
// index.html lo lee antes de pintar y pone <html data-reduce-motion> si toca: el CSS lo usa en lugar
// de prefers-reduced-motion (el optimizador de Windows quita las animaciones del sistema, y la app
// debe seguir animada salvo que el usuario lo pida aquí).
import { useSyncExternalStore } from 'react'

export interface PerfSettings { reduceMotion: boolean }

const KEY = 'couchdesk.perf.v1'
const DEFAULTS: PerfSettings = { reduceMotion: false }

export function parsePerf(raw: string | null): PerfSettings {
  try {
    const d = JSON.parse(raw ?? 'null') as Partial<PerfSettings> | null
    return { reduceMotion: d?.reduceMotion === true }
  } catch {
    return DEFAULTS
  }
}

let state: PerfSettings = (() => { try { return parsePerf(localStorage.getItem(KEY)) } catch { return DEFAULTS } })()
const listeners = new Set<() => void>()

function apply(s: PerfSettings) {
  const root = document.documentElement
  if (s.reduceMotion) root.dataset.reduceMotion = ''
  else delete root.dataset.reduceMotion
}

export const getPerf = () => state

export function setPerf(patch: Partial<PerfSettings>) {
  state = { ...state, ...patch }
  try { localStorage.setItem(KEY, JSON.stringify(state)) } catch { /* sin almacenamiento: vale para esta sesión */ }
  apply(state)
  listeners.forEach((l) => l())
}

const subscribe = (fn: () => void) => { listeners.add(fn); return () => { listeners.delete(fn) } }
export const usePerf = () => useSyncExternalStore(subscribe, getPerf)
