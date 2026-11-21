import { useState } from 'react'
import { Globe, MagnifyingGlass, ArrowSquareOut } from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { call } from '../native'

const favorites = [
  ['YouTube', 'https://www.youtube.com/tv'],
  ['Twitch', 'https://www.twitch.tv'],
  ['Google', 'https://www.google.com'],
  ['Wikipedia', 'https://es.wikipedia.org'],
  ['Xbox Cloud Gaming', 'https://www.xbox.com/play'],
  ['GeForce NOW', 'https://play.geforcenow.com'],
] as const

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
        {favorites.map(([name, url], i) => (
          <button key={url} className="tile" data-nav={`fav-${i}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => open(url)}>
            <Globe size={24} weight="light" />
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
