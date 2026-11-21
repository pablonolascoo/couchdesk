import { useEffect, useState, type CSSProperties, type ReactNode } from 'react'
import { convertFileSrc } from '@tauri-apps/api/core'
import { GameController, MagnifyingGlass, ArrowSquareOut, AmazonLogo, Television } from '@phosphor-icons/react'
import { BrandLogo, type Brand } from '../ui/brands'
import { Page } from '../components/Page'
import { call, isDesktopApp } from '../native'
import './Browser.css'

interface Favorite { name: string; url: string; tone: string; brand?: Brand; icon?: ReactNode }
interface Preview { url: string; image?: string | null; icon?: string | null }

// Cada acceso con su logo (SVG oficial si lo hay; si no, el icono grande de la propia web) y su color.
// De fondo, la imagen de compartir de cada web cuando la tiene.
const favorites: Favorite[] = [
  { name: 'YouTube', url: 'https://www.youtube.com/', brand: 'youtube', tone: '#ff0000' },
  { name: 'Netflix', url: 'https://www.netflix.com/es/', brand: 'netflix', tone: '#e50914' },
  { name: 'Prime Video', url: 'https://www.primevideo.com/', tone: '#1a98ff', icon: <Television size={30} weight="fill" /> },
  { name: 'Amazon', url: 'https://www.amazon.es/', tone: '#ff9900', icon: <AmazonLogo size={32} weight="bold" /> },
  { name: 'Twitch', url: 'https://www.twitch.tv/', brand: 'twitch', tone: '#9146ff' },
  { name: 'Xbox Cloud Gaming', url: 'https://www.xbox.com/es-ES/play', tone: '#107c10', icon: <GameController size={32} weight="fill" /> },
  { name: 'GeForce NOW', url: 'https://play.geforcenow.com/', brand: 'nvidia', tone: '#76b900' },
  { name: 'Google', url: 'https://www.google.com/', brand: 'google', tone: '#4285f4' },
  { name: 'Wikipedia', url: 'https://es.wikipedia.org/', brand: 'wikipedia', tone: '#3b3b3b' },
]

const host = (url: string) => new URL(url).hostname.replace(/^www\./, '')

export function Browser() {
  const [text, setText] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [previews, setPreviews] = useState<Record<string, Preview>>({})
  const open = (url: string) => { setError(null); call('browser_open', { url }).catch((e) => setError(String(e))) }

  // Imágenes e iconos de cada web (se guardan en el PC; la primera vez tarda unos segundos)
  useEffect(() => {
    if (!isDesktopApp) return
    call<Preview[]>('site_previews', { urls: favorites.map((f) => f.url) })
      .then((list) => setPreviews(Object.fromEntries(list.map((p) => [p.url, p]))))
      .catch(() => {})
  }, [])

  return (
    <Page title="Navegador" hints={[['A', 'Abrir'], ['B', 'Atrás']]}>
      <form className="ask" style={{ marginTop: 0 }} onSubmit={(e) => { e.preventDefault(); if (text.trim()) open(text) }}>
        <input className="ask-input" data-nav="url" value={text} onChange={(e) => setText(e.target.value)} placeholder="Escribe una web o lo que quieras buscar" />
        <button className="btn-primary" data-nav="go" type="submit" disabled={!text.trim()}><MagnifyingGlass size={18} weight="light" /> Ir</button>
      </form>

      <h3 className="label" style={{ marginTop: 28 }}>Accesos directos</h3>
      <div className="web-grid">
        {favorites.map((f, i) => {
          const p = previews[f.url]
          const bg = p?.image ? convertFileSrc(p.image) : null
          const siteIcon = p?.icon ? convertFileSrc(p.icon) : null
          return (
            <button
              key={f.url}
              className={`web-card reveal${bg ? ' has-image' : ''}`}
              style={{ '--tone': f.tone, '--i': i } as CSSProperties}
              data-nav={`fav-${i}`}
              data-nav-default={i === 0 ? '' : undefined}
              onClick={() => open(f.url)}
            >
              {bg && <img className="web-bg" src={bg} alt="" draggable={false} />}
              <span className="web-art" aria-hidden>
                {f.brand ? <BrandLogo name={f.brand} size={170} color="currentColor" /> : f.icon}
              </span>
              <span className="web-logo">
                {f.brand ? <BrandLogo name={f.brand} size={34} /> : siteIcon ? <img src={siteIcon} alt="" width={40} height={40} /> : f.icon}
              </span>
              <span className="web-text">
                <span className="web-name">{f.name}</span>
                <span className="web-host">{host(f.url)}</span>
              </span>
            </button>
          )
        })}
      </div>

      <button className="option tool" style={{ marginTop: 22 }} data-nav="system" onClick={() => call('browser_in_system', { url: text || 'https://www.google.com' })}>
        <ArrowSquareOut size={18} weight="light" /> Abrir en el navegador de Windows (Chrome, Edge…)
      </button>

      <h3 className="label" style={{ marginTop: 28 }}>Dentro de una web</h3>
      <dl className="detail-facts">
        <dt><kbd>Cruceta</kbd></dt><dd>Saltar entre enlaces y botones</dd>
        <dt><kbd>Stick der.</kbd></dt><dd>Desplazar la página</dd>
        <dt><kbd>A</kbd></dt><dd>Abrir</dd>
        <dt><kbd>B</kbd></dt><dd>Página anterior (en la primera, vuelve a la consola)</dd>
        <dt><kbd>Y</kbd> o botón Xbox</dt><dd>Volver a la consola</dd>
        <dt><kbd>Esc</kbd></dt><dd>Mantenlo 1 segundo (o púlsalo 3 veces) para volver a la consola, desde cualquier web</dd>
      </dl>
      <p className="notice info">Si tienes el móvil conectado como mando, se pone solo en modo touchpad y abre el teclado cuando la web pide escribir.</p>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}
