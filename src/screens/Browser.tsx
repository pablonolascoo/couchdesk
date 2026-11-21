import { useState } from 'react'
import { GameController, MagnifyingGlass, ArrowSquareOut } from '@phosphor-icons/react'
import { BrandLogo, type Brand } from '../ui/brands'
import './Home.css'
import { Page } from '../components/Page'
import { call } from '../native'

// Cada favorito con su logo real y su color
const favorites: { name: string; url: string; brand?: Brand; tone: string }[] = [
  { name: 'YouTube', url: 'https://www.youtube.com/tv', brand: 'youtube', tone: '#ff0000' },
  { name: 'Twitch', url: 'https://www.twitch.tv', brand: 'twitch', tone: '#9146ff' },
  { name: 'Google', url: 'https://www.google.com', brand: 'google', tone: '#4285f4' },
  { name: 'Wikipedia', url: 'https://es.wikipedia.org', brand: 'wikipedia', tone: '#3b3b3b' },
  { name: 'Xbox Cloud Gaming', url: 'https://www.xbox.com/play', tone: '#107c10' },
  { name: 'GeForce NOW', url: 'https://play.geforcenow.com', brand: 'nvidia', tone: '#76b900' },
]

export function Browser() {
  const [text, setText] = useState('')
  const [error, setError] = useState<string | null>(null)
  const open = (url: string) => { setError(null); call('browser_open', { url }).catch((e) => setError(String(e))) }

  return (
    <Page title="Navegador" hints={[['A', 'Abrir'], ['B', 'Atrás']]}>
      <form className="ask" style={{ marginTop: 0 }} onSubmit={(e) => { e.preventDefault(); if (text.trim()) open(text) }}>
        <input className="ask-input" data-nav="url" value={text} onChange={(e) => setText(e.target.value)} placeholder="Escribe una web o lo que quieras buscar" />
        <button className="btn-primary" data-nav="go" type="submit" disabled={!text.trim()}><MagnifyingGlass size={18} weight="light" /> Ir</button>
      </form>

      <h3 className="label" style={{ marginTop: 28 }}>Favoritos</h3>
      <div className="fav-grid">
        {favorites.map(({ name, url, brand, tone }, i) => (
          <button key={url} className="tile" style={{ ['--tone' as string]: tone }} data-nav={`fav-${i}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => open(url)}>
            <span className="tile-art" aria-hidden>
              {brand ? <BrandLogo name={brand} size={92} color="currentColor" /> : <GameController size={96} weight="fill" />}
            </span>
            <span className="tile-icon">{brand ? <BrandLogo name={brand} size={20} color="currentColor" /> : <GameController size={22} weight="fill" />}</span>
            <span className="tile-label">{name}</span>
            <span className="tile-hint">{url.replace('https://', '').replace(/\/.*$/, '')}</span>
          </button>
        ))}
      </div>

      <button className="option tool" style={{ marginTop: 22 }} data-nav="system" onClick={() => call('browser_in_system', { url: text || 'https://www.google.com' })}>
        <ArrowSquareOut size={18} weight="light" /> Abrir en el navegador de Windows (Chrome, Edge…)
      </button>

      <h3 className="label" style={{ marginTop: 28 }}>Con el mando, dentro de una web</h3>
      <dl className="detail-facts">
        <dt><kbd>Cruceta</kbd></dt><dd>Saltar entre enlaces y botones</dd>
        <dt><kbd>Stick der.</kbd></dt><dd>Desplazar la página</dd>
        <dt><kbd>A</kbd></dt><dd>Abrir</dd>
        <dt><kbd>B</kbd></dt><dd>Página anterior (en la primera, vuelve a la consola)</dd>
        <dt><kbd>Y</kbd></dt><dd>Volver a la consola</dd>
        <dt><kbd>☰</kbd></dt><dd>Abrir esta web en el navegador de Windows</dd>
      </dl>
      <p className="notice info">Para escribir dentro de una web, usa el teclado del Mando móvil. Si Google no te deja iniciar sesión aquí, usa «Abrir en el navegador de Windows».</p>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}
