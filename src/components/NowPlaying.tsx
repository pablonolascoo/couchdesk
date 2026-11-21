import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { convertFileSrc } from '@tauri-apps/api/core'
import { Pause, Play } from '@phosphor-icons/react'
import { call, isDesktopApp } from '../native'
import { BrandLogo } from '../ui/brands'
import { useT } from '../i18n'
import './NowPlaying.css'

/** Lo que suena en el PC (Spotify o cualquier reproductor) */
export interface Track {
  app: string
  appId: string
  title: string
  artist: string
  album: string
  playing: boolean
  position: number | null
  duration: number | null
  canNext: boolean
  canPrev: boolean
  cover: string | null
}

// En el navegador (sin la app de escritorio) se ve una canción de ejemplo
const mockTrack: Track = {
  app: 'Spotify', appId: 'Spotify.exe', title: 'Columbia', artist: 'Quevedo', album: 'Columbia',
  playing: true, position: 62, duration: 186, canNext: true, canPrev: true, cover: null,
}
let mockState = { ...mockTrack, at: Date.now() }

function mockNow(): Track {
  const pos = (mockState.position ?? 0) + (mockState.playing ? (Date.now() - mockState.at) / 1000 : 0)
  return { ...mockState, position: Math.min(pos, mockState.duration ?? pos) }
}

/** Pregunta qué suena cada 1,5 s mientras la pantalla está visible */
export function useNowPlaying() {
  const [track, setTrack] = useState<Track | null | undefined>(undefined)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(() => {
    if (document.hidden) return
    call<Track | null>('media_now_playing', {}, mockNow()).then((t) => { setTrack(t); setError(null) }).catch((e) => setError(String(e)))
  }, [])

  useEffect(() => {
    refresh()
    const id = setInterval(refresh, 1500)
    document.addEventListener('visibilitychange', refresh)
    return () => { clearInterval(id); document.removeEventListener('visibilitychange', refresh) }
  }, [refresh])

  /** play, pause, toggle, next, previous */
  const control = useCallback((action: string) => {
    if (action === 'toggle') setTrack((t) => (t ? { ...t, playing: !t.playing } : t)) // respuesta inmediata
    if (!isDesktopApp && action === 'toggle') mockState = { ...mockNow(), playing: !mockState.playing, at: Date.now() }
    call('media_control', { action })
      .then(() => setTimeout(refresh, 350))
      .catch((e) => { setError(String(e)); refresh() })
  }, [refresh])

  return { track, error, control, refresh }
}

export const coverUrl = (t: Track | null | undefined) => (t?.cover && isDesktopApp ? convertFileSrc(t.cover) : null)

/** Carátula o, si no hay, el logo de Spotify */
export function Cover({ track, size, className }: { track: Track; size: number; className?: string }) {
  const src = coverUrl(track)
  return (
    <span className={`np-cover ${className ?? ''}`} style={{ width: size, height: size }}>
      {src ? <img src={src} alt="" draggable={false} /> : <BrandLogo name="spotify" size={Math.round(size * 0.46)} color="#fff" />}
    </span>
  )
}

/** Píldora compacta para el Inicio: carátula, canción y pausa. Se oculta si no suena nada */
export function NowPlaying() {
  const navigate = useNavigate()
  const { track, control } = useNowPlaying()
  const t = useT()
  if (!track) return null
  return (
    <div className="np-pill reveal" aria-label={t('music.nowPlaying')}>
      <button className="np-open" data-nav="np-open" onClick={() => navigate('/musica')}>
        <Cover track={track} size={40} />
        <span className="np-text">
          <span className="np-title">{track.title}</span>
          <span className="np-artist">{track.artist || track.app}</span>
        </span>
      </button>
      <button className="np-toggle" data-nav="np-toggle" onClick={() => control('toggle')} aria-label={track.playing ? t('music.pause') : t('music.play')}>
        {track.playing ? <Pause size={18} weight="fill" /> : <Play size={18} weight="fill" />}
      </button>
    </div>
  )
}
