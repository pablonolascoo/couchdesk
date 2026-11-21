import type { CSSProperties } from 'react'
import { coverSrc, iconSrc, storeLabel, type Game } from '../data/games'
import { GameController, Desktop } from '@phosphor-icons/react'
import { BrandLogo, type Brand } from '../ui/brands'
import type { Store } from '../data/games'
import './GameCard.css'

const storeBrand: Partial<Record<Store, Brand>> = { steam: 'steam', epic: 'epic', gog: 'gog' }

/** Logo grande de la tienda, de fondo, cuando el juego no trae portada */
export function StoreArt({ store }: { store: Store }) {
  const brand = storeBrand[store]
  return (
    <span className="card-cover-art" aria-hidden>
      {brand
        ? <BrandLogo name={brand} size={150} color="currentColor" />
        : store === 'xbox' ? <GameController size={160} weight="fill" /> : <Desktop size={150} weight="duotone" />}
    </span>
  )
}

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
            <StoreArt store={game.store} />
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
