import { useEffect, useState } from 'react'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { isDesktopApp } from '../native'

export type Store = 'steam' | 'epic' | 'gog' | 'xbox' | 'pc'

export interface Game {
  id: string
  title: string
  store: Store
  installDir: string
  cover?: string | null
  hero?: string | null
  icon?: string | null
  lastPlayed?: number | null
  sizeBytes?: number | null
}

/** Cualquier programa del menú Inicio que no es un juego */
export interface App {
  id: string
  name: string
  icon?: string | null
  /** Herramienta de Windows (Servicios, Símbolo del sistema…) */
  system: boolean
}

export const storeLabel: Record<Store, string> = { steam: 'Steam', epic: 'Epic', gog: 'GOG', xbox: 'Xbox', pc: 'PC' }

// Juegos de ejemplo para ver el diseño en el navegador (fuera de la app)
const day = 86_400
const now = Math.floor(Date.now() / 1000)
const sample: Game[] = [
  { id: 'steam-1145360', title: 'Hades', store: 'steam', installDir: '', lastPlayed: now - day, sizeBytes: 15e9 },
  { id: 'epic-celeste', title: 'Celeste', store: 'epic', installDir: '', lastPlayed: now - 3 * day, sizeBytes: 1.2e9 },
  { id: 'gog-1207658930', title: 'The Witcher 3', store: 'gog', installDir: '', lastPlayed: now - 9 * day, sizeBytes: 50e9 },
  { id: 'steam-367520', title: 'Hollow Knight', store: 'steam', installDir: '', lastPlayed: now - 30 * day, sizeBytes: 9e9 },
  { id: 'steam-413150', title: 'Stardew Valley', store: 'steam', installDir: '', sizeBytes: 0.6e9 },
  { id: 'epic-fortnite', title: 'Rocket League', store: 'epic', installDir: '', sizeBytes: 20e9 },
]

const sampleApps: App[] = [
  { id: 'app-1', name: 'Google Chrome', system: false },
  { id: 'app-2', name: 'Spotify', system: false },
  { id: 'app-3', name: 'Discord', system: false },
  { id: 'app-4', name: 'Notepad++', system: false },
  { id: 'app-5', name: 'Símbolo del sistema', system: true },
  { id: 'app-6', name: 'Administrador de tareas', system: true },
]

interface Everything { games: Game[]; apps: App[] }
let cache: Everything | null = null
let pending: Promise<Everything> | null = null
const listeners = new Set<(e: Everything) => void>()

/** Juegos y aplicaciones del PC (se buscan una vez y se comparten entre pantallas) */
export function loadEverything(force = false): Promise<Everything> {
  if (cache && !force) return Promise.resolve(cache)
  if (pending && !force) return pending
  pending = (isDesktopApp ? invoke<Everything>('list_everything') : Promise.resolve({ games: sample, apps: sampleApps }))
    .then((e) => { cache = e; listeners.forEach((l) => l(e)); return e })
    .finally(() => { pending = null })
  return pending
}

export const loadGames = (force = false) => loadEverything(force).then((e) => e.games)

function useEverything() {
  const [all, setAll] = useState<Everything | null>(cache)
  useEffect(() => {
    let alive = true
    const on = (e: Everything) => alive && setAll(e)
    listeners.add(on)
    loadEverything().then(on).catch(() => alive && setAll({ games: [], apps: [] }))
    return () => { alive = false; listeners.delete(on) }
  }, [])
  return all
}

export const useGames = () => useEverything()?.games ?? null
export const useApps = () => useEverything()?.apps ?? null

export async function launchGame(id: string) {
  if (isDesktopApp) await invoke('launch_game', { id })
}

/** "Marcar como juego" (true) o "No es un juego" (false); luego se vuelve a leer todo */
export async function markAsGame(id: string, game: boolean) {
  if (isDesktopApp) await invoke('mark_as_game', { id, game })
  else if (cache) {
    // En el navegador de pruebas, solo en memoria
    const g = cache.games.find((x) => x.id === id), a = cache.apps.find((x) => x.id === id)
    if (game && a) cache = { games: [...cache.games, { id, title: a.name, store: 'pc', installDir: '', icon: a.icon }], apps: cache.apps.filter((x) => x.id !== id) }
    if (!game && g) cache = { games: cache.games.filter((x) => x.id !== id), apps: [...cache.apps, { id, name: g.title, icon: g.icon, system: false }] }
    listeners.forEach((l) => l(cache!))
    return
  }
  await loadEverything(true)
}

const fileSrc = (p?: string | null) => (p && isDesktopApp ? convertFileSrc(p) : null)
export const coverSrc = (g: Game) => fileSrc(g.cover)
export const heroSrc = (g: Game) => fileSrc(g.hero)
export const iconSrc = (x: { icon?: string | null }) => fileSrc(x.icon)

// Color de ambiente por tienda cuando no hay imagen del juego
export const storeTint: Record<Store, string> = {
  steam: '#9cc3dd', epic: '#e2c48a', gog: '#a9c4a2', xbox: '#d9a39e', pc: '#d6c5ae',
}

export function lastPlayedText(ts?: number | null) {
  if (!ts) return 'Sin jugar todavía'
  const days = Math.floor((Date.now() / 1000 - ts) / day)
  if (days <= 0) return 'Hoy'
  if (days === 1) return 'Ayer'
  if (days < 7) return `Hace ${days} días`
  if (days < 30) return `Hace ${Math.floor(days / 7)} semana${days < 14 ? '' : 's'}`
  return `Hace ${Math.floor(days / 30)} mes${days < 60 ? '' : 'es'}`
}

export function sizeText(bytes?: number | null) {
  if (!bytes) return null
  return bytes >= 1e9 ? `${(bytes / 1e9).toFixed(1)} GB` : `${Math.round(bytes / 1e6)} MB`
}
