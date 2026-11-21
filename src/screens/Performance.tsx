import { useCallback, useEffect, useState, type ReactNode } from 'react'
import { useNavigate } from 'react-router-dom'
import {
  CaretRight, CheckCircle, FilmStrip, WindowsLogo, GameController, ArrowsClockwise, ShieldCheck,
  ArrowSquareOut, ArrowCounterClockwise, Lightning, PauseCircle, Clock, Memory,
} from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { call } from '../native'
import { useT } from '../i18n'
import { usePerf, setPerf } from '../data/perf'
import './ScreensaverSettings.css'
import './Performance.css'

// Ajustes → Rendimiento. Lo de Windows lo hace src-tauri/src/perf.rs; el movimiento, src/data/perf.ts.

type TweakId = 'gameMode' | 'recording' | 'backgroundApps' | 'transparency'
interface TweakState { id: TweakId; on: boolean; canUndo: boolean }
interface PerfStatus {
  tweaks: TweakState[]
  anyUndo: boolean
  gamePlan: boolean
  fastPlan: string | null
  acPower: boolean
  switched: boolean
  /** La optimización a fondo está aplicada (hay algo que deshacer) */
  optimized: boolean
}

const MOCK: PerfStatus = {
  tweaks: [
    { id: 'gameMode', on: true, canUndo: false },
    { id: 'recording', on: false, canUndo: false },
    { id: 'backgroundApps', on: false, canUndo: false },
    { id: 'transparency', on: false, canUndo: false },
  ],
  anyUndo: false,
  gamePlan: true,
  fastPlan: 'Alto rendimiento',
  acPower: true,
  switched: false,
  optimized: false,
}

function usePerfStatus() {
  const [s, setS] = useState<PerfStatus | null>(null)
  const [error, setError] = useState<string | null>(null)
  const run = useCallback((cmd: string, args: Record<string, unknown> = {}, mock: PerfStatus = MOCK) => {
    setError(null)
    return call<PerfStatus>(cmd, args, mock).then(setS).catch((e) => setError(String(e)))
  }, [])
  useEffect(() => { run('perf_status') }, [run])
  return { s, setS, error, run }
}

function Toggle({ id, on, onChange, isDefault }: { id: string; on: boolean; onChange: (v: boolean) => void; isDefault?: boolean }) {
  const t = useT()
  return (
    <button className={`option sv-toggle ${on ? 'is-on' : ''}`} data-nav={id} data-nav-default={isDefault ? '' : undefined} onClick={() => onChange(!on)} aria-pressed={on}>
      {on ? <><CheckCircle size={18} weight="fill" /> {t('performance.on')}</> : t('performance.off')}
    </button>
  )
}

function Row({ label, sub, children }: { label: string; sub?: string; children: ReactNode }) {
  return (
    <div className="row pf-row">
      <span className="row-label">{label}{sub && <span className="row-sub">{sub}</span>}</span>
      {children}
    </div>
  )
}

// ---------------- Menú ----------------

export function PerformanceMenu() {
  const t = useT()
  const navigate = useNavigate()
  const perf = usePerf()
  const { s } = usePerfStatus()
  const active = s ? s.tweaks.filter((x) => x.on).length : null
  const items = [
    { path: 'movimiento', label: t('performance.motion'), icon: FilmStrip, value: perf.reduceMotion ? t('performance.motion.reduced') : t('performance.motion.full') },
    { path: 'windows', label: t('performance.windows'), icon: WindowsLogo, value: active === null ? '' : t('performance.windows.count', { n: active, total: s!.tweaks.length }) },
    { path: 'juegos', label: t('performance.game'), icon: GameController, value: s ? (s.gamePlan ? t('performance.on') : t('performance.off')) : '' },
    { path: 'memoria', label: t('multitask.mem.title'), icon: Memory, value: '' }, // Multitask.tsx → MemorySettings
    { path: 'actualizaciones', label: t('performance.updates'), icon: ArrowsClockwise, value: '' },
    { path: 'avanzado', label: t('performance.advanced'), icon: ShieldCheck, value: s?.optimized ? t('performance.advanced.appliedShort') : t('performance.advanced.value') },
  ]
  return (
    <Page title={t('performance.title')}>
      <p className="hero-meta pf-intro">{t('performance.intro')}</p>
      <div className="menu-list">
        {items.map(({ path, label, icon: Icon, value }, i) => (
          <button key={path} className="menu-item" data-nav={`pf-${path}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => navigate(`/ajustes/rendimiento/${path}`)}>
            <Icon size={24} weight="light" />
            <span className="row-label">{label}</span>
            <span className="menu-item-value">{value}</span>
            <CaretRight size={18} weight="light" />
          </button>
        ))}
      </div>
    </Page>
  )
}

// ---------------- Movimiento ----------------

export function PerformanceMotion() {
  const t = useT()
  const perf = usePerf()
  return (
    <Page title={t('performance.motion')}>
      <Row label={t('performance.motion.reduce')} sub={t('performance.motion.reduceSub')}>
        <Toggle id="pf-reduce" on={perf.reduceMotion} onChange={(v) => setPerf({ reduceMotion: v })} isDefault />
      </Row>
      <p className="notice info">{t('performance.motion.info')}</p>
    </Page>
  )
}

// ---------------- Ajustes rápidos de Windows ----------------

const TWEAK_ORDER: TweakId[] = ['gameMode', 'recording', 'backgroundApps', 'transparency']

export function PerformanceWindows() {
  const t = useT()
  const { s, error, run } = usePerfStatus()
  const set = (id: TweakId, on: boolean) =>
    run('perf_tweak_set', { id, on }, s ? { ...s, tweaks: s.tweaks.map((x) => (x.id === id ? { ...x, on, canUndo: on } : x)), anyUndo: on || s.anyUndo } : MOCK)
  return (
    <Page title={t('performance.windows')}>
      <p className="hero-meta pf-intro">{t('performance.windows.intro')}</p>
      {s === null && !error && <p className="hero-meta">{t('performance.reading')}</p>}
      {s && TWEAK_ORDER.map((id, i) => {
        const tw = s.tweaks.find((x) => x.id === id)
        if (!tw) return null
        return (
          <Row key={id} label={t(`performance.tweak.${id}`)} sub={t(`performance.tweak.${id}.sub`)}>
            <Toggle id={`pf-tw-${id}`} on={tw.on} onChange={(v) => set(id, v)} isDefault={i === 0} />
          </Row>
        )
      })}
      {s?.anyUndo && (
        <button className="option pf-action" data-nav="pf-undo" onClick={() => run('perf_undo_all', {}, MOCK)}>
          <span className="opt-line"><ArrowCounterClockwise size={20} weight="light" /> {t('performance.windows.undo')}</span>
        </button>
      )}
      <p className="notice info">{t('performance.windows.info')}</p>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

// ---------------- Energía al jugar ----------------

export function PerformanceGame() {
  const t = useT()
  const { s, error, run } = usePerfStatus()
  return (
    <Page title={t('performance.game')}>
      {s === null && !error && <p className="hero-meta">{t('performance.reading')}</p>}
      {s && (
        <>
          <Row label={t('performance.game.toggle')} sub={t('performance.game.toggleSub')}>
            <Toggle id="pf-game" on={s.gamePlan} onChange={(v) => run('perf_game_plan_set', { on: v }, { ...s, gamePlan: v })} isDefault />
          </Row>
          <div className="row pf-row">
            <span className="row-label">{t('performance.game.plan')}</span>
            <span className="row-value">{s.fastPlan ?? t('performance.game.noPlan')}</span>
          </div>
          {s.switched && <p className="notice info">{t('performance.game.nowFast')}</p>}
          {!s.acPower && <p className="notice info">{t('performance.game.battery')}</p>}
          {!s.fastPlan && <p className="notice info">{t('performance.game.noPlanInfo')}</p>}
        </>
      )}
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

// ---------------- Actualizaciones de Windows ----------------

export function PerformanceUpdates() {
  const t = useT()
  const [error, setError] = useState<string | null>(null)
  const open = (page: string) => call('perf_open_settings', { page }).catch((e) => setError(String(e)))
  return (
    <Page title={t('performance.updates')}>
      <p className="hero-meta pf-intro">{t('performance.updates.intro')}</p>
      <button className="option pf-action" data-nav="pf-pause" data-nav-default="" onClick={() => open('windowsupdate')}>
        <span className="opt-line"><PauseCircle size={20} weight="light" /> <span className="row-label">{t('performance.updates.pause')}<span className="row-sub">{t('performance.updates.pauseSub')}</span></span></span>
        <ArrowSquareOut size={18} weight="light" />
      </button>
      <button className="option pf-action" data-nav="pf-hours" onClick={() => open('windowsupdate-activehours')}>
        <span className="opt-line"><Clock size={20} weight="light" /> <span className="row-label">{t('performance.updates.hours')}<span className="row-sub">{t('performance.updates.hoursSub')}</span></span></span>
        <ArrowSquareOut size={18} weight="light" />
      </button>
      <p className="notice info">{t('performance.updates.advanced')}</p>
      <p className="notice info">{t('performance.updates.safe')}</p>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

// ---------------- Optimización a fondo (scripts/optimizar-consola.cmd, pide permiso) ----------------

const STEPS = ['telemetry', 'clutter', 'services', 'power', 'games', 'visual', 'antivirus', 'restart'] as const
const QUESTIONS = ['apps', 'updates'] as const

export function PerformanceAdvanced() {
  const t = useT()
  const { s } = usePerfStatus()
  const [msg, setMsg] = useState<{ kind: 'info' | 'error'; text: string } | null>(null)
  const run = (undo: boolean) => call('run_optimizer', { undo })
    .then(() => setMsg({ kind: 'info', text: t(undo ? 'performance.advanced.undoStarted' : 'performance.advanced.started') }))
    .catch((e) => setMsg({ kind: 'error', text: String(e) }))
  return (
    <Page title={t('performance.advanced')}>
      <p className="hero-meta pf-intro">{t('performance.advanced.intro')}</p>
      <section className="pf-card">
        <h3 className="pf-card-title">{t('performance.advanced.what')}</h3>
        <ul className="pf-steps">
          {STEPS.map((k) => <li key={k}>{t(`performance.advanced.step.${k}`)}</li>)}
        </ul>
        <h3 className="pf-card-title">{t('performance.advanced.questions')}</h3>
        <ul className="pf-steps">
          {QUESTIONS.map((k) => <li key={k}>{t(`performance.advanced.q.${k}`)}</li>)}
        </ul>
      </section>
      {s?.optimized && <p className="notice ok">{t('performance.advanced.applied')}</p>}
      <button className="option pf-action" data-nav="pf-optimize" data-nav-default="" onClick={() => run(false)}>
        <span className="opt-line"><Lightning size={20} weight="light" /> {t('performance.advanced.run')}</span>
        <ArrowSquareOut size={18} weight="light" />
      </button>
      <button className="option pf-action" data-nav="pf-optimize-undo" onClick={() => run(true)}>
        <span className="opt-line"><ArrowCounterClockwise size={20} weight="light" /> {t('performance.advanced.undo')}</span>
        <ArrowSquareOut size={18} weight="light" />
      </button>
      <p className="notice info">{t('performance.advanced.info')}</p>
      {msg && <p className={`notice ${msg.kind}`}>{msg.text}</p>}
    </Page>
  )
}
