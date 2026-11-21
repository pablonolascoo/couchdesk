import { useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { PaperPlaneRight, Sparkle, ArrowCounterClockwise, Wrench } from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { call, isDesktopApp } from '../native'
import './Claude.css'

interface Msg { role: 'user' | 'claude' | 'tool' | 'error'; text: string }

const suggestions = [
  '¿Qué volumen tengo?',
  'Pon el brillo al 50 %',
  '¿Qué juegos tengo instalados?',
  '¿Qué emuladores irán bien en mi PC?',
  '¿Cuánta batería me queda?',
  'Enséñame lo que hay en Descargas',
]

// Conversación recordada al salir y volver a la pantalla
let history: Msg[] = []

export function Claude() {
  const [msgs, setMsgs] = useState<Msg[]>(history)
  const [text, setText] = useState('')
  const [busy, setBusy] = useState(false)
  const [available, setAvailable] = useState<boolean | null>(null)
  const listRef = useRef<HTMLDivElement>(null)

  const push = (m: Msg) => setMsgs((prev) => (history = [...prev, m]))

  useEffect(() => {
    call<boolean>('claude_available', {}, true).then(setAvailable)
    if (!isDesktopApp) return
    const off = listen<{ kind: string; text: string }>('claude-event', ({ payload }) => {
      if (payload.kind === 'text' && payload.text.trim()) push({ role: 'claude', text: payload.text })
      if (payload.kind === 'tool') push({ role: 'tool', text: payload.text })
      if (payload.kind === 'error') push({ role: 'error', text: payload.text })
      if (payload.kind === 'done') setBusy(false)
    })
    return () => { off.then((f) => f()) }
  }, [])

  useEffect(() => { listRef.current?.scrollTo({ top: listRef.current.scrollHeight, behavior: 'smooth' }) }, [msgs])

  const ask = async (q: string) => {
    if (!q.trim() || busy) return
    push({ role: 'user', text: q })
    setText('')
    setBusy(true)
    try {
      await call('claude_ask', { prompt: q })
      if (!isDesktopApp) setTimeout(() => { push({ role: 'tool', text: 'ver volumen' }); push({ role: 'claude', text: 'Tienes el volumen al 60 %.' }); setBusy(false) }, 600)
    } catch (e) {
      push({ role: 'error', text: String(e) })
      setBusy(false)
    }
  }
  const reset = () => { history = []; setMsgs([]); call('claude_reset') }

  if (available === false) {
    return <Page title="Claude"><p className="notice error">Claude Code no está instalado en este PC.</p></Page>
  }

  return (
    <Page title="Claude" hints={[['A', 'Enviar / elegir'], ['B', 'Atrás']]}>
      <div className="chat" ref={listRef}>
        {msgs.length === 0 && (
          <p className="hero-meta chat-empty"><Sparkle size={18} weight="fill" /> Pídele cosas a tu PC: volumen, brillo, juegos, emuladores, archivos…</p>
        )}
        {msgs.map((m, i) => (
          <div key={i} className={`bubble ${m.role}`}>
            {m.role === 'tool' ? <><Wrench size={14} weight="light" /> Usando: {m.text}</> : m.text}
          </div>
        ))}
        {busy && <div className="bubble claude typing">Pensando…</div>}
      </div>

      <div className="chips">
        {suggestions.map((s, i) => (
          <button key={s} className="option tool" data-nav={`sug-${i}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => ask(s)} disabled={busy}>{s}</button>
        ))}
      </div>

      <form className="ask" onSubmit={(e) => { e.preventDefault(); ask(text) }}>
        <input
          className="ask-input"
          data-nav="ask-input"
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Escribe aquí (teclado, o el teclado del Mando móvil)"
        />
        <button className="btn-primary" data-nav="send" type="submit" disabled={busy || !text.trim()}><PaperPlaneRight size={18} weight="fill" /> Enviar</button>
        <button className="btn-ghost" data-nav="reset" type="button" onClick={reset}><ArrowCounterClockwise size={18} weight="light" /> Nueva</button>
      </form>
    </Page>
  )
}
