import type { CSSProperties } from 'react'
import { coverSrc, iconSrc, storeLabel, type Game } from '../data/games'
import './GameCard.css'

interface Props {
  game: Game
  index: number
  onOpen: () => void
  isDefault?: boolean
  /** Solo portada (fila de Inicio): el nombre sale en grande arriba */
  compact?: boolean
  /** Adónde va el foco al pulsar abajo */
  navDown?: string
}

export function GameCard({ game, index, onOpen, isDefault, compact, navDown }: Props) {
  const src = coverSrc(game)
  const icon = iconSrc(game)
  return (
    <button
      className={`card reveal${compact ? ' card-compact' : ''}`}
      style={{ '--i': index } as CSSProperties}
      data-nav={`game-${game.id}`}
      data-nav-default={isDefault ? '' : undefined}
      data-nav-down={navDown}
      onClick={onOpen}
      aria-label={game.title}
    >
      <span className="card-shell">
        {src ? (
          <img className="card-cover" src={src} alt="" draggable={false} />
        ) : (
          <span className={`card-cover ${game.store}`}>
            <span className="card-cover-store">{storeLabel[game.store]}</span>
            {icon && <img className="card-cover-icon" src={icon} alt="" draggable={false} />}
            <span className="card-cover-title">{game.title}</span>
          </span>
        )}
        <span className="card-sheen" aria-hidden />
      </span>
      {!compact && (
        <span className="card-meta">
          <span className="card-name">{game.title}</span>
          <span className="card-sub">{storeLabel[game.store]}</span>
        </span>
      )}
    </button>
  )
}
