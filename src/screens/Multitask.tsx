// Multitarea: Inicio y cada ventana abierta en una fila de tarjetas, con accesos rápidos debajo.
// Se abre con el botón Xbox o PS (corto) o Ctrl + Alt + Inicio, desde cualquier sitio: lo hace
// src-tauri/src/multitask.rs, que manda la lista ya hecha ("multitask-open") para que se vea al momento.
// A cambia · X cierra · Y libera memoria · B vuelve a lo que había delante.
// Rutas: /multitarea · /multitarea/forzar/:hwnd · /ajustes/rendimiento/memoria
import { useCallback, useEffect, useRef, useState, type CSSProperties } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import {
  AppWindow, ArrowLeft, CheckCircle, Cloud, Gear, Globe, House, Memory, Minus, MusicNotes, Pause, Play, Plus, Power, Sparkle, Warning,
} from '@phosphor-icons/react'
import { HintBar } from '../components/HintBar'
import { Page } from '../components/Page'
import { useNowPlaying, Cover } from '../components/NowPlaying'
import { iconSrc } from '../data/games'
import { call } from '../native'
import { useT, fmt } from '../i18n'
import type { PadKey } from '../nav/padConfig'
import {
  MB, MOCK_FREED, MOCK_MEM, dropCard, errorText, getSnapshot, refresh, setSnapshot, snapshotAge, useSnapshot,
  type Card, type Freed, type MemStatus,
} from './multitaskStore'
import './Multitask.css'

// ---------------- Piezas ----------------

function CardIcon({ card, size }: { card: Card; size: number }) {
  const src = iconSrc(card)
  if (src) return <img className="mt-icon" src={src} alt="" width={size} height={size} draggable={false} />
  const Icon = card.kind === 'navegador' ? Globe : card.kind === 'nube' ? Cloud : AppWindow
  return <span className="mt-icon mt-icon-empty" style={{ width: size, height: size }}><Icon size={Math.round(size * 0.56)} weight="light" /></span>
}

function Shot({ card }: { card: Card }) {
  const src = iconSrc({ icon: card.thumb })
  return (
    <span className="mt-shot">
      {src ? <img src={src} alt="" draggable={false} /> : <span className="mt-shot-empty"><CardIcon card={card} size={72} /></span>}
    </span>
  )
}

function useVolume() {
  const [level, setLevel] = useState<number | null>(null)
  useEffect(() => { call<{ level: number }>('get_volume', {}, { level: 60 }).then((v) => setLevel(v.level)).catch(() => {}) }, [])
  const change = (delta: number) => setLevel((l) => {
    const next = Math.max(0, Math.min(100, (l ?? 50) + delta))
    call('set_volume', { level: next }).catch(() => {})
    return next
  })
  return { level, change }
}

function ControlCenter() {
  const t = useT()
  const navigate = useNavigate()
  const { level, change } = useVolume()
  const { track, control } = useNowPlaying()
  return (
    <nav className="mt-cc" aria-label={t('multitask.cc.label')}>
      <div className="mt-cc-group mt-vol">
        <button className="mt-cc-round" data-nav="cc-vol-down" onClick={() => change(-10)} aria-label={t('multitask.cc.volumeDown')}><Minus size={18} weight="light" /></button>
        <span className="mt-vol-level mono">{level ?? '—'}%</span>
        <button className="mt-cc-round" data-nav="cc-vol-up" onClick={() => change(10)} aria-label={t('multitask.cc.volumeUp')}><Plus size={18} weight="light" /></button>
      </div>
      <button className="mt-cc-btn mt-music" data-nav="cc-music" onClick={() => (track ? control('toggle') : navigate('/musica'))}>
        {track ? <Cover track={track} size={32} /> : <MusicNotes size={20} weight="light" />}
        <span className="mt-music-text">{track ? track.title : t('multitask.cc.nothing')}</span>
        {track && (track.playing ? <Pause size={16} weight="fill" /> : <Play size={16} weight="fill" />)}
      </button>
      <button className="mt-cc-btn" data-nav="cc-claude" onClick={() => navigate('/claude')}><Sparkle size={20} weight="light" /> {t('multitask.cc.claude')}</button>
      <button className="mt-cc-btn" data-nav="cc-settings" onClick={() => navigate('/ajustes')}><Gear size={20} weight="light" /> {t('multitask.cc.settings')}</button>
      <button className="mt-cc-btn" data-nav="cc-power" onClick={() => navigate('/ajustes/energia')}><Power size={20} weight="light" /> {t('multitask.cc.power')}</button>
    </nav>
  )
}

function RamPill({ mem }: { mem: MemStatus }) {
  const t = useT()
  const navigate = useNavigate()
  const pct = mem.totalMb ? Math.round((mem.usedMb / mem.totalMb) * 100) : 0
  return (
    <button className="mt-ram" data-nav="mt-ram" onClick={() => navigate('/ajustes/rendimiento/memoria')}>
      <Memory size={20} weight="light" />
      <span className="mt-ram-text">
        <span>{t('multitask.ramLabel')} · {t('multitask.ram', { used: fmt.bytes(mem.usedMb * MB), total: fmt.bytes(mem.totalMb * MB) })}</span>
        {mem.lastFreedMb != null && <small>{t('multitask.lastFreed', { size: fmt.bytes(mem.lastFreedMb * MB) })}</small>}
      </span>
      <span className="mt-ram-bar" aria-hidden><i style={{ transform: `scaleX(${pct / 100})` }} /></span>
    </button>
  )
}

// ---------------- Pantalla ----------------

export function Multitask() {
  const t = useT()
  const navigate = useNavigate()
  const snap = useSnapshot()
  const [closing, setClosing] = useState<number[]>([])
  const [notice, setNotice] = useState<{ kind: 'info' | 'ok' | 'error'; text: string } | null>(null)
  const focused = useRef<string>('mt-home')
  const timers = useRef<number[]>([])

  // La parte nativa sabe si la pantalla está a la vista (el botón otra vez = volver)
  useEffect(() => {
    call('multitask_visible', { on: true }).catch(() => {})
    if (!getSnapshot() || snapshotAge() > 1500) refresh().catch(() => {})
    const list = timers.current
    return () => {
      call('multitask_visible', { on: false }).catch(() => {})
      list.forEach(clearTimeout)
    }
  }, [])

  const say = useCallback((kind: 'info' | 'ok' | 'error', text: string, ms = 4000) => {
    setNotice({ kind, text })
    timers.current.push(window.setTimeout(() => setNotice((n) => (n?.text === text ? null : n)), ms))
  }, [])

  const cardFor = (nav: string) => getSnapshot()?.cards.find((c) => `mt-${c.hwnd}` === nav)

  const switchTo = (card: Card | null) => {
    if (!card) return navigate('/')
    call('multitask_switch', { hwndId: card.hwnd })
      .then(() => navigate(-1))
      .catch((e) => { say('error', errorText(e)); refresh().catch(() => {}) })
  }

  const close = useCallback((card: Card) => {
    setClosing((l) => [...l, card.hwnd])
    call('multitask_close', { hwndId: card.hwnd }).catch((e) => say('error', errorText(e)))
    // Se le da 3 s para cerrarse por las buenas; si sigue, se ofrece forzar
    timers.current.push(window.setTimeout(async () => {
      const alive = await call<boolean>('multitask_alive', { hwndId: card.hwnd }, false).catch(() => false)
      setClosing((l) => l.filter((h) => h !== card.hwnd))
      if (alive) navigate(`/multitarea/forzar/${card.hwnd}`)
      else dropCard(card.hwnd)
    }, 3000))
  }, [navigate, say])

  const free = useCallback(() => {
    say('info', t('multitask.freeing'), 20000)
    call<Freed>('memory_free', {}, MOCK_FREED)
      .then((f) => {
        const mb = Math.max(f.freedMb, f.trimmedMb)
        say('ok', mb > 0 ? t('multitask.freed', { size: fmt.bytes(mb * MB) }) : t('multitask.freedNone'))
        return call<MemStatus>('memory_status', {}, { ...MOCK_MEM, lastFreedMb: mb })
      })
      .then((memory) => { const s = getSnapshot(); if (s) setSnapshot({ ...s, memory }) })
      .catch((e) => say('error', errorText(e)))
  }, [say, t])

  const back = useCallback(() => { call('multitask_back').catch(() => {}) }, [])

  useEffect(() => {
    const onFocus = (e: Event) => { focused.current = (e.target as HTMLElement).dataset.nav ?? '' }
    // B (o Esc) ya hace "atrás" en la interfaz; aquí además se trae delante lo que había
    const onKey = (k: PadKey) => {
      if (k === 'back') back()
      const card = cardFor(focused.current)
      if (k === 'x' && card && !closing.includes(card.hwnd)) close(card)
      if (k === 'y') free()
    }
    const onPad = (e: Event) => onKey((e as CustomEvent<PadKey>).detail)
    const onKeyboard = (e: KeyboardEvent) => {
      if (e.altKey || e.ctrlKey || e.metaKey) return
      if (e.key === 'Escape' || e.key === 'Backspace') back()
      if (e.key === 'Delete') onKey('x')
    }
    // LB / RB: tarjeta anterior o siguiente (como la cruceta)
    const onPage = (e: Event) => {
      const dir = (e as CustomEvent<number>).detail < 0 ? 'ArrowLeft' : 'ArrowRight'
      if (focused.current.startsWith('mt-') && focused.current !== 'mt-ram') window.dispatchEvent(new KeyboardEvent('keydown', { key: dir }))
    }
    // Botón central otra vez: volver igual que con B
    const onGuide = () => { back(); navigate(-1) }
    window.addEventListener('navfocus', onFocus)
    window.addEventListener('padkey', onPad)
    window.addEventListener('keydown', onKeyboard)
    window.addEventListener('padpage', onPage)
    window.addEventListener('multitask-back', onGuide)
    return () => {
      window.removeEventListener('navfocus', onFocus)
      window.removeEventListener('padkey', onPad)
      window.removeEventListener('keydown', onKeyboard)
      window.removeEventListener('padpage', onPage)
      window.removeEventListener('multitask-back', onGuide)
    }
  }, [back, close, closing, free, navigate]) // eslint-disable-line react-hooks/exhaustive-deps

  const cards = snap?.cards ?? []
  return (
    <div className="screen mt">
      <header className="mt-head">
        <div className="mt-head-text">
          <span className="eyebrow">{t('multitask.title')}</span>
          <p className="mt-help">{t('multitask.help')}</p>
        </div>
        {snap && <RamPill mem={snap.memory} />}
      </header>

      <div className="mt-row" role="list">
        <button className="mt-card mt-card-home" role="listitem" data-nav="mt-home" data-nav-default={snap?.fromOutside ? undefined : ''} style={{ '--i': 0 } as CSSProperties} onClick={() => switchTo(null)}>
          <span className="mt-shot mt-shot-home"><span className="mt-home-mark" aria-hidden><House size={56} weight="light" /></span></span>
          <span className="mt-meta">
            <span className="mt-icon mt-icon-home" aria-hidden><House size={18} weight="light" /></span>
            <span className="mt-names"><span className="mt-app">{t('multitask.home')}</span><span className="mt-title">{t('multitask.homeSub')}</span></span>
          </span>
        </button>
        {cards.map((c, i) => {
          const isClosing = closing.includes(c.hwnd)
          return (
            <button
              key={c.hwnd}
              className={`mt-card${isClosing ? ' is-closing' : ''}`}
              role="listitem"
              data-nav={`mt-${c.hwnd}`}
              data-nav-default={snap?.fromOutside && i === 0 ? '' : undefined}
              style={{ '--i': Math.min(i + 1, 10) } as CSSProperties}
              onClick={() => switchTo(c)}
            >
              <Shot card={c} />
              {(isClosing || c.hung || c.minimized) && (
                <span className={`mt-badge${c.hung && !isClosing ? ' is-warn' : ''}`}>
                  {isClosing ? t('multitask.closing') : c.hung ? t('multitask.hung') : t('multitask.minimized')}
                </span>
              )}
              <span className="mt-meta">
                <CardIcon card={c} size={32} />
                <span className="mt-names">
                  <span className="mt-app">{c.app}</span>
                  {c.title !== c.app && <span className="mt-title">{c.title}</span>}
                </span>
                {c.memory > 0 && <span className="mt-mem mono">{fmt.bytes(c.memory)}</span>}
              </span>
            </button>
          )
        })}
        {snap && cards.length === 0 && <p className="mt-empty">{t('multitask.empty')}</p>}
      </div>

      <div className="mt-foot">
        {notice && <p className={`mt-notice is-${notice.kind}`} role="status">{notice.text}</p>}
        <ControlCenter />
      </div>

      <HintBar hints={[['A', t('multitask.hint.switch')], ['X', t('multitask.hint.close')], ['Y', t('multitask.hint.free')], ['B', t('multitask.hint.back')]]} />
    </div>
  )
}

// ---------------- Forzar cierre (confirmación) ----------------

export function MultitaskForceClose() {
  const t = useT()
  const navigate = useNavigate()
  const { hwnd = '' } = useParams()
  const id = Number(hwnd)
  const card = useSnapshot()?.cards.find((c) => c.hwnd === id)
  const [error, setError] = useState<string | null>(null)
  const force = () =>
    call('multitask_force_close', { hwndId: id })
      .then(() => { dropCard(id); navigate(-1) })
      .catch((e) => setError(errorText(e)))
  return (
    <Page title={t('multitask.force.title')}>
      <div className="mt-force">
        {card && <CardIcon card={card} size={64} />}
        {card && <h2 className="mt-force-title">{t('multitask.force.question', { app: card.app })}</h2>}
        <p className="hero-meta">{t('multitask.force.text')}</p>
        <div className="hero-actions">
          <button className="pill pill-dark" data-nav="force" onClick={force}>
            {t('multitask.force.confirm')} <span className="pill-icon"><Warning size={16} weight="light" /></span>
          </button>
          <button className="pill" data-nav="wait" data-nav-default="" onClick={() => navigate(-1)}>
            {t('multitask.force.wait')} <span className="pill-icon"><ArrowLeft size={16} weight="light" /></span>
          </button>
        </div>
        {error && <p className="notice error">{error}</p>}
      </div>
    </Page>
  )
}

// ---------------- Ajustes → Rendimiento → Memoria ----------------

export function MemorySettings() {
  const t = useT()
  const [mem, setMem] = useState<MemStatus | null>(null)
  const [msg, setMsg] = useState<{ kind: 'info' | 'ok' | 'error'; text: string } | null>(null)
  useEffect(() => { call<MemStatus>('memory_status', {}, MOCK_MEM).then(setMem).catch((e) => setMsg({ kind: 'error', text: String(e) })) }, [])
  const set = (patch: Partial<Pick<MemStatus, 'auto' | 'eco'>>) => {
    if (!mem) return
    const next = { ...mem, ...patch }
    setMem(next)
    call<MemStatus>('memory_settings_set', { auto: next.auto, eco: next.eco }, next).then(setMem).catch((e) => setMsg({ kind: 'error', text: String(e) }))
  }
  const free = () => {
    setMsg({ kind: 'info', text: t('multitask.freeing') })
    call<Freed>('memory_free', {}, MOCK_FREED)
      .then((f) => {
        const mb = Math.max(f.freedMb, f.trimmedMb)
        setMsg({ kind: 'ok', text: mb > 0 ? t('multitask.freed', { size: fmt.bytes(mb * MB) }) : t('multitask.freedNone') })
        return call<MemStatus>('memory_status', {}, { ...MOCK_MEM, lastFreedMb: mb }).then(setMem)
      })
      .catch((e) => setMsg({ kind: 'error', text: String(e) }))
  }
  const toggle = (id: string, on: boolean, change: (v: boolean) => void) => (
    <button className="option mt-toggle" data-nav={id} aria-pressed={on} onClick={() => change(!on)}>
      {on ? <><CheckCircle size={18} weight="fill" /> {t('multitask.mem.on')}</> : t('multitask.mem.off')}
    </button>
  )
  const sub = mem ? [mem.ecoCount ? t('multitask.mem.ecoCount', { n: mem.ecoCount }) : '', mem.boosted ? t('multitask.mem.boosted', { app: mem.boosted }) : ''].filter(Boolean).join(' · ') : ''
  return (
    <Page title={t('multitask.mem.title')}>
      <div className="row">
        <span className="row-label">{t('multitask.mem.used')}{sub && <span className="row-sub">{sub}</span>}</span>
        <span className="row-value">{mem ? t('multitask.ram', { used: fmt.bytes(mem.usedMb * MB), total: fmt.bytes(mem.totalMb * MB) }) : '—'}</span>
      </div>
      <div className="row">
        <span className="row-label">{t('multitask.mem.free')}<span className="row-sub">{t('multitask.mem.freeSub')}</span></span>
        <button className="option mt-toggle" data-nav="mem-free" data-nav-default="" onClick={free}><Memory size={18} weight="light" /> {t('multitask.mem.free')}</button>
      </div>
      {msg && <p className={`notice ${msg.kind}`} role="status">{msg.text}</p>}
      <div className="row">
        <span className="row-label">{t('multitask.mem.auto')}<span className="row-sub">{t('multitask.mem.autoSub')}</span></span>
        {toggle('mem-auto', mem?.auto ?? true, (v) => set({ auto: v }))}
      </div>
      <div className="row">
        <span className="row-label">{t('multitask.mem.eco')}<span className="row-sub">{t('multitask.mem.ecoSub')}</span></span>
        {toggle('mem-eco', mem?.eco ?? true, (v) => set({ eco: v }))}
      </div>
      <p className="notice info">{t('multitask.mem.info')}</p>
    </Page>
  )
}
