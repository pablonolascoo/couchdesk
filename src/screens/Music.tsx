import { useEffect, useState } from 'react'
import { Pause, Play, SkipBack, SkipForward, ArrowSquareOut, Globe } from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { Stepper } from '../components/Stepper'
import { Cover, useNowPlaying, type Track } from '../components/NowPlaying'
import { BrandLogo } from '../ui/brands'
import { call } from '../native'
import { useT } from '../i18n'
import './Music.css'

interface AppVolume { level: number; scope: 'app' | 'system'; app: string | null }

const SPOTIFY_WEB = 'https://open.spotify.com/'
const APPLE_MUSIC_WEB = 'https://music.apple.com/es/'

const clock = (s: number) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`

/** Barra de progreso que avanza sola entre una consulta y la siguiente */
function Progress({ track }: { track: Track }) {
  const [now, setNow] = useState(() => Date.now())
  const [base, setBase] = useState({ pos: track.position ?? 0, at: Date.now() })
  useEffect(() => setBase({ pos: track.position ?? 0, at: Date.now() }), [track.position, track.title])
  useEffect(() => {
    if (!track.playing) return
    const id = setInterval(() => setNow(Date.now()), 500)
    return () => clearInterval(id)
  }, [track.playing])
  if (track.duration == null || track.position == null) return null
  const pos = Math.min(track.duration, base.pos + (track.playing ? Math.max(0, now - base.at) / 1000 : 0))
  return (
    <div className="mus-progress">
      <div className="bar" role="progressbar" aria-valuenow={Math.round(pos)} aria-valuemin={0} aria-valuemax={Math.round(track.duration)}>
        <div className="bar-fill" style={{ width: `${(pos / track.duration) * 100}%` }} />
      </div>
      <span className="mono">{clock(pos)}</span>
      <span className="mono">{clock(track.duration)}</span>
    </div>
  )
}

export function Music() {
  const { track, error, control } = useNowPlaying()
  const t = useT()
  const [vol, setVol] = useState<AppVolume | null>(null)
  const [notice, setNotice] = useState<{ kind: 'info' | 'error'; text: string } | null>(null)

  // Volumen de la app que suena (se relee al cambiar de app)
  useEffect(() => {
    call<AppVolume>('media_volume_get', {}, { level: 70, scope: 'app', app: 'Spotify' }).then(setVol).catch(() => {})
  }, [track?.app])

  // Botones extra del mando: LB/RB cambian de canción y Menú pausa
  useEffect(() => {
    const onPage = (e: Event) => control((e as CustomEvent<number>).detail < 0 ? 'previous' : 'next')
    const onMenu = () => control('toggle')
    window.addEventListener('padpage', onPage)
    window.addEventListener('padmenu', onMenu)
    return () => { window.removeEventListener('padpage', onPage); window.removeEventListener('padmenu', onMenu) }
  }, [control])

  const setLevel = (level: number) => {
    setVol((v) => (v ? { ...v, level } : v))
    call<AppVolume>('media_volume_set', { level }, { level, scope: 'app', app: 'Spotify' })
      .then(setVol)
      .catch((e) => setNotice({ kind: 'error', text: String(e) }))
  }

  const openWeb = (url: string, name: string) => {
    setNotice({ kind: 'info', text: t('music.opening', { name }) })
    call('browser_open', { url }).catch((e) => setNotice({ kind: 'error', text: String(e) }))
  }

  // App de escritorio si está instalada; si no, el reproductor web
  const openSpotify = () => {
    call('media_open_spotify')
      .then(() => setNotice({ kind: 'info', text: t('music.openingSpotify') }))
      .catch(() => {
        setNotice({ kind: 'info', text: t('music.noSpotify') })
        call('browser_open', { url: SPOTIFY_WEB }).catch((e) => setNotice({ kind: 'error', text: String(e) }))
      })
  }

  const openButtons = (withDefault: boolean) => (
    <div className="mus-open">
      <button className="option" data-nav="mus-spotify" data-nav-default={withDefault ? '' : undefined} onClick={openSpotify}>
        <span className="mus-open-label"><BrandLogo name="spotify" size={22} /> {t('music.openSpotify')}</span>
        <span className="mus-open-hint">{t('music.openSpotifyHint')}</span>
      </button>
      <button className="option" data-nav="mus-spotify-web" onClick={() => openWeb(SPOTIFY_WEB, 'Spotify')}>
        <span className="mus-open-label"><Globe size={22} /> {t('music.openSpotifyWeb')}</span>
        <ArrowSquareOut size={18} />
      </button>
      <button className="option" data-nav="mus-apple" onClick={() => openWeb(APPLE_MUSIC_WEB, 'Apple Music')}>
        <span className="mus-open-label"><BrandLogo name="applemusic" size={22} /> {t('music.openApple')}</span>
        <ArrowSquareOut size={18} />
      </button>
    </div>
  )

  const notices = (
    <>
      {notice && <p className={`notice ${notice.kind}`}>{notice.text}</p>}
      {error && <p className="notice error">{error}</p>}
    </>
  )

  if (track === undefined) {
    return <Page title={t('music.title')}><p className="hero-meta">{t('music.loading')}</p></Page>
  }

  if (track === null) {
    return (
      <Page title={t('music.title')}>
        <div className="empty-state mus-empty">
          <span className="empty-icon"><BrandLogo name="spotify" size={28} /></span>
          <h2>{t('music.empty.title')}</h2>
          <p>{t('music.empty.text')}</p>
        </div>
        {openButtons(true)}
        {notices}
      </Page>
    )
  }

  const volLabel = vol?.scope === 'system' ? t('music.volumePc') : t('music.volumeApp', { app: vol?.app ?? track.app })

  return (
    <Page title={t('music.title')} hints={[['A', t('music.hint.select')], ['☰', t('music.hint.pause')], ['B', t('common.back')]]}>
      <section className="mus-now">
        <Cover track={track} size={280} className="mus-cover" />
        <div className="mus-info">
          <span className="tag mus-app">{track.app}</span>
          <h2 className="mus-title">{track.title}</h2>
          <p className="mus-artist">{[track.artist, track.album].filter(Boolean).join(' · ')}</p>
          <Progress track={track} />
          <div className="mus-controls">
            <button className="mus-btn" data-nav="mus-prev" onClick={() => control('previous')} disabled={!track.canPrev} aria-label={t('music.previous')}>
              <SkipBack size={22} weight="fill" />
            </button>
            <button className="mus-btn mus-main" data-nav="mus-play" data-nav-default="" onClick={() => control('toggle')} aria-label={track.playing ? t('music.pause') : t('music.play')}>
              {track.playing ? <Pause size={28} weight="fill" /> : <Play size={28} weight="fill" />}
            </button>
            <button className="mus-btn" data-nav="mus-next" onClick={() => control('next')} disabled={!track.canNext} aria-label={t('music.next')}>
              <SkipForward size={22} weight="fill" />
            </button>
          </div>
        </div>
      </section>

      <Stepper id="mus-vol" label={volLabel} value={vol?.level ?? null} onChange={setLevel} />
      {vol?.scope === 'system' && <p className="hero-meta mus-note">{t('music.systemVolume')}</p>}

      {openButtons(false)}
      {notices}
    </Page>
  )
}
