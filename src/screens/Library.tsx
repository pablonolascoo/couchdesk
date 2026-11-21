import { useNavigate } from 'react-router-dom'
import { GameController, ArrowUpRight } from '@phosphor-icons/react'
import { TopBar } from '../components/TopBar'
import { HintBar } from '../components/HintBar'
import { GameCard } from '../components/GameCard'
import { useGames } from '../data/games'
import './Library.css'

export function Library() {
  const navigate = useNavigate()
  const games = useGames()

  return (
    <div className="screen">
      <TopBar title="Biblioteca" />
      <p className="library-count">
        {games === null ? 'Buscando juegos…' : `${games.length} ${games.length === 1 ? 'juego instalado' : 'juegos instalados'}`}
      </p>
      <div className="library-grid">
        {games?.map((g, i) => (
          <GameCard key={g.id} game={g} index={i} isDefault={i === 0} onOpen={() => navigate(`/juego/${g.id}`)} />
        ))}
        {games?.length === 0 && (
          <div className="empty-state">
            <span className="empty-icon"><GameController size={26} /></span>
            <h2>Aún no hay juegos</h2>
            <p>Los juegos de Steam, Epic, GOG y Xbox aparecen aquí solos en cuanto los instalas. Mientras, puedes jugar con emuladores.</p>
            <button className="pill pill-dark" data-nav="emus" data-nav-default="" onClick={() => navigate('/emuladores')}>
              Ir a Emuladores
              <span className="pill-icon"><ArrowUpRight size={16} /></span>
            </button>
          </div>
        )}
      </div>
      <HintBar hints={[['A', 'Abrir'], ['B', 'Atrás']]} />
    </div>
  )
}
