// Ventana "capa-escritorio": los marcos de las zonas, detrás de los iconos del escritorio de Windows
// y encima del fondo. Transparente, sin clics y sin animación: solo se repinta cuando cambian las zonas.
import { useEffect, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { call, isDesktopApp } from '../native'
import { useT } from '../i18n'
import { zoneName, mockFrames, type Frames } from './DeskZones'
import './DeskZones.css'

export function DeskZonesLayer() {
  const t = useT()
  const [frames, setFrames] = useState<Frames | null>(null)

  useEffect(() => {
    document.documentElement.classList.add('dz-layer-root')
    call<Frames>('deskzones_frames', {}, mockFrames()).then(setFrames).catch(() => {})
    if (!isDesktopApp) return
    const off = listen<Frames>('deskzones-frames', (e) => setFrames(e.payload))
    return () => { off.then((f) => f()) }
  }, [])

  if (!frames) return null
  // Rust da píxeles de pantalla; la página trabaja en píxeles CSS
  const dpr = window.devicePixelRatio || 1
  const px = (v: number) => `${v / dpr}px`
  return (
    <div className="dz-layer" aria-hidden>
      {frames.zones.map((z) => (
        <section key={z.id} className="dz-frame" style={{ left: px(z.rect.x - frames.monitor.x), top: px(z.rect.y - frames.monitor.y), width: px(z.rect.w), height: px(z.rect.h) }}>
          <header className="dz-frame-head" style={{ height: px(frames.header), padding: `0 ${px(frames.pad + 4)}` }}>
            <span className="dz-frame-title">{zoneName(z)}</span>
            <span className="dz-frame-count">{t('deskzones.zone.count', { n: z.count })}</span>
          </header>
        </section>
      ))}
    </div>
  )
}
