// Ajustes del salvapantallas y el tiempo. Los guarda la parte nativa
// (%LOCALAPPDATA%\Consola\salvapantallas.json) para que funcionen con la consola oculta.
// En el navegador se guardan en este navegador, para poder probar las pantallas.

import { call, isDesktopApp } from '../native'
import type { CollectionId } from './ambience'
import { t } from '../i18n'

export type SaverStyle = 'paisajes' | 'bajo-consumo' | 'reloj'

export interface Place { name: string; detail: string; latitude: number; longitude: number }

export interface SaverSettings {
  enabled: boolean
  afterMin: number
  style: SaverStyle
  collections: CollectionId[]
  showWeather: boolean
  locationMode: 'auto' | 'ciudad'
  place: Place | null
  skipFullscreen: boolean
  monitorOffMin: number
  /** Sonido de ambiente bajo los vídeos sin sonido (mar, viento, lluvia, fuego) */
  sound?: boolean
  /** 0-100 */
  soundVolume?: number
}

export interface Weather {
  place: string
  temperature: number
  code: number
  isDay: boolean
  label: string
  /** Nombre del icono de Phosphor */
  icon: string
}

export const defaultSettings: SaverSettings = {
  enabled: true,
  afterMin: 5,
  style: 'paisajes',
  collections: ['costas', 'montanas', 'bosques', 'ciudades', 'nubes', 'agua'],
  showWeather: true,
  locationMode: 'ciudad',
  place: null,
  skipFullscreen: true,
  monitorOffMin: 0,
  sound: false,
  soundVolume: 50,
}

export interface WindowsSaver { enabled: boolean; path: string }

/** ¿CouchDesk es el salvapantallas de Windows? (también fuera de la consola) */
export const windowsSaverStatus = () => call<WindowsSaver>('windows_saver_status', {}, { enabled: false, path: '' })
export const windowsSaverSet = (enabled: boolean, afterMin: number) =>
  call<WindowsSaver>('windows_saver_set', { enabled, afterMin }, { enabled, path: '' })

const KEY = 'consola.salvapantallas'

function readLocal(): SaverSettings {
  try {
    const raw = localStorage.getItem(KEY)
    if (raw) return { ...defaultSettings, ...JSON.parse(raw) }
  } catch { /* sin almacenamiento: valores por defecto */ }
  return defaultSettings
}

export async function loadSettings(): Promise<SaverSettings> {
  const s = await call<SaverSettings>('screensaver_settings_get', {}, readLocal())
  return { ...defaultSettings, ...s }
}

export async function saveSettings(settings: SaverSettings): Promise<SaverSettings> {
  if (!isDesktopApp) {
    try { localStorage.setItem(KEY, JSON.stringify(settings)) } catch { /* nada */ }
  }
  return call<SaverSettings>('screensaver_settings_set', { settings }, settings)
}

const mockPlaces: Place[] = [
  { name: 'Madrid', detail: 'Comunidad de Madrid, España', latitude: 40.4165, longitude: -3.70256 },
  { name: 'Barcelona', detail: 'Cataluña, España', latitude: 41.38879, longitude: 2.15899 },
  { name: 'Valencia', detail: 'Comunidad Valenciana, España', latitude: 39.46975, longitude: -0.37739 },
  { name: 'Sevilla', detail: 'Andalucía, España', latitude: 37.38283, longitude: -5.97317 },
  { name: 'Bilbao', detail: 'País Vasco, España', latitude: 43.26271, longitude: -2.92528 },
]

export function searchPlaces(q: string): Promise<Place[]> {
  const t = q.trim().toLowerCase()
  return call<Place[]>('weather_search', { q }, mockPlaces.filter((p) => t.length > 1 && p.name.toLowerCase().includes(t)))
}

export function weatherNow(): Promise<Weather> {
  const s = readLocal()
  return call<Weather>('weather_now', {}, {
    place: s.locationMode === 'auto' ? 'Madrid' : s.place?.name ?? 'Madrid',
    temperature: 14.2, code: 2, isDay: true, label: t('screensaver.mock.cloudy'), icon: 'CloudSun',
  })
}

export function locateApprox(): Promise<Place> {
  return call<Place>('weather_locate', {}, mockPlaces[0])
}

/** Enseñar el salvapantallas ya (en el navegador se abre como página) */
export async function previewSaver() {
  if (isDesktopApp) await call('screensaver_preview')
  else window.location.assign('/salvapantallas?prueba=1')
}

/** Mientras se ve el modo ambiente, el salvapantallas no lo tapa */
export function inhibitSaver(on: boolean) {
  if (isDesktopApp) call('screensaver_inhibit', { on }).catch(() => {})
}
