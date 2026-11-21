import { useNavigate } from 'react-router-dom'
import { GameController, ArrowUpRight } from '@phosphor-icons/react'
import { TopBar } from '../components/TopBar'
import { HintBar } from '../components/HintBar'
import { GameCard } from '../components/GameCard'
import { useGames } from '../data/games'
import { useT } from '../i18n'
import './Library.css'

export function Library() {
  const t = useT()
  const navigate = useNavigate()
  const games = useGames()

  return (
    <div className="screen">
      <TopBar title={t('library.title')} />
      <p className="library-count">
        {games === null ? t('library.searching') : t('library.count', { n: games.length })}
      </p>
      <div className="library-grid">
        {games?.map((g, i) => (
          <GameCard key={g.id} game={g} index={i} isDefault={i === 0} onOpen={() => navigate(`/juego/${g.id}`)} />
        ))}
        {games?.length === 0 && (
          <div className="empty-state">
            <span className="empty-icon"><GameController size={26} /></span>
            <h2>{t('library.empty.title')}</h2>
            <p>{t('library.empty.text')}</p>
            <button className="pill pill-dark" data-nav="emus" data-nav-default="" onClick={() => navigate('/emuladores')}>
              {t('library.empty.emulators')}
              <span className="pill-icon"><ArrowUpRight size={16} /></span>
            </button>
          </div>
        )}
      </div>
      <HintBar hints={[['A', t('common.open')], ['B', t('common.back')]]} />
    </div>
  )
}
