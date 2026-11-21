import { useNavigate, useParams } from 'react-router-dom'
import { ArrowLeft, Play, AppWindow } from '@phosphor-icons/react'
import { TopBar } from '../components/TopBar'
import { HintBar } from '../components/HintBar'
import { Ambient } from '../components/Ambient'
import { useGames, storeLabel, lastPlayedText, sizeText, coverSrc, iconSrc, markAsGame } from '../data/games'
import { usePlay } from '../data/usePlay'
import { StoreArt } from '../components/GameCard'
import '../components/GameCard.css'
import './GameDetail.css'

export function GameDetail() {
  const { id } = useParams()
  const navigate = useNavigate()
  const games = useGames()
  const game = games?.find((g) => g.id === id)
  const { play, launching, error } = usePlay()

  if (!game) {
    return (
      <div className="screen">
        <TopBar title="Juego" />
        <section className="detail">
          <div>
            <p className="hero-meta">{games === null ? 'Cargando…' : 'Este juego ya no está instalado.'}</p>
            <div className="hero-actions">
              <button className="pill" data-nav="back" data-nav-default="" onClick={() => navigate(-1)}>
                Volver <span className="pill-icon"><ArrowLeft size={16} /></span>
              </button>
            </div>
          </div>
        </section>
      </div>
    )
  }

  const src = coverSrc(game)
  const size = sizeText(game.sizeBytes)

  return (
    <div className="screen">
      <Ambient game={game} />
      <TopBar title={storeLabel[game.store]} />
      <section className="detail">
        <div className="detail-shell">
          {src ? (
            <img className="detail-cover" src={src} alt="" />
          ) : (
            <div className={`detail-cover card-cover ${game.store}`}>
              <StoreArt store={game.store} />
              <span className="card-cover-store">{storeLabel[game.store]}</span>
              {iconSrc(game) && <img className="card-cover-icon" src={iconSrc(game)!} alt="" style={{ width: 96, height: 96 }} />}
              <span className="card-cover-title">{game.title}</span>
            </div>
          )}
        </div>
        <div className="detail-info">
          <span className={`tag ${game.store}`}>{storeLabel[game.store]}</span>
          <h2 className="hero-title">
            {game.title.split(' ').map((w, i) => (
              <span className="word" key={i}><span style={{ animationDelay: `${120 + i * 70}ms` }}>{w}</span></span>
            ))}
          </h2>
          <dl className="detail-facts">
            <dt>Última vez</dt><dd>{lastPlayedText(game.lastPlayed)}</dd>
            {size && (<><dt>Tamaño</dt><dd>{size}</dd></>)}
            {game.installDir && (<><dt>Carpeta</dt><dd className="mono">{game.installDir}</dd></>)}
          </dl>
          <div className="hero-actions">
            <button className="pill pill-dark" data-nav="play" data-nav-default="" onClick={() => play(game)}>
              {launching ? 'Abriendo…' : 'Jugar'}
              <span className="pill-icon"><Play size={14} weight="fill" /></span>
            </button>
            <button className="pill" data-nav="back" onClick={() => navigate(-1)}>
              Volver
              <span className="pill-icon"><ArrowLeft size={16} /></span>
            </button>
            <button className="pill" data-nav="notgame" onClick={() => markAsGame(game.id, false).then(() => navigate('/aplicaciones', { replace: true }))}>
              No es un juego
              <span className="pill-icon"><AppWindow size={16} /></span>
            </button>
          </div>
          {error && <p className="detail-error">No se pudo abrir: {error}</p>}
        </div>
      </section>
      <HintBar hints={[['A', 'Seleccionar'], ['B', 'Atrás']]} />
    </div>
  )
}
