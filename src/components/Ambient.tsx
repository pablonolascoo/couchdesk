import { useEffect, useState } from 'react'
import { heroSrc, storeTint, type Game } from '../data/games'
import './Ambient.css'

let seq = 0

/** Fondo de ambiente: la imagen del juego (o su color) entra fundida sobre la anterior */
export function Ambient({ game }: { game?: Game }) {
  const [layers, setLayers] = useState<{ key: string; src: string | null; tint: string }[]>([])
  useEffect(() => {
    if (!game) return
    const layer = { key: `${game.id}-${++seq}`, src: heroSrc(game), tint: storeTint[game.store] }
    setLayers((l) => [...l.slice(-1), layer]) // como mucho dos capas a la vez
  }, [game?.id]) // eslint-disable-line react-hooks/exhaustive-deps
  return (
    <div className="ambient" aria-hidden>
      {layers.map((l) => (
        <div key={l.key} className="ambient-layer">
          {l.src ? (
            <img src={l.src} alt="" />
          ) : (
            <span className="ambient-orb" style={{ background: `radial-gradient(closest-side, ${l.tint}, transparent)` }} />
          )}
        </div>
      ))}
      <div className="ambient-fade" />
    </div>
  )
}
