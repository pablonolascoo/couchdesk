import { useSyncExternalStore } from 'react'

export interface Entry { name: string; path: string; isDir: boolean; size?: number | null; modified?: number | null }
export interface Place { label: string; path: string }

// Datos de ejemplo para el navegador
export const mockPlaces: Place[] = [
  { label: 'Escritorio', path: 'C:\\Users\\CASA\\Desktop' },
  { label: 'Descargas', path: 'C:\\Users\\CASA\\Downloads' },
  { label: 'Documentos', path: 'C:\\Users\\CASA\\Documents' },
  { label: 'Disco C:', path: 'C:\\' },
  { label: 'Disco D:', path: 'D:\\' },
]
export const mockEntries = (dir: string): Entry[] => [
  { name: 'Partidas guardadas', path: `${dir}\\Partidas guardadas`, isDir: true },
  { name: 'Recibidos', path: `${dir}\\Recibidos`, isDir: true },
  { name: 'captura-2026.png', path: `${dir}\\captura-2026.png`, isDir: false, size: 2_400_000, modified: 1_794_900_000 },
  { name: 'notas.txt', path: `${dir}\\notas.txt`, isDir: false, size: 1_200, modified: 1_794_000_000 },
]

export const parentOf = (p: string) => {
  const trimmed = p.replace(/\\+$/, '')
  const i = trimmed.lastIndexOf('\\')
  if (i <= 2) return trimmed.length > 2 ? trimmed.slice(0, 3) : null // "C:\"
  return trimmed.slice(0, i)
}
export const nameOf = (p: string) => p.replace(/\\+$/, '').split('\\').pop() || p

// Portapapeles interno de la consola (copiar/cortar → pegar)
type Clip = { items: string[]; cut: boolean } | null
let clip: Clip = null
const listeners = new Set<() => void>()
export const clipboard = {
  set(c: Clip) { clip = c; listeners.forEach((l) => l()) },
  get: () => clip,
  subscribe(l: () => void) { listeners.add(l); return () => { listeners.delete(l) } },
}
export const useClipboard = () => useSyncExternalStore(clipboard.subscribe, clipboard.get)
