// Ajustes → Escritorio organizado (/ajustes/escritorio-organizado/*)
// Los iconos del escritorio de Windows se ordenan solos en zonas. Cada parte tiene su ruta:
//   /                 interruptor, vista previa, tamaño, zonas, reglas y "volver al de antes"
//   /zona/:id         nombre, mostrar u ocultar, orden
//   /reglas           reglas propias (borrar)
//   /reglas/nueva     crear una regla
import { useEffect, useState, type FormEvent } from 'react'
import { Route, Routes, useNavigate, useParams } from 'react-router-dom'
import { ArrowClockwise, ArrowCounterClockwise, CaretRight, CheckCircle, ListChecks, Plus, ArrowUp, ArrowDown, Trash } from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { call } from '../native'
import { useT, t as tr, fmt } from '../i18n'
import './DeskZones.css'

// ---------------- Datos (deskzones.rs) ----------------

export interface Zone { id: string; name: string; hidden: boolean }
export interface Rule { kind: 'ext' | 'name'; value: string; zone: string }
export interface DeskConfig { enabled: boolean; frames: boolean; iconSize: number; zones: Zone[]; rules: Rule[]; restorePending: boolean }
export interface Rect { x: number; y: number; w: number; h: number }
export interface Frame { id: string; name: string; count: number; cols: number; rect: Rect }
export interface Frames { monitor: Rect; work: Rect; cellW: number; cellH: number; pad: number; header: number; fits: boolean; zones: Frame[] }
interface Status {
  enabled: boolean; desktopVisible: boolean; layer: 'off' | 'attached' | 'failed'; layerError: string | null; layerKind: string | null
  organizedAt: number | null; itemCount: number; fits: boolean; hasBackup: boolean; restorePending: boolean; lastError: string | null
}

const BASE = '/ajustes/escritorio-organizado'
const IDS = ['juegos', 'apps', 'carpetas', 'documentos', 'multimedia', 'comprimidos', 'otros']
const SIZES = [48, 72, 96]
const MOCK_CONFIG: DeskConfig = { enabled: true, frames: true, iconSize: 72, zones: IDS.map((id) => ({ id, name: '', hidden: false })), rules: [], restorePending: false }
const MOCK_STATUS: Status = { enabled: true, desktopVisible: false, layer: 'off', layerError: null, layerKind: null, organizedAt: null, itemCount: 0, fits: true, hasBackup: false, restorePending: false, lastError: null }
/** Ejemplo para verlo en el navegador, sin la app */
export const mockFrames = (): Frames => {
  const cell = { w: 112, h: 118 }
  const counts = [6, 9, 3, 5, 4, 2, 3]
  let x = 18
  let y = 18
  let rowH = 0
  const zones: Frame[] = []
  counts.forEach((count, i) => {
    const cols = Math.min(4, Math.max(2, count))
    const w = 20 + cols * cell.w
    const h = 58 + Math.ceil(count / cols) * cell.h
    if (x + w > 1902) { x = 18; y += rowH + 18; rowH = 0 }
    zones.push({ id: IDS[i], name: '', count, cols, rect: { x, y, w, h } })
    x += w + 18
    rowH = Math.max(rowH, h)
  })
  return { monitor: { x: 0, y: 0, w: 1920, h: 1080 }, work: { x: 0, y: 0, w: 1920, h: 1032 }, cellW: cell.w, cellH: cell.h, pad: 10, header: 38, fits: true, zones }
}

export const zoneName = (z: { id: string; name: string }) => z.name.trim() || tr(`deskzones.zone.${z.id}`)

/** Ajustes guardados al momento, con cada cambio */
function useDeskConfig() {
  const [c, setC] = useState<DeskConfig | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => { call<DeskConfig>('deskzones_get', {}, MOCK_CONFIG).then(setC).catch((e) => setError(String(e))) }, [])
  const update = (patch: Partial<DeskConfig>) => {
    if (!c) return
    const next = { ...c, ...patch }
    setC(next)
    setError(null)
    call<DeskConfig>('deskzones_set', { config: next }, next).then(setC).catch((e) => setError(String(e)))
  }
  return { c, setC, update, error, setError }
}

function Toggle({ id, on, onChange, isDefault, disabled }: { id: string; on: boolean; onChange: (v: boolean) => void; isDefault?: boolean; disabled?: boolean }) {
  const t = useT()
  return (
    <button className={`option dz-toggle ${on ? 'is-on' : ''}`} data-nav={id} data-nav-default={isDefault ? '' : undefined} aria-pressed={on} disabled={disabled} onClick={() => onChange(!on)}>
      {on ? <><CheckCircle size={18} weight="fill" /> {t('deskzones.on')}</> : t('deskzones.off')}
    </button>
  )
}

function Chips<T extends string | number>({ id, options, value, label, onPick }: { id: string; options: T[]; value: T; label: (o: T) => string; onPick: (o: T) => void }) {
  return (
    <div className="dz-chips" role="group">
      {options.map((o) => (
        <button key={String(o)} className={`dz-chip ${value === o ? 'is-on' : ''}`} data-nav={`${id}-${o}`} aria-pressed={value === o} onClick={() => onPick(o)}>
          {value === o && <CheckCircle size={16} weight="fill" />}
          {label(o)}
        </button>
      ))}
    </div>
  )
}

// ---------------- Vista previa ----------------

/** El monitor en pequeño con las zonas y un punto por icono */
export function ZonesPreview({ frames }: { frames: Frames | null }) {
  const t = useT()
  if (!frames) return <div className="dz-preview is-loading" />
  const { monitor: m, work } = frames
  const pct = (v: number, total: number) => `${(v / total) * 100}%`
  return (
    <div className="dz-preview" style={{ aspectRatio: `${m.w} / ${m.h}` }} role="img" aria-label={t('deskzones.preview')}>
      <div className="dz-preview-taskbar" style={{ top: pct(work.y + work.h - m.y, m.h), height: pct(m.h - work.h - (work.y - m.y), m.h) }} />
      {frames.zones.length === 0 && <p className="dz-preview-empty">{t('deskzones.preview.empty')}</p>}
      {frames.zones.map((z) => (
        <div key={z.id} className="dz-preview-zone" style={{ left: pct(z.rect.x - m.x, m.w), top: pct(z.rect.y - m.y, m.h), width: pct(z.rect.w, m.w), height: pct(z.rect.h, m.h) }}>
          <span className="dz-preview-title">{zoneName(z)}</span>
          {/* Un punto en el centro de cada hueco de icono, en su sitio exacto */}
          {Array.from({ length: z.count }, (_, i) => (
            <i key={i} className="dz-preview-dot" style={{
              left: pct(frames.pad + ((i % z.cols) + 0.5) * frames.cellW, z.rect.w),
              top: pct(frames.header + frames.pad + (Math.floor(i / z.cols) + 0.5) * frames.cellH, z.rect.h),
            }} />
          ))}
        </div>
      ))}
    </div>
  )
}

// ---------------- /ajustes/escritorio-organizado ----------------

function Main() {
  const t = useT()
  const navigate = useNavigate()
  const { c, setC, update, error, setError } = useDeskConfig()
  const [status, setStatus] = useState<Status | null>(null)
  const [frames, setFrames] = useState<Frames | null>(null)
  const [busy, setBusy] = useState(false)
  const [confirm, setConfirm] = useState(false)
  const [restored, setRestored] = useState(false)

  const refresh = () => {
    call<Status>('deskzones_status', {}, MOCK_STATUS).then(setStatus).catch(() => {})
    call<Frames>('deskzones_preview', {}, mockFrames()).then(setFrames).catch(() => {})
  }
  // La vista previa sigue a los ajustes; el estado se mira de vez en cuando (no hay animación)
  useEffect(refresh, [c?.iconSize, c?.zones, c?.rules, c?.enabled])
  useEffect(() => {
    const id = window.setInterval(() => call<Status>('deskzones_status', {}, MOCK_STATUS).then(setStatus).catch(() => {}), 4000)
    return () => clearInterval(id)
  }, [])

  if (!c) return <Page title={t('deskzones.title')}><p className="hero-meta">{error ?? t('deskzones.loading')}</p></Page>

  const reorganize = async () => {
    setBusy(true); setError(null)
    try { setStatus(await call<Status>('deskzones_organize', {}, { ...MOCK_STATUS, organizedAt: Date.now() / 1000, itemCount: 32 })); refresh() } catch (e) { setError(String(e)) } finally { setBusy(false) }
  }
  const restore = async () => {
    if (!confirm) return setConfirm(true) // pulsar dos veces = confirmar
    setConfirm(false); setError(null)
    try {
      setStatus(await call<Status>('deskzones_restore', {}, { ...MOCK_STATUS, enabled: false }))
      setRestored(true)
      call<DeskConfig>('deskzones_get', {}, { ...c, enabled: false }).then(setC).catch(() => {})
    } catch (e) { setError(String(e)) }
  }

  const counts = new Map(frames?.zones.map((z) => [z.id, z.count]))
  const errorText = (e: string) => (e === 'no-desktop' ? t('deskzones.error.noDesktop') : e)

  return (
    <Page title={t('deskzones.title')}>
      <div className="row">
        <span className="row-label">
          {t('deskzones.toggle')}
          <span className="row-sub">{t('deskzones.toggle.sub')}</span>
        </span>
        <Toggle id="dz-on" on={c.enabled} onChange={(enabled) => { setRestored(false); update({ enabled }) }} isDefault />
      </div>

      <div className="dz-status">
        {status && !status.desktopVisible && <p className="notice info">{c.restorePending ? t('deskzones.restore.pending') : t('deskzones.state.noDesktop')}</p>}
        {status?.desktopVisible && c.enabled && (
          <p className="hero-meta">
            {status.organizedAt ? t('deskzones.state.done', { when: fmt.relative(new Date(status.organizedAt * 1000)), n: status.itemCount }) : t('deskzones.state.waiting')}
          </p>
        )}
        {status?.desktopVisible && c.enabled && !status.fits && status.organizedAt && <p className="notice info">{t('deskzones.state.noFit')}</p>}
        {c.enabled && c.frames && status?.layer === 'failed' && (
          <p className="notice error">{t('deskzones.layer.failed', { reason: t(`deskzones.reason.${(status.layerError ?? 'attach-failed').split(':')[0]}`) })}</p>
        )}
        {restored && <p className="notice ok">{t('deskzones.restore.done')}</p>}
      </div>

      <h3 className="label dz-label">{t('deskzones.preview')}</h3>
      <ZonesPreview frames={frames} />
      <div className="dz-actions">
        <button className="pill pill-dark" data-nav="dz-now" disabled={busy || !c.enabled} onClick={reorganize}>
          {busy ? t('deskzones.reorganizing') : t('deskzones.reorganize')}
          <span className="pill-icon"><ArrowClockwise size={14} weight="bold" /></span>
        </button>
      </div>

      <h3 className="label dz-label">{t('deskzones.size')}</h3>
      <Chips id="dz-size" options={SIZES} value={c.iconSize} label={(s) => t(`deskzones.size.${s}`)} onPick={(iconSize) => update({ iconSize })} />

      <div className="row">
        <span className="row-label">{t('deskzones.frames')}<span className="row-sub">{t('deskzones.frames.sub')}</span></span>
        <Toggle id="dz-frames" on={c.frames} onChange={(frames) => update({ frames })} />
      </div>

      <h3 className="label dz-label">{t('deskzones.zones')}</h3>
      {c.zones.map((z) => (
        <button key={z.id} className="option" data-nav={`dz-zone-${z.id}`} onClick={() => navigate(`${BASE}/zona/${z.id}`)}>
          <span className="row-label">
            {zoneName(z)}
            <span className="row-sub">{z.hidden ? t('deskzones.zones.hidden') : t('deskzones.zone.count', { n: counts.get(z.id) ?? 0 })}</span>
          </span>
          <CaretRight size={18} weight="light" />
        </button>
      ))}
      <button className="option" data-nav="dz-rules" onClick={() => navigate(`${BASE}/reglas`)}>
        <span className="dz-option-main">
          <ListChecks size={22} weight="light" />
          <span className="row-label">{t('deskzones.rules')}<span className="row-sub">{c.rules.length ? t('deskzones.rules.count', { n: c.rules.length }) : t('deskzones.rules.sub')}</span></span>
        </span>
        <CaretRight size={18} weight="light" />
      </button>

      <h3 className="label dz-label">{t('deskzones.restore')}</h3>
      <button className="option" data-nav="dz-restore" disabled={!status?.hasBackup && !c.restorePending} onClick={restore}>
        <span className="dz-option-main">
          <ArrowCounterClockwise size={22} weight="light" />
          <span className="row-label">{t('deskzones.restore')}<span className="row-sub">{t('deskzones.restore.sub')}</span></span>
        </span>
        {confirm && <span className="option-check dz-warn">{t('deskzones.restore.confirm')}</span>}
      </button>
      <p className="hero-meta dz-foot">{t('deskzones.safe')}</p>
      {error && <p className="notice error">{errorText(error)}</p>}
    </Page>
  )
}

// ---------------- /ajustes/escritorio-organizado/zona/:id ----------------

function ZoneEdit() {
  const t = useT()
  const { id = '' } = useParams()
  const { c, update, error } = useDeskConfig()
  const [name, setName] = useState<string | null>(null)
  const [saved, setSaved] = useState(false)
  if (!c) return <Page title={t('deskzones.title')}><p className="hero-meta">{error ?? t('deskzones.loading')}</p></Page>
  const idx = c.zones.findIndex((z) => z.id === id)
  const zone = c.zones[idx]
  if (!zone) return <Page title={t('deskzones.title')}><p className="hero-meta">{t('deskzones.loading')}</p></Page>
  const value = name ?? zone.name
  const setZone = (patch: Partial<Zone>) => update({ zones: c.zones.map((z) => (z.id === id ? { ...z, ...patch } : z)) })
  const move = (d: -1 | 1) => {
    const j = idx + d
    if (j < 0 || j >= c.zones.length) return
    const zones = [...c.zones]
    ;[zones[idx], zones[j]] = [zones[j], zones[idx]]
    update({ zones })
  }
  const save = (e: FormEvent) => { e.preventDefault(); setZone({ name: value.trim() }); setSaved(true) }
  const fallback = id === 'otros'

  return (
    <Page title={zoneName(zone)}>
      <h3 className="label">{t('deskzones.edit.name')}</h3>
      <form className="dz-field" onSubmit={save}>
        <input
          className="dz-input"
          data-nav="dz-name"
          data-nav-default=""
          value={value}
          maxLength={40}
          onChange={(e) => { setName(e.target.value); setSaved(false) }}
          placeholder={tr(`deskzones.zone.${id}`)}
          aria-label={t('deskzones.edit.name')}
          autoComplete="off"
          spellCheck={false}
        />
        <button className="pill pill-dark" type="submit" data-nav="dz-name-save">{saved ? <><CheckCircle size={16} weight="fill" /> {t('deskzones.edit.saved')}</> : t('deskzones.edit.save')}</button>
      </form>
      {zone.name && (
        <button className="option" data-nav="dz-name-reset" onClick={() => { setName(''); setZone({ name: '' }) }}>
          {t('deskzones.edit.reset', { name: tr(`deskzones.zone.${id}`) })}
        </button>
      )}

      <div className="row">
        <span className="row-label">{t('deskzones.edit.show')}<span className="row-sub">{fallback ? t('deskzones.edit.fallback') : t('deskzones.edit.show.sub')}</span></span>
        <Toggle id="dz-show" on={!zone.hidden} disabled={fallback} onChange={(on) => setZone({ hidden: !on })} />
      </div>

      <h3 className="label dz-label">{t('deskzones.edit.position', { pos: idx + 1, total: c.zones.length })}</h3>
      <div className="dz-chips">
        <button className="dz-chip" data-nav="dz-up" disabled={idx === 0} onClick={() => move(-1)}><ArrowUp size={16} weight="light" /> {t('deskzones.edit.up')}</button>
        <button className="dz-chip" data-nav="dz-down" disabled={idx === c.zones.length - 1} onClick={() => move(1)}><ArrowDown size={16} weight="light" /> {t('deskzones.edit.down')}</button>
      </div>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

// ---------------- /ajustes/escritorio-organizado/reglas ----------------

const ruleText = (r: Rule, zones: Zone[]) => {
  const z = zones.find((x) => x.id === r.zone) ?? { id: r.zone, name: '' }
  return tr(r.kind === 'ext' ? 'deskzones.rule.ext' : 'deskzones.rule.name', { value: r.kind === 'ext' ? r.value.replace(/^\./, '') : r.value, zone: zoneName(z) })
}

function Rules() {
  const t = useT()
  const navigate = useNavigate()
  const { c, update, error } = useDeskConfig()
  const [confirm, setConfirm] = useState<number | null>(null)
  if (!c) return <Page title={t('deskzones.rules')}><p className="hero-meta">{error ?? t('deskzones.loading')}</p></Page>
  const remove = (i: number) => {
    if (confirm !== i) return setConfirm(i)
    setConfirm(null)
    update({ rules: c.rules.filter((_, k) => k !== i) })
  }
  return (
    <Page title={t('deskzones.rules')}>
      <p className="hero-meta">{t('deskzones.rules.sub')}</p>
      <button className="option" data-nav="dz-rule-new" data-nav-default="" onClick={() => navigate(`${BASE}/reglas/nueva`)}>
        <span className="dz-option-main"><Plus size={22} weight="light" /><span className="row-label">{t('deskzones.rules.new')}</span></span>
        <CaretRight size={18} weight="light" />
      </button>
      {c.rules.length === 0 && <p className="notice info">{t('deskzones.rules.none')}</p>}
      {c.rules.map((r, i) => (
        <button key={`${r.kind}-${r.value}-${i}`} className="option" data-nav={`dz-rule-${i}`} onClick={() => remove(i)}>
          <span className="row-label">{ruleText(r, c.zones)}</span>
          <span className={`option-check ${confirm === i ? 'dz-warn' : 'dz-muted'}`}><Trash size={16} weight="light" /> {confirm === i ? t('deskzones.rule.confirm') : t('deskzones.rule.delete')}</span>
        </button>
      ))}
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

// ---------------- /ajustes/escritorio-organizado/reglas/nueva ----------------

function RuleNew() {
  const t = useT()
  const navigate = useNavigate()
  const { c, update, error } = useDeskConfig()
  const [kind, setKind] = useState<Rule['kind']>('ext')
  const [value, setValue] = useState('')
  const [zone, setZone] = useState('documentos')
  if (!c) return <Page title={t('deskzones.rules.new')}><p className="hero-meta">{error ?? t('deskzones.loading')}</p></Page>
  const clean = kind === 'ext' ? value.trim().replace(/^\.+/, '').toLowerCase() : value.trim()
  const save = (e?: FormEvent) => {
    e?.preventDefault()
    if (!clean) return
    update({ rules: [...c.rules, { kind, value: clean, zone }] })
    navigate(-1)
  }
  return (
    <Page title={t('deskzones.rules.new')}>
      <h3 className="label">{t('deskzones.rule.kind')}</h3>
      <Chips id="dz-kind" options={['ext', 'name'] as Rule['kind'][]} value={kind} label={(k) => t(`deskzones.rule.kind.${k}`)} onPick={setKind} />
      <h3 className="label dz-label">{t('deskzones.rule.value')}</h3>
      <form className="dz-field" onSubmit={save}>
        <input
          className="dz-input"
          data-nav="dz-value"
          data-nav-default=""
          value={value}
          maxLength={60}
          onChange={(e) => setValue(e.target.value)}
          placeholder={t(`deskzones.rule.value.${kind}`)}
          aria-label={t('deskzones.rule.value')}
          autoComplete="off"
          spellCheck={false}
        />
      </form>
      <h3 className="label dz-label">{t('deskzones.rule.zone')}</h3>
      <Chips id="dz-target" options={c.zones.map((z) => z.id)} value={zone} label={(id) => zoneName(c.zones.find((z) => z.id === id)!)} onPick={setZone} />
      {clean && <p className="hero-meta dz-foot">{ruleText({ kind, value: clean, zone }, c.zones)}</p>}
      <div className="dz-actions">
        <button className="pill pill-dark" data-nav="dz-rule-save" disabled={!clean} onClick={() => save()}>
          {t('deskzones.rule.save')}
          <span className="pill-icon"><CheckCircle size={14} weight="fill" /></span>
        </button>
      </div>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

/** Todas las pantallas de /ajustes/escritorio-organizado/* */
export function DeskZones() {
  return (
    <Routes>
      <Route index element={<Main />} />
      <Route path="zona/:id" element={<ZoneEdit />} />
      <Route path="reglas" element={<Rules />} />
      <Route path="reglas/nueva" element={<RuleNew />} />
    </Routes>
  )
}
