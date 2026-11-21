import { useEffect, useRef, useState, type CSSProperties } from 'react'
import {
  choiceKey, errors, isHttpUrl, mediaSrc, reportBackdropError, useBackdrop, useBackdropFailure, useBackdropRevision,
  youtubeEmbedUrl, type AnimatedStyle, type BackdropChoice,
} from '../data/backdrop'
import type { ModelScene } from './backdrop/scene3d'
import { useT } from '../i18n'
import './Backdrop.css'

/** Pausado si la ventana está oculta o hay un juego / el navegador delante */
function usePaused() {
  const check = () => document.hidden || !document.hasFocus()
  const [paused, setPaused] = useState(check)
  useEffect(() => {
    const update = () => setPaused(check())
    document.addEventListener('visibilitychange', update)
    window.addEventListener('focus', update)
    window.addEventListener('blur', update)
    return () => {
      document.removeEventListener('visibilitychange', update)
      window.removeEventListener('focus', update)
      window.removeEventListener('blur', update)
    }
  }, [])
  return paused
}

// Puntos de luz con posición, tamaño y ritmo fijos (mismo dibujo siempre)
const dots = Array.from({ length: 16 }, (_, i) => {
  const r = (n: number) => ((Math.sin(i * 12.9898 + n * 78.233) * 43758.5453) % 1 + 1) % 1
  return {
    '--x': `${(i * 6.3 + r(1) * 5) % 100}%`,
    '--s': (0.7 + r(2) * 1.6).toFixed(2),
    '--d': `${Math.round(28 + r(3) * 26)}s`,
    '--delay': `${-Math.round(r(4) * 50)}s`,
    '--y': Math.round(10 + r(5) * 80),
  } as CSSProperties
})

/** Fondo animado solo con CSS: capas que se mueven con transform (las mueve la tarjeta gráfica) */
export function AnimatedLayers({ style }: { style: AnimatedStyle }) {
  return (
    <div className={`bd-anim bd-${style}`} aria-hidden>
      {style === 'ondas' && (<><i className="bd-wave w1" /><i className="bd-wave w2" /><i className="bd-wave w3" /></>)}
      {style === 'aurora' && (<><i className="bd-blob b1" /><i className="bd-blob b2" /><i className="bd-blob b3" /></>)}
      {style === 'particulas' && (<><i className="bd-blob b1" />{dots.map((s, i) => <i key={i} className="bd-dot" style={s} />)}</>)}
    </div>
  )
}

/** Fondo de toda la consola, detrás de todas las pantallas */
export function Backdrop() {
  const { choice, dim, style } = useBackdrop()
  const failure = useBackdropFailure()
  const revision = useBackdropRevision()
  const paused = usePaused()
  const key = choiceKey(choice)
  // Si el fondo propio falla, se ve el animado de siempre
  const media = choice.kind !== 'animated' && failure?.key !== key
  const animated = choice.kind === 'animated' ? choice.style : style
  const fail = (message: string) => reportBackdropError(choice, message)

  return (
    <div className={`backdrop${paused ? ' is-paused' : ''}`} aria-hidden>
      <div className="bd-layer" key={media ? `${key}-${revision}` : `anim-${animated}`}>
        {/* Mientras carga un fondo propio se ve el color plano de base */}
        <AnimatedLayers style={media ? 'plano' : animated} />
        {media && choice.kind === 'image' && (
          <img className="bd-media" src={mediaSrc(choice.src)} alt="" onError={() => fail(errors.image)} />
        )}
        {media && choice.kind === 'video' && <VideoLayer src={choice.src} paused={paused} onError={() => fail(errors.video)} />}
        {media && choice.kind === 'youtube' && <YouTubeLayer choice={choice} paused={paused} onError={() => fail(errors.youtube)} />}
        {media && choice.kind === 'model' && <ModelLayer src={choice.src} paused={paused} onError={() => fail(errors.model)} />}
      </div>
      {media && <div className="bd-veil" style={{ opacity: dim / 100 }} />}
    </div>
  )
}

function VideoLayer({ src, paused, onError }: { src: string; paused: boolean; onError: () => void }) {
  const ref = useRef<HTMLVideoElement>(null)
  useEffect(() => {
    const v = ref.current
    if (!v) return
    if (paused) v.pause()
    else v.play().catch(() => { /* el navegador puede negarse a reproducir; se reintenta al volver */ })
  }, [paused])
  return <video ref={ref} className="bd-media" src={mediaSrc(src)} muted loop autoPlay playsInline preload="auto" onError={onError} />
}

const YT_ORIGIN = 'https://www.youtube-nocookie.com'

function YouTubeLayer({ choice, paused, onError }: { choice: Extract<BackdropChoice, { kind: 'youtube' }>; paused: boolean; onError: () => void }) {
  const ref = useRef<HTMLIFrameElement>(null)
  const t = useT()
  const errRef = useRef(onError)
  errRef.current = onError
  const send = (func: string, args: unknown[] = []) =>
    ref.current?.contentWindow?.postMessage(JSON.stringify({ event: 'command', func, args }), YT_ORIGIN)

  useEffect(() => {
    // El reproductor avisa si el vídeo no existe o no se puede insertar
    const onMessage = (e: MessageEvent) => {
      if (e.origin !== YT_ORIGIN || e.source !== ref.current?.contentWindow) return
      try {
        const data = typeof e.data === 'string' ? JSON.parse(e.data) : e.data
        if (data?.event === 'onError') errRef.current()
      } catch { /* mensaje ajeno */ }
    }
    window.addEventListener('message', onMessage)
    return () => window.removeEventListener('message', onMessage)
  }, [])
  useEffect(() => { send(paused ? 'pauseVideo' : 'playVideo') }, [paused])

  const onLoad = () => {
    ref.current?.contentWindow?.postMessage(JSON.stringify({ event: 'listening', id: 'fondo', channel: 'widget' }), YT_ORIGIN)
    send('addEventListener', ['onError'])
    if (paused) send('pauseVideo')
  }

  return (
    <iframe
      ref={ref}
      className="bd-youtube"
      src={youtubeEmbedUrl(choice)}
      title={t('background.titles.youtube')}
      tabIndex={-1}
      allow="autoplay; encrypted-media"
      referrerPolicy="strict-origin-when-cross-origin"
      onLoad={onLoad}
    />
  )
}

function ModelLayer({ src, paused, onError }: { src: string; paused: boolean; onError: () => void }) {
  const box = useRef<HTMLDivElement>(null)
  const scene = useRef<ModelScene | null>(null)
  const pausedRef = useRef(paused)
  pausedRef.current = paused
  const errRef = useRef(onError)
  errRef.current = onError

  useEffect(() => {
    let cancelled = false
    // three.js se descarga solo ahora, en un archivo aparte
    import('./backdrop/scene3d')
      .then(({ mountModel }) => {
        if (cancelled || !box.current) return
        const local = !isHttpUrl(src)
        const folder = src.replace(/[\\/][^\\/]*$/, '')
        scene.current = mountModel(box.current, {
          url: mediaSrc(src),
          resolveRelative: local ? (rel) => mediaSrc(`${folder}\\${rel.replace(/\//g, '\\')}`) : undefined,
          onError: () => errRef.current(),
        })
        scene.current.setPaused(pausedRef.current)
      })
      .catch(() => errRef.current())
    return () => { cancelled = true; scene.current?.dispose(); scene.current = null }
  }, [src])

  useEffect(() => { scene.current?.setPaused(paused) }, [paused])

  return <div ref={box} className="bd-model" />
}
