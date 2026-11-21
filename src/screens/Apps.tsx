import type { CSSProperties } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { AppWindow, ArrowLeft, ArrowUpRight, GameController, Wrench } from '@phosphor-icons/react'
import { TopBar } from '../components/TopBar'
import { HintBar } from '../components/HintBar'
import { useApps, iconSrc, launchGame, markAsGame, type App } from '../data/games'
import { useT } from '../i18n'
import './Apps.css'

function AppIcon({ app, size }: { app: App; size: number }) {
  const src = iconSrc(app)
  return src
    ? <img className="app-icon" src={src} alt="" width={size} height={size} draggable={false} />
    : <span className="app-icon app-icon-empty" style={{ width: size, height: size }}>{app.system ? <Wrench size={size * 0.5} /> : <AppWindow size={size * 0.5} />}</span>
}

function Grid({ apps, offset, first }: { apps: App[]; offset: number; first?: boolean }) {
  const navigate = useNavigate()
  return (
    <div className="apps-grid">
      {apps.map((a, i) => (
        <button
          key={a.id}
          className="app-tile reveal"
          style={{ '--i': Math.min(offset + i, 14) } as CSSProperties}
          data-nav={a.id}
          data-nav-default={first && i === 0 ? '' : undefined}
          onClick={() => navigate(`/aplicaciones/${encodeURIComponent(a.id)}`)}
        >
          <AppIcon app={a} size={44} />
          <span className="app-name">{a.name}</span>
        </button>
      ))}
    </div>
  )
}

// Todo lo demás que hay instalado en el PC
export function Apps() {
  const t = useT()
  const apps = useApps()
  const mine = apps?.filter((a) => !a.system) ?? []
  const tools = apps?.filter((a) => a.system) ?? []
  return (
    <div className="screen">
      <TopBar title={t('apps.title')} />
      <p className="library-count">
        {apps === null ? t('apps.searching') : t('apps.count', { n: apps.length })}
      </p>
      <div className="apps-scroll">
        {mine.length > 0 && <Grid apps={mine} offset={0} first />}
        {tools.length > 0 && (
          <>
            <h3 className="label apps-section">{t('apps.windowsTools')}</h3>
            <Grid apps={tools} offset={mine.length} first={mine.length === 0} />
          </>
        )}
      </div>
      <HintBar hints={[['A', t('shell.hint.view')], ['B', t('common.back')]]} />
    </div>
  )
}

export function AppDetail() {
  const t = useT()
  const { id = '' } = useParams()
  const navigate = useNavigate()
  const apps = useApps()
  const app = apps?.find((a) => a.id === id)

  if (!app) {
    return (
      <div className="screen">
        <TopBar title={t('apps.app')} />
        <section className="app-detail">
          <p className="hero-meta">{apps === null ? t('shell.loading') : t('apps.gone')}</p>
          <div className="hero-actions">
            <button className="pill" data-nav="back" data-nav-default="" onClick={() => navigate(-1)}>
              {t('shell.goBack')} <span className="pill-icon"><ArrowLeft size={16} /></span>
            </button>
          </div>
        </section>
      </div>
    )
  }

  return (
    <div className="screen">
      <TopBar title={app.system ? t('apps.windowsTool') : t('apps.app')} />
      <section className="app-detail">
        <div className="app-detail-icon reveal"><AppIcon app={app} size={96} /></div>
        <span className="eyebrow reveal" style={{ '--i': 1 } as CSSProperties}>{app.system ? 'Windows' : t('apps.app')}</span>
        <h2 className="hero-title reveal" style={{ '--i': 2 } as CSSProperties}>{app.name}</h2>
        <div className="hero-actions reveal" style={{ '--i': 3 } as CSSProperties}>
          <button className="pill pill-dark" data-nav="open" data-nav-default="" onClick={() => launchGame(app.id)}>
            {t('common.open')}
            <span className="pill-icon"><ArrowUpRight size={16} /></span>
          </button>
          <button className="pill" data-nav="game" onClick={() => markAsGame(app.id, true).then(() => navigate(`/juego/${encodeURIComponent(app.id)}`, { replace: true }))}>
            {t('apps.markGame')}
            <span className="pill-icon"><GameController size={16} /></span>
          </button>
          <button className="pill" data-nav="back" onClick={() => navigate(-1)}>
            {t('shell.goBack')}
            <span className="pill-icon"><ArrowLeft size={16} /></span>
          </button>
        </div>
        <p className="hero-meta reveal" style={{ '--i': 4 } as CSSProperties}>
          {t('apps.markGame.note')}
        </p>
      </section>
      <HintBar hints={[['A', t('shell.hint.select')], ['B', t('common.back')]]} />
    </div>
  )
}
