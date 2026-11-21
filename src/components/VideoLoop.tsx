import { useEffect, useRef, useState } from 'react'
import { useT } from '../i18n'
import './VideoLoop.css'

interface Props {
  /** MP4/WebM directo, o un vídeo o lista de YouTube */
  src: string
  poster?: string
  muted?: boolean
  /** false = en pausa (también se pausa solo si la ventana se oculta) */
  playing?: boolean
  onError?: () => void
  className?: string
  /** Velocidad (1 = normal, 0,5 = cámara lenta). En YouTube se redondea a 0,25/0,5/0,75/1… */
  rate?: number
}

const YT_RATES = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2]
const ytRate = (r: number) => YT_RATES.reduce((a, b) => (Math.abs(b - r) < Math.abs(a - r) ? b : a))

/** Dirección para insertar un vídeo o lista de YouTube en bucle y sin controles; null si no es YouTube */
export function youtubeEmbed(url: string): string | null {
  let u: URL
  try { u = new URL(url) } catch { return null }
  const host = u.hostname.replace(/^www\.|^m\./, '')
  if (!['youtube.com', 'youtu.be', 'youtube-nocookie.com', 'music.youtube.com'].includes(host)) return null
  const list = u.searchParams.get('list')
  let id = host === 'youtu.be' ? u.pathname.slice(1) : u.searchParams.get('v')
  if (!id) id = u.pathname.match(/\/(?:embed|shorts|live)\/([\w-]{6,})/)?.[1] ?? null
  const common = 'autoplay=1&mute=1&controls=0&rel=0&iv_load_policy=3&disablekb=1&playsinline=1&modestbranding=1&enablejsapi=1&loop=1'
  const origin = `&origin=${encodeURIComponent(window.location.origin)}`
  if (id) return `https://www.youtube-nocookie.com/embed/${id}?${common}&playlist=${list ?? id}${origin}`
  if (list) return `https://www.youtube-nocookie.com/embed/videoseries?list=${list}&${common}${origin}`
  return null
}

/** Vídeo de fondo en bucle, sin sonido salvo que se pida, que cubre todo su hueco */
export function VideoLoop({ src, poster, muted = true, playing = true, onError, className, rate = 1 }: Props) {
  const embed = youtubeEmbed(src)
  const t = useT()
  const videoRef = useRef<HTMLVideoElement>(null)
  const frameRef = useRef<HTMLIFrameElement>(null)
  const [ready, setReady] = useState(false)
  const [hidden, setHidden] = useState(document.hidden)
  const errorRef = useRef(onError)
  errorRef.current = onError
  const active = playing && !hidden

  useEffect(() => setReady(false), [src])

  // Ventana oculta o minimizada: nada se reproduce
  useEffect(() => {
    const on = () => setHidden(document.hidden)
    document.addEventListener('visibilitychange', on)
    return () => document.removeEventListener('visibilitychange', on)
  }, [])

  // ---- MP4 ----
  useEffect(() => {
    const v = videoRef.current
    if (!v) return
    v.muted = muted
    if (active) v.play().catch(() => { /* el navegador puede pedir un gesto: sigue en pausa */ })
    else v.pause()
  }, [active, muted, src])

  // Velocidad: también la "por defecto", para que no vuelva a 1 al cargar o repetir
  const applyRate = () => {
    const v = videoRef.current
    if (!v) return
    v.defaultPlaybackRate = rate
    v.playbackRate = rate
  }
  useEffect(applyRate, [rate, src]) // eslint-disable-line react-hooks/exhaustive-deps

  // ---- YouTube: órdenes por postMessage (API del reproductor insertado) ----
  const command = (func: string, args: unknown[] = []) =>
    frameRef.current?.contentWindow?.postMessage(JSON.stringify({ event: 'command', func, args }), '*')

  useEffect(() => {
    if (!embed) return
    const onMessage = (e: MessageEvent) => {
      if (e.source !== frameRef.current?.contentWindow || typeof e.data !== 'string') return
      let data: { event?: string; info?: { playerState?: number } | number }
      try { data = JSON.parse(e.data) } catch { return }
      if (data.event === 'onError') errorRef.current?.()
      // 1 = reproduciendo
      if (data.event === 'infoDelivery' && typeof data.info === 'object' && data.info?.playerState === 1) setReady(true)
      if (data.event === 'onStateChange' && data.info === 1) setReady(true)
    }
    window.addEventListener('message', onMessage)
    return () => window.removeEventListener('message', onMessage)
  }, [embed])

  useEffect(() => {
    if (!embed) return
    command(muted ? 'mute' : 'unMute')
    if (!muted) command('setVolume', [100])
    command(active ? 'playVideo' : 'pauseVideo')
  }, [embed, muted, active])

  useEffect(() => {
    if (embed && ready) command('setPlaybackRate', [ytRate(rate)])
  }, [embed, rate, ready]) // eslint-disable-line react-hooks/exhaustive-deps

  const onFrameLoad = () => {
    // Pide al reproductor que avise de su estado y de los errores (p. ej. el dueño no deja insertarlo)
    frameRef.current?.contentWindow?.postMessage(JSON.stringify({ event: 'listening', id: 'loop', channel: 'widget' }), '*')
    command(muted ? 'mute' : 'unMute')
    command('setPlaybackRate', [ytRate(rate)])
    if (!active) command('pauseVideo')
  }

  return (
    <div className={`vl ${ready ? 'vl-ready' : ''} ${className ?? ''}`}>
      {poster && <img className="vl-poster" src={poster} alt="" draggable={false} />}
      {embed ? (
        <iframe
          ref={frameRef}
          className="vl-frame"
          src={embed}
          title={t('ambience.video')}
          allow="autoplay; encrypted-media; picture-in-picture"
          referrerPolicy="strict-origin-when-cross-origin"
          tabIndex={-1}
          onLoad={onFrameLoad}
        />
      ) : (
        <video
          ref={videoRef}
          className="vl-video"
          src={src}
          muted={muted}
          autoPlay={active}
          loop
          playsInline
          preload="auto"
          disablePictureInPicture
          onLoadedMetadata={applyRate}
          onPlaying={() => { applyRate(); setReady(true) }}
          onError={() => onError?.()}
        />
      )}
    </div>
  )
}
