import { useRemote } from '../data/remote'
import './RemoteCursors.css'

// Cursores con nombre de cada móvil conectado (como en Figma).
// Las coordenadas son de pantalla; la consola ocupa la pantalla completa.
export function RemoteCursors() {
  const remote = useRemote()
  const ratio = window.devicePixelRatio || 1
  return (
    <div className="remote-layer" aria-hidden>
      {remote?.clients.map((c) => (
        <div key={c.id} className="remote-cursor" style={{ transform: `translate(${c.x / ratio}px, ${c.y / ratio}px)`, color: c.color }}>
          <svg width="18" height="18" viewBox="0 0 18 18"><path d="M1 1l6.5 15 2.2-6.3L16 7.5z" fill="currentColor" stroke="#fff" strokeWidth="1.2" /></svg>
          <span style={{ background: c.color }}>{c.name}</span>
        </div>
      ))}
    </div>
  )
}
