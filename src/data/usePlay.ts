import { useState } from 'react'
import { launchGame, type Game } from './games'

// Lanza un juego y muestra "Abriendo…" unos segundos
export function usePlay() {
  const [launching, setLaunching] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const play = async (g: Game) => {
    setError(null)
    setLaunching(g.id)
    try {
      await launchGame(g.id)
    } catch (e) {
      setError(String(e))
    }
    setTimeout(() => setLaunching(null), 4000)
  }
  return { play, launching, error }
}
