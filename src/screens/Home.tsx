import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Play, ArrowUpRight, Waves } from '@phosphor-icons/react'
import { TopBar } from '../components/TopBar'
import { HintBar } from '../components/HintBar'
import { GameCard } from '../components/GameCard'
import { sections, type Section } from '../data/mock'
import { Ambient } from '../components/Ambient'
import { useGames, storeLabel, lastPlayedText, sizeText, type Game } from '../data/games'
import { usePlay } from '../data/usePlay'
import { BrandLogo } from '../ui/brands'
import { NowPlaying } from '../components/NowPlaying'
import { t, useT } from '../i18n'
import './Home.css'

// Juego destacado recordado al volver atrás
let lastSelectedId: string | undefined

// Modo ambiente, junto a Música
function dockSections(): Section[] {
  const all = sections()
  const ambienceSection: Section = { path: '/ambiente', label: t('home.section.ambience'), hint: t('home.section.ambience.hint'), icon: Waves, tone: '#2f6f6a' }
  const dockAt = all.findIndex((s) => s.path === '/recibir')
  return dockAt < 0 ? [...all, ambienceSection] : [...all.slice(0, dockAt), ambienceSection, ...all.slice(dockAt)]
}

export function Home() {
  const t = useT()
  const dock = dockSections()
  const navigate = useNavigate()
  const games = useGames()
  const recent = (games ?? []).slice(0, 10)
  const [selectedId, setSelectedId] = useState(lastSelectedId)
  const selected: Game | undefined = recent.find((g) => g.id === selectedId) ?? recent[0]
  const railRef = useRef<HTMLDivElement>(null)
  const { play, launching } = usePlay()

  // El destacado sigue al juego que tiene el foco del mando
  useEffect(() => {
    const rail = railRef.current
    const onNavFocus = (e: Event) => {
      const id = (e.target as HTMLElement).dataset.nav?.replace('game-', '')
      if (id) { lastSelectedId = id; setSelectedId(id) }
    }
    rail?.addEventListener('navfocus', onNavFocus)
    return () => rail?.removeEventListener('navfocus', onNavFocus)
  }, [games])

  const size = selected ? sizeText(selected.sizeBytes) : null

  return (
    <div className="screen home">
      <Ambient game={selected} />
      <TopBar title={t('home.title')} />

      <section className="rail" aria-label={t('home.rail')}>
        <div className="rail-row" ref={railRef}>
          {recent.map((g, i) => (
            <GameCard key={g.id} game={g} index={i} compact navDown="play" isDefault={i === 0} onOpen={() => navigate(`/juego/${g.id}`)} />
          ))}
          {games && recent.length === 0 && <p className="empty">{t('home.rail.empty')}</p>}
        </div>
      </section>

      <section className="hero" aria-live="polite">
        {selected ? (
          <div className="hero-inner" key={selected.id}>
            <span className={`tag ${selected.store}`}>{storeLabel[selected.store]}</span>
            <h2 className="hero-title">
              {selected.title.split(' ').map((w, i) => (
                <span className="word" key={i}><span style={{ animationDelay: `${i * 70}ms` }}>{w}</span></span>
              ))}
            </h2>
            <p className="hero-meta">
              <span>{lastPlayedText(selected.lastPlayed)}</span>
              {size && <><span className="dot" />{size}</>}
            </p>
            <div className="hero-actions">
              <button className="pill pill-dark" data-nav="play" data-nav-up={`game-${selected.id}`} onClick={() => play(selected)}>
                {launching === selected.id ? t('shell.opening') : t('home.play')}
                <span className="pill-icon"><Play size={14} weight="fill" /></span>
              </button>
              <button className="pill" data-nav="info" data-nav-up={`game-${selected.id}`} onClick={() => navigate(`/juego/${selected.id}`)}>
                {t('home.details')}
                <span className="pill-icon"><ArrowUpRight size={16} /></span>
              </button>
            </div>
          </div>
        ) : (
          <div className="hero-inner">
            <span className="eyebrow">{t('home.eyebrow')}</span>
            <h2 className="hero-title">{games === null ? t('home.searching') : t('home.noGames')}</h2>
            <p className="hero-meta">{t('home.noGames.text')}</p>
          </div>
        )}
        <div className="home-np"><NowPlaying /></div>
      </section>

      <nav className="dock" aria-label={t('home.sections')}>
        {dock.map(({ path, label, hint, icon: Icon, tone, brand }, i) => (
          <button
            key={path}
            className="tile reveal"
            style={{ ['--i' as string]: i + 4, ['--tone' as string]: tone }}
            data-nav={`sec-${path}`}
            data-nav-up={selected ? 'play' : undefined}
            data-nav-default={games && recent.length === 0 && i === 0 ? '' : undefined}
            onClick={() => navigate(path)}
          >
            {/* Dibujo grande de fondo, propio de cada sección */}
            <span className="tile-art" aria-hidden>
              {brand ? <BrandLogo name={brand} size={92} color="currentColor" /> : <Icon size={96} weight="duotone" />}
            </span>
            <span className="tile-icon">{brand ? <BrandLogo name={brand} size={20} color="currentColor" /> : <Icon size={22} weight="regular" />}</span>
            <span className="tile-label">{label}</span>
            <span className="tile-hint">{hint}</span>
          </button>
        ))}
      </nav>

      <HintBar hints={[['A', t('shell.hint.select')]]} />
    </div>
  )
}
