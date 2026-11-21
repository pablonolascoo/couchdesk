import { useEffect, useRef, useState, useSyncExternalStore, type FormEvent } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import {
  GameController, CaretRight, CheckCircle, Plug, Bluetooth, WifiHigh, BatteryFull, BatteryMedium,
  BatteryLow, BatteryEmpty, BatteryWarning, Plus, Copy, PencilSimple, Trash, Star, Sliders,
} from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { call } from '../native'
import { sound, padKind } from '../ui/sound'
import { t as tr, useT } from '../i18n'
import {
  ACTIONS, getConfig, setConfig, subscribe, allProfiles, findProfile, profileFor, effectiveButtons,
  duplicate, updateProfile, setButton, removeProfile, setDefault, assign, newId, buttonName,
  lockNav, unlockNav, type Action, type PadButton, type Profile, type Speed, type Zone,
} from '../nav/padConfig'
import './Controllers.css'

// ---------------- Datos ----------------

/** Lo que devuelve `pads_list` (src-tauri/src/pads.rs) */
export interface PadInfo {
  kind: 'xbox' | 'ds4' | 'ds5' | 'ds5edge'
  name: string
  slot: number | null
  connection: 'usb' | 'bluetooth' | 'wireless' | 'unknown'
  battery: 'empty' | 'low' | 'medium' | 'full' | 'wired' | 'unknown' | null
  source: 'xinput' | 'hid'
  productId: number
}

const MOCK_PADS: PadInfo[] = [
  { kind: 'xbox', name: 'Mando de Xbox', slot: 0, connection: 'wireless', battery: 'medium', source: 'xinput', productId: 0 },
  { kind: 'ds5', name: 'DualSense', slot: null, connection: 'bluetooth', battery: null, source: 'hid', productId: 0x0ce6 },
]

/** Mandos que ve Windows, recargados cada pocos segundos */
function useNativePads(every = 3000) {
  const [pads, setPads] = useState<PadInfo[] | null>(null)
  useEffect(() => {
    let alive = true
    const load = () => call<PadInfo[]>('pads_list', {}, MOCK_PADS).then((p) => alive && setPads(p)).catch(() => alive && setPads([]))
    load()
    const id = setInterval(load, every)
    return () => { alive = false; clearInterval(id) }
  }, [every])
  return pads
}

interface LivePad { index: number; id: string; pressed: number[]; axes: number[] }

/** Mandos que ve la consola en directo (con sus botones). Solo aparecen tras pulsar algo. */
function useLivePads() {
  const [pads, setPads] = useState<LivePad[]>([])
  useEffect(() => {
    let raf = 0
    let last = ''
    const tick = () => {
      const now = navigator.getGamepads().filter((p): p is Gamepad => !!p).map((p) => ({
        index: p.index,
        id: p.id,
        pressed: p.buttons.flatMap((b, i) => (b.pressed ? [i] : [])),
        axes: p.axes.slice(0, 4).map((a) => Math.round(a * 10) / 10),
      }))
      const key = JSON.stringify(now)
      if (key !== last) { last = key; setPads(now) }
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [])
  return pads
}

const usePadConfig = () => useSyncExternalStore(subscribe, getConfig)

/** Para el menú de Ajustes: "2 conectados" */
export function usePadsLabel(): string {
  const t = useT()
  const native = useNativePads(5000)
  const live = useLivePads()
  if (native === null) return ''
  const n = Math.max(native.length, live.length)
  return n === 0 ? t('controllers.count.none') : t('controllers.count.connected', { n })
}

interface Card {
  key: string
  name: string
  ps: boolean
  player: number
  connection: PadInfo['connection'] | null
  battery: PadInfo['battery'] | undefined
  live?: LivePad
}

const hex4 = (n: number) => n.toString(16).padStart(4, '0')

/** Junta lo que ve Windows con lo que ve la consola en directo */
function buildCards(native: PadInfo[], live: LivePad[]): Card[] {
  const used = new Set<number>()
  const cards: Card[] = native.map((n, i) => {
    const match = n.source === 'xinput'
      ? live.find((l) => !used.has(l.index) && /xinput/i.test(l.id))
      : live.find((l) => !used.has(l.index) && /054c/i.test(l.id) && l.id.toLowerCase().includes(hex4(n.productId)))
    if (match) used.add(match.index)
    return {
      key: n.slot !== null ? `x${n.slot}` : `hid-${i}`,
      name: n.name,
      ps: n.kind !== 'xbox',
      player: n.slot !== null ? n.slot + 1 : (match ? match.index + 1 : i + 1),
      connection: n.connection,
      battery: n.battery,
      live: match,
    }
  })
  for (const l of live) {
    if (used.has(l.index)) continue
    const ps = /054c|playstation|dualsense|dualshock/i.test(l.id)
    const name = ps ? tr('controllers.pad.playstation') : l.id.replace(/\(.*$/, '').trim() || tr('controllers.pad.generic')
    cards.push({ key: `gp-${l.index}`, name, ps, player: l.index + 1, connection: null, battery: undefined, live: l })
  }
  return cards
}

// Texto: controllers.conn.<clave> y controllers.battery.<clave>
const CONNECTION: Record<PadInfo['connection'], typeof Plug> = {
  usb: Plug,
  bluetooth: Bluetooth,
  wireless: WifiHigh,
  unknown: WifiHigh,
}
const BATTERY: Record<NonNullable<PadInfo['battery']>, typeof Plug> = {
  full: BatteryFull,
  medium: BatteryMedium,
  low: BatteryLow,
  empty: BatteryEmpty,
  wired: Plug,
  unknown: BatteryWarning,
}

const padPath = (id: string) => `/ajustes/mandos/mando/${encodeURIComponent(id)}`
const profilePath = (id: string) => `/ajustes/mandos/perfil/${encodeURIComponent(id)}`

// ---------------- Mandos conectados ----------------

const TEST_BUTTONS = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
const ARROWS: Record<number, string> = { 12: '↑', 13: '↓', 14: '←', 15: '→' }

function ButtonTest({ live, ps }: { live?: LivePad; ps: boolean }) {
  const t = useT()
  if (!live) return <p className="pad-test-wait">{t('controllers.test.wait')}</p>
  const on = new Set(live.pressed)
  const stick = (x = 0, y = 0) => <span className="pad-stick"><span style={{ translate: `${x * 14}px ${y * 14}px` }} /></span>
  return (
    <div className="pad-test" aria-label={t('controllers.test.title')}>
      {TEST_BUTTONS.map((i) => (
        <span key={i} className={`pad-btn${on.has(i) ? ' on' : ''}`}>{ARROWS[i] ?? buttonName(i, ps)}</span>
      ))}
      {stick(live.axes[0], live.axes[1])}
      {stick(live.axes[2], live.axes[3])}
    </div>
  )
}

export function ControllersScreen() {
  const t = useT()
  const navigate = useNavigate()
  const native = useNativePads()
  const live = useLivePads()
  const cfg = usePadConfig()
  const cards = buildCards(native ?? [], live)
  const firstLive = cards.findIndex((c) => c.live)

  return (
    <Page title={t('controllers.title')}>
      <button className="menu-item" data-nav="pads-profiles" data-nav-default={firstLive < 0 ? '' : undefined} onClick={() => navigate('/ajustes/mandos/perfiles')}>
        <Sliders size={24} weight="light" />
        <span className="row-label">{t('controllers.profiles')}</span>
        <span className="menu-item-value">{t('controllers.defaultIs', { name: findProfile(cfg, cfg.defaultId)?.name ?? '' })}</span>
        <CaretRight size={18} weight="light" />
      </button>

      <h3 className="label" style={{ marginTop: 32 }}>{t('controllers.connected')}</h3>
      {native === null && <p className="hero-meta">{t('controllers.searching')}</p>}
      {native !== null && cards.length === 0 && (
        <div className="empty-state">
          <span className="empty-icon"><GameController size={28} weight="light" /></span>
          <h2>{t('controllers.empty.title')}</h2>
          <p>{t('controllers.empty.text')}</p>
        </div>
      )}
      <div className="pad-cards">
        {cards.map((c, i) => {
          const [connText, ConnIcon] = c.connection ? [t(`controllers.conn.${c.connection}`), CONNECTION[c.connection]] : ['', Plug]
          const bat = c.battery ? [t(`controllers.battery.${c.battery}`), BATTERY[c.battery]] as const : null
          return (
            <section key={c.key} className="pad-card">
              <header className="pad-head">
                <span className="pad-icon"><GameController size={28} weight="light" /></span>
                <span className="pad-title">
                  <span className="pad-name">{c.name}</span>
                  <span className="pad-player">{t('controllers.player', { n: c.player })}</span>
                </span>
                <span className="pad-facts">
                  {c.connection && <span className="pad-fact"><ConnIcon size={16} weight="light" /> {connText}</span>}
                  {bat && <span className={`pad-fact${c.battery === 'low' || c.battery === 'empty' ? ' warn' : ''}`}>{(() => { const Icon = bat[1]; return <Icon size={16} weight="light" /> })()} {bat[0]}</span>}
                  {c.ps && c.battery === null && <span className="pad-fact">{t('controllers.battery.na')}</span>}
                </span>
              </header>
              <h4 className="label" style={{ margin: '18px 0 10px' }}>{t('controllers.test.title')}</h4>
              <ButtonTest live={c.live} ps={c.ps} />
              {c.live && (
                <button
                  className="option"
                  data-nav={`pad-${c.key}-profile`}
                  data-nav-default={i === firstLive ? '' : undefined}
                  onClick={() => navigate(padPath(c.live!.id))}
                >
                  <span className="opt-line"><Sliders size={20} weight="light" /> {t('controllers.pickProfile')}</span>
                  <span className="menu-item-value">{profileFor(cfg, c.live.id).name}{cfg.perPad[c.live.id] ? '' : ` ${t('controllers.defaultTag')}`}</span>
                </button>
              )}
            </section>
          )
        })}
      </div>
      {cards.some((c) => !c.live) && cards.length > 0 && (
        <p className="notice info">{t('controllers.pressToUse')}</p>
      )}
    </Page>
  )
}

// ---------------- Perfil de un mando ----------------

export function PadProfilePick() {
  const t = useT()
  const navigate = useNavigate()
  const { pad = '' } = useParams()
  const cfg = usePadConfig()
  const current = cfg.perPad[pad] ?? null
  const pick = (id: string | null) => { setConfig((c) => assign(c, pad, id)); sound.select(); navigate(-1) }
  const def = findProfile(cfg, cfg.defaultId)
  return (
    <Page title={t('controllers.padProfile')}>
      <p className="hero-meta pad-id">{pad.replace(/\(.*$/, '').trim() || pad}</p>
      <button className="option" data-nav="pick-default" data-nav-default={current === null ? '' : undefined} onClick={() => pick(null)}>
        {t('controllers.useDefault', { name: def?.name ?? '' })}
        {current === null && <span className="option-check"><CheckCircle size={18} weight="fill" /> {t('controllers.chosen')}</span>}
      </button>
      {allProfiles(cfg).map((p) => (
        <button key={p.id} className="option" data-nav={`pick-${p.id}`} data-nav-default={current === p.id ? '' : undefined} onClick={() => pick(p.id)}>
          {p.name}
          {current === p.id && <span className="option-check"><CheckCircle size={18} weight="fill" /> {t('controllers.chosen')}</span>}
        </button>
      ))}
    </Page>
  )
}

// ---------------- Lista de perfiles ----------------

export function ProfilesScreen() {
  const t = useT()
  const navigate = useNavigate()
  const cfg = usePadConfig()
  const create = () => {
    const id = newId()
    setConfig((c) => duplicate(c, 'estandar', id, t('controllers.newName')))
    navigate(profilePath(id))
  }
  return (
    <Page title={t('controllers.profiles')}>
      <div className="menu-list">
        {allProfiles(cfg).map((p, i) => (
          <button key={p.id} className="menu-item" data-nav={`prof-${p.id}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => navigate(profilePath(p.id))}>
            {p.id === cfg.defaultId ? <Star size={24} weight="light" /> : <GameController size={24} weight="light" />}
            <span className="row-label">{p.name}</span>
            <span className="menu-item-value">{[p.id === cfg.defaultId && t('controllers.default'), p.builtIn && t('controllers.builtIn')].filter(Boolean).join(' · ')}</span>
            <CaretRight size={18} weight="light" />
          </button>
        ))}
      </div>
      <button className="option" style={{ marginTop: 24 }} data-nav="prof-create" onClick={create}>
        <span className="opt-line"><Plus size={20} weight="light" /> {t('controllers.create')}</span>
      </button>
      <p className="notice info">{t('controllers.profilesInfo')}</p>
    </Page>
  )
}

// ---------------- Un perfil ----------------

type Choice<T> = { value: T; label: string }
const YES_NO = (): Choice<boolean>[] => [{ value: false, label: tr('controllers.no') }, { value: true, label: tr('controllers.yes') }]
const SPEEDS = (): Choice<Speed>[] => [{ value: 'slow', label: tr('controllers.speed.slow') }, { value: 'normal', label: tr('controllers.speed.normal') }, { value: 'fast', label: tr('controllers.speed.fast') }]
const ZONES = (): Choice<Zone>[] => [{ value: 'small', label: tr('controllers.zone.small') }, { value: 'normal', label: tr('controllers.zone.normal') }, { value: 'large', label: tr('controllers.zone.large') }]

function Options<T extends string | boolean>({ id, label, sub, choices, value, onPick, locked }: {
  id: string; label: string; sub?: string; choices: Choice<T>[]; value: T; onPick: (v: T) => void; locked: boolean
}) {
  return (
    <div className="row">
      <span className="row-label">{label}{sub && <span className="row-sub">{sub}</span>}</span>
      {locked ? (
        <span className="row-value">{choices.find((c) => c.value === value)?.label}</span>
      ) : (
        <span className="choices">
          {choices.map((c) => (
            <button key={String(c.value)} className={`choice${c.value === value ? ' on' : ''}`} data-nav={`opt-${id}-${String(c.value)}`} onClick={() => onPick(c.value)} aria-pressed={c.value === value}>
              {c.label}
            </button>
          ))}
        </span>
      )}
    </div>
  )
}

function useProfile() {
  const { id = '' } = useParams()
  const cfg = usePadConfig()
  return { cfg, profile: findProfile(cfg, id) }
}

function Missing() {
  const t = useT()
  const navigate = useNavigate()
  return (
    <Page title={t('controllers.profile')}>
      <div className="empty-state">
        <h2>{t('controllers.missing.title')}</h2>
        <p>{t('controllers.missing.text')}</p>
        <button className="pill" data-nav="missing-back" data-nav-default="" onClick={() => navigate('/ajustes/mandos/perfiles', { replace: true })}>{t('controllers.missing.see')} <span className="pill-icon"><CaretRight size={18} weight="light" /></span></button>
      </div>
    </Page>
  )
}

export function ProfileDetail() {
  const t = useT()
  const navigate = useNavigate()
  const { cfg, profile: p } = useProfile()
  if (!p) return <Missing />
  const locked = !!p.builtIn
  const ps = padKind() === 'ps'
  const eff = effectiveButtons(p)
  const patch = (x: Partial<Profile>) => setConfig((c) => updateProfile(c, p.id, x))
  const copy = () => {
    const id = newId()
    setConfig((c) => duplicate(c, p.id, id))
    navigate(profilePath(id))
  }
  const isDefault = cfg.defaultId === p.id

  return (
    <Page title={p.name}>
      <div className="prof-actions">
        <button className="option" data-nav="prof-default" data-nav-default={isDefault ? undefined : ''} onClick={() => setConfig((c) => setDefault(c, p.id))} disabled={isDefault}>
          <span className="opt-line"><Star size={20} weight={isDefault ? 'fill' : 'light'} /> {isDefault ? t('controllers.isDefault') : t('controllers.makeDefault')}</span>
        </button>
        <button className="option" data-nav="prof-copy" data-nav-default={isDefault ? '' : undefined} onClick={copy}>
          <span className="opt-line"><Copy size={20} weight="light" /> {t('controllers.duplicate')}</span>
        </button>
        {!locked && (
          <>
            <button className="option" data-nav="prof-rename" onClick={() => navigate(`${profilePath(p.id)}/nombre`)}>
              <span className="opt-line"><PencilSimple size={20} weight="light" /> {t('controllers.rename')}</span>
            </button>
            <button className="option" data-nav="prof-delete" onClick={() => navigate(`${profilePath(p.id)}/borrar`)}>
              <span className="opt-line"><Trash size={20} weight="light" /> {t('controllers.delete')}</span>
            </button>
          </>
        )}
      </div>
      {locked && <p className="notice info">{t('controllers.lockedInfo')}</p>}

      <h3 className="label" style={{ marginTop: 32 }}>{t('controllers.buttons')}</h3>
      <div className="menu-list">
        {ACTIONS.map(({ action, label }) => locked ? (
          <div key={action} className="row">
            <span className="row-label">{label}</span>
            <span className="row-value">{buttonName(eff[action], ps)}</span>
          </div>
        ) : (
          <button key={action} className="menu-item" data-nav={`map-${action}`} onClick={() => navigate(`${profilePath(p.id)}/boton/${action}`)}>
            <GameController size={24} weight="light" />
            <span className="row-label">{label}</span>
            <span className="menu-item-value pad-key">{buttonName(eff[action], ps)}</span>
            <CaretRight size={18} weight="light" />
          </button>
        ))}
      </div>

      <h3 className="label" style={{ marginTop: 32 }}>{t('controllers.options')}</h3>
      <Options id="swap" label={t('controllers.opt.swap')} sub={t('controllers.opt.swapSub')} choices={YES_NO()} value={p.swapAB} onPick={(v) => patch({ swapAB: v })} locked={locked} />
      <Options id="repeat" label={t('controllers.opt.repeat')} sub={t('controllers.opt.repeatSub')} choices={SPEEDS()} value={p.repeat} onPick={(v) => patch({ repeat: v })} locked={locked} />
      <Options id="zone" label={t('controllers.opt.zone')} sub={t('controllers.opt.zoneSub')} choices={ZONES()} value={p.deadzone} onPick={(v) => patch({ deadzone: v })} locked={locked} />
      <Options id="invert" label={t('controllers.opt.invert')} sub={t('controllers.opt.invertSub')} choices={YES_NO()} value={p.invertY} onPick={(v) => patch({ invertY: v })} locked={locked} />
      <Options id="left" label={t('controllers.opt.left')} sub={t('controllers.opt.leftSub')} choices={YES_NO()} value={p.leftHanded} onPick={(v) => patch({ leftHanded: v })} locked={locked} />
    </Page>
  )
}

// ---------------- Cambiar nombre ----------------

export function ProfileName() {
  const t = useT()
  const navigate = useNavigate()
  const { profile: p } = useProfile()
  const [name, setName] = useState(p?.name ?? '')
  if (!p || p.builtIn) return <Missing />
  const save = (e: FormEvent) => {
    e.preventDefault()
    const clean = name.trim().slice(0, 40)
    if (!clean) return
    setConfig((c) => updateProfile(c, p.id, { name: clean }))
    sound.select()
    navigate(-1)
  }
  return (
    <Page title={t('controllers.rename')} hints={[['A', t('controllers.hint.type')], ['☰', t('controllers.hint.done')], ['B', t('common.back')]]}>
      <form className="name-form" onSubmit={save}>
        <label className="label" htmlFor="prof-name">{t('controllers.nameLabel')}</label>
        <input id="prof-name" className="pad-input" data-nav="name-input" data-nav-default="" value={name} maxLength={40} onChange={(e) => setName(e.target.value)} placeholder={t('controllers.nameLabel')} autoComplete="off" />
        <button className="btn-primary" type="submit" data-nav="name-save" disabled={!name.trim()}>{t('controllers.saveName')}</button>
      </form>
      <p className="notice info">{t('controllers.nameInfo')}</p>
    </Page>
  )
}

// ---------------- Borrar ----------------

export function ProfileDelete() {
  const t = useT()
  const navigate = useNavigate()
  const { profile: p } = useProfile()
  if (!p || p.builtIn) return <Missing />
  const remove = () => {
    setConfig((c) => removeProfile(c, p.id))
    sound.back()
    // Volver a la lista (saltando la ficha del perfil borrado)
    const idx = (window.history.state as { idx?: number } | null)?.idx ?? 0
    if (idx >= 2) navigate(-2)
    else navigate('/ajustes/mandos/perfiles', { replace: true })
  }
  return (
    <Page title={t('controllers.delete')}>
      <div className="confirm">
        <h2 className="confirm-title">{t('controllers.del.title', { name: p.name })}</h2>
        <p className="hero-meta">{t('controllers.del.text')}</p>
        <div className="hero-actions">
          <button className="btn-ghost" data-nav="del-keep" data-nav-default="" onClick={() => navigate(-1)}>{t('controllers.del.keep')}</button>
          <button className="btn-primary danger" data-nav="del-yes" onClick={remove}><Trash size={18} weight="light" /> {t('controllers.delete')}</button>
        </div>
      </div>
    </Page>
  )
}

// ---------------- Reasignar un botón ----------------

const WAIT_MS = 8000
const HOLD_TO_CANCEL = 1000

export function RemapCapture() {
  const t = useT()
  const navigate = useNavigate()
  const { action = '' } = useParams()
  const { profile: p } = useProfile()
  const [round, setRound] = useState(0)
  const [phase, setPhase] = useState<'wait' | 'holding' | 'timeout' | 'dpad'>('wait')
  const [holding, setHolding] = useState<number | null>(null)
  const ps = padKind() === 'ps'
  const meta = ACTIONS.find((a) => a.action === action)
  const pRef = useRef(p)
  pRef.current = p

  useEffect(() => {
    if (!p || p.builtIn || !meta) return
    lockNav('capture')
    let down: { button: number; at: number } | null = null
    let cancelTimer = 0
    const finish = () => { unlockNav('capture'); navigate(-1) }
    const timeout = window.setTimeout(() => { unlockNav('capture'); setPhase('timeout') }, WAIT_MS)
    const onButton = (e: Event) => {
      const b = (e as CustomEvent<PadButton>).detail
      if (b.pressed && !down) {
        down = { button: b.button, at: performance.now() }
        setHolding(b.button)
        setPhase('holding')
        // Mantener 1 s = salir sin cambiar nada (sirve aunque B sea lo que se está cambiando)
        cancelTimer = window.setTimeout(() => { sound.back(); finish() }, HOLD_TO_CANCEL)
      } else if (!b.pressed && down && b.button === down.button) {
        clearTimeout(cancelTimer)
        const button = down.button
        down = null
        if (button >= 12 && button <= 15) { setHolding(null); setPhase('dpad'); return }
        const cur = pRef.current!
        // Con A y B intercambiados se guarda en el otro hueco, para que haga lo que se ha pulsado
        const slot: Action = cur.swapAB && (action === 'accept' || action === 'back') ? (action === 'accept' ? 'back' : 'accept') : (action as Action)
        setConfig((c) => setButton(c, cur.id, slot, button))
        sound.select()
        finish()
      }
    }
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') { e.preventDefault(); sound.back(); finish() } }
    window.addEventListener('padbutton', onButton)
    window.addEventListener('keydown', onKey)
    return () => {
      clearTimeout(timeout)
      clearTimeout(cancelTimer)
      window.removeEventListener('padbutton', onButton)
      window.removeEventListener('keydown', onKey)
      unlockNav('capture')
    }
  }, [round, p?.id, action]) // eslint-disable-line react-hooks/exhaustive-deps

  if (!p || p.builtIn || !meta) return <Missing />
  const retry = () => { setPhase('wait'); setHolding(null); setRound((r) => r + 1) }

  return (
    <Page title={t('controllers.cap.title', { action: meta.label })} hints={phase === 'timeout' ? undefined : [['B', t('controllers.cap.holdHint')]]}>
      {phase === 'timeout' ? (
        <div className="capture">
          <h2 className="confirm-title">{t('controllers.cap.timeout')}</h2>
          <p className="hero-meta">{t('controllers.cap.timeoutText')}</p>
          <div className="hero-actions">
            <button className="btn-primary" data-nav="cap-retry" data-nav-default="" onClick={retry}>{t('controllers.cap.retry')}</button>
            <button className="btn-ghost" data-nav="cap-back" onClick={() => navigate(-1)}>{t('controllers.cap.back')}</button>
          </div>
        </div>
      ) : (
        <div className="capture">
          <span className="capture-ring" key={round}><GameController size={44} weight="light" /></span>
          <h2 className="confirm-title">
            {phase === 'holding' && holding !== null ? t('controllers.cap.release', { button: buttonName(holding, ps) }) : t('controllers.cap.press')}
          </h2>
          <p className="hero-meta">
            {phase === 'dpad' ? t('controllers.cap.dpad') : t('controllers.cap.exitInfo')}
          </p>
          <span className="capture-bar" key={`bar-${round}`} style={{ animationDuration: `${WAIT_MS}ms` }} />
          <button className="btn-ghost" data-nav="cap-cancel" onClick={() => { unlockNav('capture'); navigate(-1) }}>{t('controllers.cap.cancel')}</button>
        </div>
      )}
    </Page>
  )
}
