import { useEffect, useState } from 'react'
import { useNavigate, useParams, Navigate } from 'react-router-dom'
import {
  Waves, Image, FilmStrip, YoutubeLogo, Cube, CaretRight, CheckCircle, ArrowCounterClockwise,
  FolderOpen, LinkSimple, Warning,
} from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { Stepper } from '../components/Stepper'
import { AnimatedLayers } from '../components/Backdrop'
import { call } from '../native'
import {
  animatedStyles, fileName, isHttpUrl, kindLabel, mediaSrc, parseYouTube, restoreAnimated, setBackdrop,
  useBackdrop, useBackdropError, youtubeThumb, type BackdropChoice, type MediaKind,
} from '../data/backdrop'
import { t as tr, useT } from '../i18n'
import './BackgroundSettings.css'

// Cada tipo de fondo tiene su propia dirección: /ajustes/fondo/imagen, /video, /youtube, /modelo
const slugs = { imagen: 'image', video: 'video', youtube: 'youtube', modelo: 'model' } as const
type Slug = keyof typeof slugs

// Nombre en background.kind.<tipo>, pista en background.hint.<tipo>, título en background.titles.<tipo>
const sources: { slug: Slug; icon: typeof Image }[] = [
  { slug: 'imagen', icon: Image },
  { slug: 'video', icon: FilmStrip },
  { slug: 'youtube', icon: YoutubeLogo },
  { slug: 'modelo', icon: Cube },
]

function choiceName(c: BackdropChoice) {
  if (c.kind === 'animated') return animatedStyles.find((s) => s.id === c.style)?.label ?? ''
  if (c.kind === 'youtube') return c.playlistId ? tr('background.ytList') : tr('background.ytVideo')
  return c.name
}

/** Miniatura del fondo actual */
function Preview({ choice, failed }: { choice: BackdropChoice; failed: boolean }) {
  const t = useT()
  const thumb = choice.kind === 'youtube' ? youtubeThumb(choice.videoId) : null
  return (
    <figure className="bg-preview">
      <div className="bg-preview-frame">
        {choice.kind === 'animated' || failed ? (
          <AnimatedLayers style={choice.kind === 'animated' ? choice.style : 'plano'} />
        ) : choice.kind === 'image' ? (
          <img src={mediaSrc(choice.src)} alt="" />
        ) : choice.kind === 'video' ? (
          <video src={`${mediaSrc(choice.src)}#t=1`} muted preload="metadata" />
        ) : thumb ? (
          <img src={thumb} alt="" />
        ) : (
          <span className="bg-preview-icon">{choice.kind === 'model' ? <Cube size={56} weight="light" /> : <YoutubeLogo size={56} weight="light" />}</span>
        )}
      </div>
      <figcaption>
        <span className="label">{t('background.current')}</span>
        <strong>{kindLabel(choice.kind)}</strong>
        <span className="bg-preview-name">{choiceName(choice)}</span>
      </figcaption>
    </figure>
  )
}

// ---------------- /ajustes/fondo ----------------

export function BackgroundSettings() {
  const navigate = useNavigate()
  const t = useT()
  const { choice, dim } = useBackdrop()
  const error = useBackdropError()
  const current = (slug: Slug | 'animado') => (slug === 'animado' ? choice.kind === 'animated' : choice.kind === slugs[slug])

  return (
    <Page title={t('background.title')}>
      <Preview choice={choice} failed={!!error} />
      {error && (
        <p className="notice error bg-error">
          <Warning size={18} weight="light" /> {error} {t('background.errorFallback')}
        </p>
      )}

      <div className="menu-list bg-menu">
        <button className="menu-item" data-nav="bg-animado" data-nav-default="" onClick={() => navigate('/ajustes/fondo/animado')}>
          <Waves size={24} weight="light" />
          <span className="row-label">{kindLabel('animated')}<span className="row-sub">{t('background.animated.sub')}</span></span>
          {current('animado') && <span className="option-check"><CheckCircle size={18} weight="fill" /> {t('background.inUse')}</span>}
          <CaretRight size={18} weight="light" />
        </button>
        {sources.map(({ slug, icon: Icon }) => (
          <button key={slug} className="menu-item" data-nav={`bg-${slug}`} onClick={() => navigate(`/ajustes/fondo/${slug}`)}>
            <Icon size={24} weight="light" />
            <span className="row-label">{kindLabel(slugs[slug])}<span className="row-sub">{t(`background.hint.${slugs[slug]}`)}</span></span>
            {current(slug) && <span className="option-check"><CheckCircle size={18} weight="fill" /> {t('background.inUse')}</span>}
            <CaretRight size={18} weight="light" />
          </button>
        ))}
      </div>

      <Stepper id="bg-dim" label={t('background.dim')} value={dim} onChange={(v) => setBackdrop({ dim: Math.min(90, v) })} />
      <p className="row-sub bg-dim-hint">{t('background.dim.hint')}</p>

      <button className="option" data-nav="bg-restore" onClick={restoreAnimated} disabled={choice.kind === 'animated' && !error}>
        <span className="bg-inline"><ArrowCounterClockwise size={20} weight="light" /> {t('background.restore')}</span>
      </button>
    </Page>
  )
}

// ---------------- /ajustes/fondo/animado ----------------

export function BackgroundStyles() {
  const t = useT()
  const { choice } = useBackdrop()
  const active = choice.kind === 'animated' ? choice.style : null
  return (
    <Page title={t('background.animated.title')}>
      <div className="bg-styles">
        {animatedStyles.map((s, i) => (
          <button
            key={s.id}
            className="bg-style"
            data-nav={`bg-style-${s.id}`}
            data-nav-default={i === 0 ? '' : undefined}
            onClick={() => setBackdrop({ choice: { kind: 'animated', style: s.id } })}
          >
            <span className="bg-style-frame"><AnimatedLayers style={s.id} /></span>
            <span className="bg-style-text">
              <span className="row-label">{s.label}</span>
              <span className="row-sub">{s.hint}</span>
            </span>
            {active === s.id && <span className="option-check"><CheckCircle size={18} weight="fill" /> {t('background.inUse')}</span>}
          </button>
        ))}
      </div>
      <p className="row-sub bg-dim-hint">{t('background.animated.hint')}</p>
    </Page>
  )
}

// ---------------- /ajustes/fondo/:tipo ----------------

// Ayuda del enlace en background.link.<tipo>

export function BackgroundSource() {
  const navigate = useNavigate()
  const t = useT()
  const { kind } = useParams()
  const [text, setText] = useState('')
  const [error, setError] = useState<string | null>(null)
  if (!kind || !(kind in slugs)) return <Navigate to="/ajustes/fondo" replace />
  const slug = kind as Slug
  const isYouTube = slug === 'youtube'
  const linkHelp = t(`background.link.${slugs[slug]}`)

  const applyLink = () => {
    const url = text.trim()
    if (isYouTube) {
      const yt = parseYouTube(url)
      if (!yt) return setError(t('background.error.notYouTube'))
      setBackdrop({ choice: { kind: 'youtube', url, ...yt } })
      return navigate(-1)
    }
    if (!isHttpUrl(url)) return setError(t('background.error.notUrl'))
    setBackdrop({ choice: { kind: slugs[slug] as MediaKind, src: url, name: fileName(url) } })
    navigate(-1)
  }

  return (
    <Page title={t(`background.titles.${slugs[slug]}`)}>
      {!isYouTube && (
        <button className="menu-item" data-nav="bg-pc" data-nav-default="" onClick={() => navigate(`/ajustes/fondo/${slug}/pc`)}>
          <FolderOpen size={24} weight="light" />
          <span className="row-label">{t('background.pc')}<span className="row-sub">{t('background.pc.sub')}</span></span>
          <CaretRight size={18} weight="light" />
        </button>
      )}

      <h3 className="label bg-link-label">{isYouTube ? t('background.link.ytLabel') : t('background.link.orLabel')}</h3>
      <form className="bg-link" onSubmit={(e) => { e.preventDefault(); applyLink() }}>
        <LinkSimple size={22} weight="light" aria-hidden />
        <input
          className="bg-link-input"
          data-nav="bg-link-input"
          data-nav-default={isYouTube ? '' : undefined}
          value={text}
          onChange={(e) => { setText(e.target.value); setError(null) }}
          placeholder={isYouTube ? 'https://www.youtube.com/watch?v=…' : 'https://…'}
          spellCheck={false}
          inputMode="url"
          aria-label={linkHelp}
        />
      </form>
      <p className="row-sub bg-dim-hint">{linkHelp}</p>
      <button className="btn-primary bg-use" data-nav="bg-use-link" onClick={applyLink} disabled={!text.trim()}>
        {t('background.useLink')}
      </button>
      {error && <p className="notice error">{error}</p>}
      {isYouTube && <p className="notice info">{t('background.ytInfo')}</p>}
    </Page>
  )
}

// ---------------- /ajustes/fondo/:tipo/pc ----------------

interface MediaFile { name: string; path: string; size: number; modified: number; folder: string }

const mockFiles: Record<MediaKind, MediaFile[]> = {
  image: [
    { name: 'Atardecer.jpg', path: 'C:\\Users\\CASA\\Pictures\\Atardecer.jpg', size: 2_400_000, modified: 1_763_000_000, folder: 'Imágenes' },
    { name: 'Montaña.png', path: 'C:\\Users\\CASA\\Downloads\\Montaña.png', size: 5_100_000, modified: 1_762_000_000, folder: 'Descargas' },
    { name: 'Playa.webp', path: 'C:\\Users\\CASA\\Desktop\\Playa.webp', size: 900_000, modified: 1_761_000_000, folder: 'Escritorio' },
  ],
  video: [
    { name: 'Olas.mp4', path: 'C:\\Users\\CASA\\Videos\\Olas.mp4', size: 48_000_000, modified: 1_763_000_000, folder: 'Vídeos' },
  ],
  model: [
    { name: 'Coche.glb', path: 'C:\\Users\\CASA\\Downloads\\Coche.glb', size: 3_200_000, modified: 1_763_000_000, folder: 'Descargas' },
  ],
}

const sizeText = (b: number) => (b >= 1e9 ? `${(b / 1e9).toFixed(1)} GB` : b >= 1e6 ? `${Math.round(b / 1e6)} MB` : `${Math.max(1, Math.round(b / 1e3))} KB`)

export function BackgroundFiles() {
  const navigate = useNavigate()
  const t = useT()
  const { kind } = useParams()
  const slug = kind as Slug
  const media = slugs[slug] as MediaKind | undefined
  const [files, setFiles] = useState<MediaFile[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState<string | null>(null)

  useEffect(() => {
    if (!media || slug === 'youtube') return
    call<MediaFile[]>('backdrop_list_media', { kind: media }, mockFiles[media])
      .then(setFiles)
      .catch((e) => { setFiles([]); setError(tr('background.files.readError', { error: String(e) })) })
  }, [media, slug])

  if (!media || slug === 'youtube') return <Navigate to="/ajustes/fondo" replace />

  const choose = async (f: MediaFile) => {
    if (busy) return
    setBusy(f.path); setError(null)
    try {
      const src = await call<string>('backdrop_import', { path: f.path }, f.path)
      setBackdrop({ choice: { kind: media, src, name: f.name } })
      navigate(-2) // vuelve a Ajustes → Fondo
    } catch (e) {
      setError(String(e))
      setBusy(null)
    }
  }

  const Icon = media === 'video' ? FilmStrip : Cube

  return (
    <Page title={t(`background.files.title.${media}`)}>
      {files === null && <p className="hero-meta">{t(`background.files.searching.${media}`)}</p>}
      {files?.length === 0 && !error && (
        <p className="empty">{t(`background.files.empty.${media}`)}</p>
      )}
      {error && <p className="notice error">{error}</p>}
      {!!files?.length && (
        <div className="bg-files">
          {files.map((f, i) => (
            <button
              key={f.path}
              className="bg-file"
              data-nav={`bg-file-${f.path}`}
              data-nav-default={i === 0 ? '' : undefined}
              onClick={() => choose(f)}
              disabled={!!busy && busy !== f.path}
            >
              <span className="bg-file-thumb">
                {media === 'image' ? (
                  <img src={mediaSrc(f.path)} alt="" loading="lazy" decoding="async" onError={(e) => { e.currentTarget.style.display = 'none' }} />
                ) : null}
                <Icon size={40} weight="light" />
              </span>
              <span className="bg-file-name">{busy === f.path ? t('background.files.copying') : f.name}</span>
              <span className="bg-file-meta">{f.folder} · {sizeText(f.size)}</span>
            </button>
          ))}
        </div>
      )}
    </Page>
  )
}
