import { useSyncExternalStore } from 'react'
import { convertFileSrc } from '@tauri-apps/api/core'
import { isDesktopApp } from '../native'
import { t } from '../i18n'

// Fondo de la consola: qué se ve detrás de todas las pantallas

export type AnimatedStyle = 'ondas' | 'aurora' | 'particulas' | 'plano'
export type MediaKind = 'image' | 'video' | 'model'

export type BackdropChoice =
  | { kind: 'animated'; style: AnimatedStyle }
  | { kind: 'image' | 'video' | 'model'; src: string; name: string }
  | { kind: 'youtube'; url: string; videoId: string | null; playlistId: string | null }

export interface BackdropState {
  choice: BackdropChoice
  /** 0-90: cuánto se cubre el fondo con el color crema para que el texto se lea bien */
  dim: number
  /** Estilo animado que vuelve al restaurar */
  style: AnimatedStyle
}

// Los textos se traducen al leerlos (background.style.<id>, background.kind.<tipo>)
export const animatedStyles: { id: AnimatedStyle; readonly label: string; readonly hint: string }[] =
  (['ondas', 'aurora', 'particulas', 'plano'] as const).map((id) => ({
    id,
    get label() { return t(`background.style.${id}`) },
    get hint() { return t(`background.style.${id}.hint`) },
  }))

export const kindLabel = (kind: BackdropChoice['kind']) => t(`background.kind.${kind}`)

const KEY = 'couchdesk.backdrop.v1'
const DEFAULT: BackdropState = { choice: { kind: 'animated', style: 'ondas' }, dim: 40, style: 'ondas' }

function read(): BackdropState {
  try {
    const raw = localStorage.getItem(KEY)
    if (!raw) return DEFAULT
    const s = JSON.parse(raw) as Partial<BackdropState>
    if (!s.choice || typeof s.choice !== 'object' || !('kind' in s.choice)) return DEFAULT
    return { ...DEFAULT, ...s } as BackdropState
  } catch {
    return DEFAULT
  }
}

let state = read()
/** Fondo que no se pudo cargar (clave de la elección) y por qué */
let failure: { key: string; message: string } | null = null
export const choiceKey = (c: BackdropChoice) => JSON.stringify(c)
/** Sube cada vez que se elige un fondo: obliga a cargarlo de nuevo */
let revision = 0
export function useBackdropRevision() {
  return useSyncExternalStore(subscribe, () => revision)
}
const listeners = new Set<() => void>()
const emit = () => listeners.forEach((l) => l())

function subscribe(fn: () => void) {
  listeners.add(fn)
  return () => listeners.delete(fn)
}

export function getBackdrop() { return state }

export function setBackdrop(patch: Partial<BackdropState>) {
  state = { ...state, ...patch }
  if (patch.choice) { failure = null; revision++ } // elegir otra vez = volver a intentarlo
  if (patch.choice?.kind === 'animated') state.style = patch.choice.style
  try { localStorage.setItem(KEY, JSON.stringify(state)) } catch { /* sin almacenamiento: solo dura esta sesión */ }
  emit()
}

export function restoreAnimated() {
  setBackdrop({ choice: { kind: 'animated', style: state.style } })
}

/** El fondo elegido no se pudo cargar: se avisa en Ajustes y se ve el animado */
export function reportBackdropError(choice: BackdropChoice, message: string) {
  const key = choiceKey(choice)
  if (key !== choiceKey(state.choice) || failure?.key === key) return // ya no es el elegido
  failure = { key, message }
  emit()
}

export function useBackdrop() {
  return useSyncExternalStore(subscribe, () => state)
}
/** Clave del fondo que falló (o null) */
export function useBackdropFailure() {
  return useSyncExternalStore(subscribe, () => failure)
}
export function useBackdropError() {
  return useBackdropFailure()?.message ?? null
}

// ---------------- Enlaces ----------------

export function isHttpUrl(text: string) {
  try {
    const u = new URL(text.trim())
    return u.protocol === 'http:' || u.protocol === 'https:'
  } catch {
    return false
  }
}

const exts: Record<MediaKind, string[]> = {
  image: ['jpg', 'jpeg', 'png', 'webp', 'gif', 'avif', 'bmp'],
  video: ['mp4', 'm4v', 'webm', 'ogv'],
  model: ['glb', 'gltf'],
}

/** Tipo de archivo por su extensión (ruta local o enlace) */
export function mediaKindOf(pathOrUrl: string): MediaKind | null {
  let p = pathOrUrl.trim()
  try { p = new URL(p).pathname } catch { /* ruta local */ }
  const ext = p.split(/[?#]/)[0].split('.').pop()?.toLowerCase() ?? ''
  return (Object.keys(exts) as MediaKind[]).find((k) => exts[k].includes(ext)) ?? null
}

export function fileName(pathOrUrl: string) {
  let p = pathOrUrl.trim()
  try { p = decodeURIComponent(new URL(p).pathname) } catch { /* ruta local */ }
  return p.split(/[\\/]/).filter(Boolean).pop() ?? p
}

const ID = /^[\w-]{11}$/
const LIST = /^[\w-]{10,64}$/

/** Lee un enlace de YouTube: vídeo (watch?v=, youtu.be/, /shorts/, /embed/, /live/) y/o lista (list=) */
export function parseYouTube(text: string): { videoId: string | null; playlistId: string | null } | null {
  let u: URL
  try { u = new URL(text.trim().replace(/^(?!https?:\/\/)/i, 'https://')) } catch { return null }
  const host = u.hostname.replace(/^(www\.|m\.|music\.)/, '')
  let videoId: string | null = null
  if (host === 'youtu.be') {
    videoId = u.pathname.split('/')[1] ?? null
  } else if (host === 'youtube.com' || host === 'youtube-nocookie.com') {
    const [, first, second] = u.pathname.split('/')
    if (first === 'watch') videoId = u.searchParams.get('v')
    else if (['shorts', 'embed', 'live', 'v'].includes(first)) videoId = second ?? null
  } else {
    return null
  }
  if (videoId && !ID.test(videoId)) videoId = null
  let playlistId = u.searchParams.get('list')
  if (playlistId && !LIST.test(playlistId)) playlistId = null
  if (!videoId && !playlistId) return null
  return { videoId, playlistId }
}

/** Dirección del reproductor: sin sonido, en bucle, sin controles */
export function youtubeEmbedUrl({ videoId, playlistId }: { videoId: string | null; playlistId: string | null }) {
  const p = new URLSearchParams({
    autoplay: '1', mute: '1', loop: '1', controls: '0', playsinline: '1',
    disablekb: '1', fs: '0', iv_load_policy: '3', rel: '0', modestbranding: '1', enablejsapi: '1',
  })
  // Para repetir un solo vídeo, YouTube pide que también sea su propia lista
  if (playlistId) { p.set('list', playlistId); p.set('listType', 'playlist') }
  else if (videoId) p.set('playlist', videoId)
  const path = videoId ? `embed/${videoId}` : 'embed/videoseries'
  return `https://www.youtube-nocookie.com/${path}?${p}`
}

export function youtubeThumb(videoId: string | null) {
  return videoId ? `https://i.ytimg.com/vi/${videoId}/hqdefault.jpg` : null
}

/** Ruta local → dirección que puede abrir la app. Los enlaces se quedan igual. */
export function mediaSrc(src: string) {
  if (isHttpUrl(src) || !isDesktopApp) return src
  return convertFileSrc(src)
}

export const errors = {
  get image() { return t('background.error.image') },
  get video() { return t('background.error.video') },
  get model() { return t('background.error.model') },
  get youtube() { return t('background.error.youtube') },
}
