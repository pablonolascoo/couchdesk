import { useEffect, useSyncExternalStore } from 'react'
import { playAmbience, soundForCollection } from '../audio/saverAmbience'
import type { SaverSettings } from '../data/screensaver'

// Sonido del salvapantallas. El paisaje que se ve ahora lo cuenta Screensaver.tsx con setSaverClip.

interface ClipInfo { collection: string; source: string }
let clip: ClipInfo | null = null
const listeners = new Set<() => void>()
const subscribe = (fn: () => void) => { listeners.add(fn); return () => { listeners.delete(fn) } }

export function setSaverClip(c: ClipInfo | null) {
  if (c?.collection === clip?.collection && c?.source === clip?.source) return
  clip = c
  listeners.forEach((l) => l())
}

/** Los vídeos de YouTube traen su propio sonido: se usa ese en lugar del generado */
export const videoHasSound = (s: SaverSettings, c: { source: string }) => !!s.sound && c.source === 'youtube'

export function SaverSound({ settings }: { settings: SaverSettings }) {
  const current = useSyncExternalStore(subscribe, () => clip)
  const on = !!settings.sound && !(current && videoHasSound(settings, current))
  const kind = on ? soundForCollection(current?.collection ?? null, settings.style) : null
  const level = Math.max(0, Math.min(100, settings.soundVolume ?? 50)) / 100

  useEffect(() => {
    if (!kind) return
    const p = playAmbience(kind)
    p?.setLevel(level, 3)
    return () => {
      // Cambio de paisaje: el anterior se va despacio
      p?.setLevel(0, 2.5)
      window.setTimeout(() => p?.dispose(), 2800)
    }
  }, [kind, level])

  return null
}
