import { useSyncExternalStore } from 'react'
import { t } from '../i18n'

export interface Entry { name: string; path: string; isDir: boolean; size?: number | null; modified?: number | null }
export interface Place { label: string; path: string; kind: 'carpeta' | 'usb' | 'disco' }

// Datos de ejemplo para el navegador
export const mockPlaces = (): Place[] => [
  { label: t('files.place.usb', { drive: 'E:' }), path: 'E:\\', kind: 'usb' },
  { label: t('files.place.received'), path: 'C:\\Users\\CASA\\Recibidos', kind: 'carpeta' },
  { label: t('files.place.downloads'), path: 'C:\\Users\\CASA\\Downloads', kind: 'carpeta' },
  { label: t('files.place.games'), path: 'C:\\Users\\CASA\\Juegos', kind: 'carpeta' },
  { label: t('files.place.desktop'), path: 'C:\\Users\\CASA\\Desktop', kind: 'carpeta' },
  { label: t('files.place.disk', { drive: 'C:' }), path: 'C:\\', kind: 'disco' },
  { label: t('files.place.disk', { drive: 'D:' }), path: 'D:\\', kind: 'disco' },
]
export const mockEntries = (dir: string): Entry[] => [
  { name: 'Partidas guardadas', path: `${dir}\\Partidas guardadas`, isDir: true },
  { name: 'Recibidos', path: `${dir}\\Recibidos`, isDir: true },
  { name: 'scph5501.bin', path: `${dir}\\scph5501.bin`, isDir: false, size: 524_288, modified: 1_794_900_000 },
  { name: 'Crash Bandicoot (Europe).chd', path: `${dir}\\Crash Bandicoot (Europe).chd`, isDir: false, size: 412_000_000, modified: 1_794_900_000 },
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
