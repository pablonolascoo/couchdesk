import { useEffect, useState, type CSSProperties, type ReactNode } from 'react'
import { convertFileSrc } from '@tauri-apps/api/core'
import { GameController, MagnifyingGlass, ArrowSquareOut, AmazonLogo, Television, FilmSlate, Popcorn, FilmStrip, Cloud } from '@phosphor-icons/react'
import { BrandLogo, type Brand } from '../ui/brands'
import { Page } from '../components/Page'
import { call, isDesktopApp } from '../native'
import './Browser.css'

interface Favorite { name: string; url: string; tone: string; brand?: Brand; logoColor?: string; icon?: ReactNode }
interface Preview { url: string; image?: string | null; icon?: string | null }

// Cada acceso con su logo (SVG oficial si lo hay; si no, el icono grande de la propia web) y su color.
// De fondo, la imagen de compartir de cada web cuando la tiene. Agrupados por tipo.
const groups: { title: string; items: Favorite[] }[] = [
  {
    title: 'Cine y series',
    items: [
      { name: 'Netflix', url: 'https://www.netflix.com/es/', brand: 'netflix', tone: '#e50914' },
      { name: 'Prime Video', url: 'https://www.primevideo.com/', tone: '#1a98ff', icon: <Television size={30} weight="fill" /> },
      { name: 'Disney+', url: 'https://www.disneyplus.com/es-es', tone: '#113ccf', icon: <FilmSlate size={30} weight="fill" /> },
      { name: 'HBO Max', url: 'https://www.hbomax.com/es/es', brand: 'hbomax', tone: '#2a46e0' },
      { name: 'Apple TV', url: 'https://tv.apple.com/es', brand: 'appletv', tone: '#1d1d1f' },
      { name: 'Movistar Plus+', url: 'https://ver.movistarplus.es/', brand: 'movistar', tone: '#019df4' },
      { name: 'SkyShowtime', url: 'https://www.skyshowtime.com/es', tone: '#1b3fa0', icon: <Popcorn size={30} weight="fill" /> },
      { name: 'Filmin', url: 'https://www.filmin.es/', tone: '#d81e4a', icon: <FilmStrip size={30} weight="fill" /> },
      { name: 'RTVE Play', url: 'https://www.rtve.es/play/', tone: '#e0611b', icon: <Television size={30} weight="fill" /> },
      { name: 'Atresplayer', url: 'https://www.atresplayer.com/', tone: '#f08a00', icon: <Television size={30} weight="fill" /> },
      { name: 'Mitele', url: 'https://www.mitele.es/', tone: '#0a8fd6', icon: <Television size={30} weight="fill" /> },
      { name: 'Crunchyroll', url: 'https://www.crunchyroll.com/es-es', brand: 'crunchyroll', tone: '#ff5e00' },
    ],
  },
  {
    title: 'Deporte, directos y música',
    items: [
      { name: 'YouTube', url: 'https://www.youtube.com/', brand: 'youtube', tone: '#ff0000' },
      { name: 'DAZN', url: 'https://www.dazn.com/es-ES', brand: 'dazn', logoColor: '#0c161c', tone: '#c9d600' },
      { name: 'Twitch', url: 'https://www.twitch.tv/', brand: 'twitch', tone: '#9146ff' },
      { name: 'Kick', url: 'https://kick.com/', brand: 'kick', logoColor: '#0b0e0f', tone: '#3fd40f' },
      { name: 'Spotify', url: 'https://open.spotify.com/', brand: 'spotify', tone: '#1ed760' },
      { name: 'Apple Music', url: 'https://music.apple.com/es/', brand: 'applemusic', tone: '#fa243c' },
    ],
  },
  {
    title: 'Juegos en la nube',
    items: [
      { name: 'Xbox Cloud Gaming', url: 'https://www.xbox.com/es-ES/play', tone: '#107c10', icon: <GameController size={32} weight="fill" /> },
      { name: 'GeForce NOW', url: 'https://play.geforcenow.com/', brand: 'nvidia', tone: '#76b900' },
      { name: 'Boosteroid', url: 'https://cloud.boosteroid.com/', tone: '#2f55e8', icon: <Cloud size={30} weight="fill" /> },
    ],
  },
  {
    title: 'Compras y consultas',
    items: [
      { name: 'Amazon', url: 'https://www.amazon.es/', tone: '#ff9900', icon: <AmazonLogo size={32} weight="bold" /> },
      { name: 'Google', url: 'https://www.google.com/', brand: 'google', tone: '#4285f4' },
      { name: 'Wikipedia', url: 'https://es.wikipedia.org/', brand: 'wikipedia', tone: '#3b3b3b' },
    ],
  },
]
const favorites = groups.flatMap((g) => g.items)

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

      {groups.map((g) => (
        <section key={g.title}>
          <h3 className="label" style={{ marginTop: 28 }}>{g.title}</h3>
          <div className="web-grid">
            {g.items.map((f, j) => {
              const i = favorites.indexOf(f)
              const p = previews[f.url]
              const bg = p?.image ? convertFileSrc(p.image) : null
              const siteIcon = p?.icon ? convertFileSrc(p.icon) : null
              return (
                <button
                  key={f.url}
                  className={`web-card reveal${bg ? ' has-image' : ''}`}
                  style={{ '--tone': f.tone, '--i': j } as CSSProperties}
                  data-nav={`fav-${i}`}
                  data-nav-default={i === 0 ? '' : undefined}
                  onClick={() => open(f.url)}
                >
                  {bg && <img className="web-bg" src={bg} alt="" draggable={false} />}
                  <span className="web-art" aria-hidden>
                    {f.brand ? <BrandLogo name={f.brand} size={170} color="currentColor" /> : f.icon}
                  </span>
                  <span className="web-logo">
                    {f.brand ? <BrandLogo name={f.brand} size={34} color={f.logoColor} /> : siteIcon ? <img src={siteIcon} alt="" width={40} height={40} /> : f.icon}
                  </span>
                  <span className="web-text">
                    <span className="web-name">{f.name}</span>
                    <span className="web-host">{host(f.url)}</span>
                  </span>
                </button>
              )
            })}
          </div>
        </section>
      ))}

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
