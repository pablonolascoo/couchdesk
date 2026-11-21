// Datos de Multitarea y su unión con la parte nativa (src-tauri/src/multitask.rs).
// Archivo pequeño aparte: va dentro del arranque (escucha el botón Xbox/PS desde el principio)
// mientras la pantalla (Multitask.tsx) se carga aparte como las demás.
import { useEffect, useSyncExternalStore } from 'react'
import { useNavigate } from 'react-router-dom'
import { call, isDesktopApp } from '../native'
import { t } from '../i18n'
import { lockNav, unlockNav, type PadButton } from '../nav/padConfig'

export interface Card {
  hwnd: number
  pid: number
  kind: 'app' | 'navegador' | 'nube'
  title: string
  app: string
  exe: string
  icon: string | null
  thumb: string | null
  memory: number
  minimized: boolean
  hung: boolean
}
export interface MemStatus {
  totalMb: number
  usedMb: number
  lastFreedMb: number | null
  auto: boolean
  eco: boolean
  ecoCount: number
  boosted: string | null
}
export interface Snapshot { cards: Card[]; fromOutside: boolean; memory: MemStatus }
export interface Freed { freedMb: number; trimmedMb: number; processes: number }

export const MB = 1024 * 1024
export const MOCK_MEM: MemStatus = { totalMb: 16304, usedMb: 9420, lastFreedMb: null, auto: true, eco: true, ecoCount: 3, boosted: 'hades2.exe' }
export const MOCK_FREED: Freed = { freedMb: 412, trimmedMb: 640, processes: 23 }
const MOCK: Snapshot = {
  fromOutside: true,
  memory: MOCK_MEM,
  cards: [
    { hwnd: 1001, pid: 8812, kind: 'app', title: 'Hades II', app: 'Hades II', exe: 'Hades2.exe', icon: null, thumb: null, memory: 3_120 * MB, minimized: false, hung: false },
    { hwnd: 1002, pid: 4410, kind: 'navegador', title: 'YouTube', app: 'Navegador', exe: '', icon: null, thumb: null, memory: 0, minimized: false, hung: false },
    { hwnd: 1003, pid: 2290, kind: 'app', title: 'Spotify Premium', app: 'Spotify', exe: 'Spotify.exe', icon: null, thumb: null, memory: 412 * MB, minimized: false, hung: false },
    { hwnd: 1004, pid: 7710, kind: 'app', title: '#general - Discord', app: 'Discord', exe: 'Discord.exe', icon: null, thumb: null, memory: 286 * MB, minimized: true, hung: false },
    { hwnd: 1005, pid: 9120, kind: 'app', title: 'Notas.txt - Bloc de notas', app: 'Bloc de notas', exe: 'notepad.exe', icon: null, thumb: null, memory: 38 * MB, minimized: false, hung: true },
  ],
}

// Lo último que mandó la parte nativa (vive fuera de la pantalla: llega antes de que se pinte)
let snapshot: Snapshot | null = null
let snapshotAt = 0
const listeners = new Set<() => void>()

export const getSnapshot = () => snapshot
export const snapshotAge = () => Date.now() - snapshotAt
export function setSnapshot(next: Snapshot | null) {
  snapshot = next
  snapshotAt = Date.now()
  listeners.forEach((l) => l())
}
const subscribe = (fn: () => void) => { listeners.add(fn); return () => { listeners.delete(fn) } }
export const useSnapshot = () => useSyncExternalStore(subscribe, getSnapshot)

export const refresh = () => call<Snapshot>('multitask_list', {}, MOCK).then(setSnapshot)
export function patchCards(cards: Card[]) {
  if (snapshot) setSnapshot({ ...snapshot, cards })
}
export function dropCard(hwnd: number) {
  if (snapshot) setSnapshot({ ...snapshot, cards: snapshot.cards.filter((c) => c.hwnd !== hwnd) })
}

/** Errores de la parte nativa: clave de texto (multitask.*) o el mensaje tal cual */
export function errorText(e: unknown) {
  const s = String(e)
  return s.startsWith('multitask.') ? t(s) : s
}

// Botón central del mando (16): lo atiende la parte nativa (corto = Multitarea, largo = Inicio).
// Con mandos de Xbox y PlayStation, la interfaz no lo usa para ir al Inicio por su cuenta.
const NATIVE_GUIDE = /xinput|xbox|054c|dualsense|dualshock|wireless controller/i

/** Une la interfaz con Multitarea. Una vez, en la ventana principal. */
export function useMultitaskLink() {
  const navigate = useNavigate()
  useEffect(() => {
    const onButton = (e: Event) => {
      const b = (e as CustomEvent<PadButton>).detail
      if (b.button !== 16 || !isDesktopApp || !NATIVE_GUIDE.test(b.id)) return
      if (b.pressed) lockNav('guide')
      else unlockNav('guide')
    }
    window.addEventListener('padbutton', onButton)
    let off: (() => void) | undefined
    let gone = false
    if (isDesktopApp) {
      import('@tauri-apps/api/event').then(({ listen }) =>
        Promise.all([
          listen<Snapshot>('multitask-open', ({ payload }) => {
            setSnapshot(payload)
            if (window.location.pathname !== '/multitarea') navigate('/multitarea')
          }),
          listen<Card[]>('multitask-cards', ({ payload }) => patchCards(payload)),
          listen('multitask-back', () => window.dispatchEvent(new Event('multitask-back'))),
        ]).then((u) => {
          off = () => u.forEach((f) => f())
          if (gone) off()
        }),
      )
    }
    return () => { gone = true; window.removeEventListener('padbutton', onButton); off?.() }
  }, [navigate])
}
