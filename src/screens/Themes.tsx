import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import {
  CheckCircle, Plus, DownloadSimple, MusicNotes, PencilSimple, Copy, ShareNetwork, Trash,
  ClipboardText, FileArrowUp, FloppyDisk, ImageSquare, CaretRight,
} from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { Stepper } from '../components/Stepper'
import { t as tr, useT } from '../i18n'
import {
  BUILTINS, MUSIC_PRESETS, activeTheme, blankTheme, deleteTheme, duplicateOf, exportCode, findTheme, getDraft,
  importCode, saveTheme, setActiveTheme, setDraft, themeVars, useDraft, useThemes,
  type BuiltinId, type Corners, type FontSet, type MusicChoice, type Theme,
} from '../data/themes'
import { getBackdrop, parseYouTube, isHttpUrl, type BackdropChoice } from '../data/backdrop'
import { setMusicSettings, useMusicSettings } from '../audio/menuMusic'
import { isHex, normHex } from '../data/themePalettes'
import './Themes.css'

const themeName = (t: Theme, tr: (k: string) => string) => (t.builtIn ? tr(t.name) : t.name)

/** Variables del tema para una miniatura (sin color-scheme, que no es una variable) */
function previewStyle(t: Theme): CSSProperties {
  const { 'color-scheme': _cs, ...vars } = themeVars(t) // eslint-disable-line @typescript-eslint/no-unused-vars
  return vars as CSSProperties
}

/** Miniatura viva del tema: fondo, barra con la marca, una tarjeta y el acento */
export function ThemePreview({ theme, big = false }: { theme: Theme; big?: boolean }) {
  return (
    <div className={`th-prev ${big ? 'is-big' : ''}`} style={previewStyle(theme)} aria-hidden>
      <span className="th-prev-glow" />
      <span className="th-prev-bar"><span className="th-prev-mark" /><span className="th-prev-title">Aa</span></span>
      <span className="th-prev-card">
        <span className="th-prev-line" />
        <span className="th-prev-line is-short" />
        <span className="th-prev-pill" />
      </span>
      <span className="th-prev-focus" />
    </div>
  )
}

// ---------------- /ajustes/temas ----------------

export function ThemesScreen() {
  const t = useT()
  const navigate = useNavigate()
  const { all, active } = useThemes()
  const music = useMusicSettings()
  const current = findTheme(active) ?? BUILTINS[0]

  const duplicate = () => {
    setDraft(duplicateOf(current, t('themes.copyName', { name: themeName(current, t) })))
    navigate('/ajustes/temas/nuevo')
  }

  return (
    <Page wide title={t('themes.title')}>
      <p className="hero-meta th-intro">{t('themes.intro')}</p>
      <div className="th-grid">
        {all.map((theme, i) => {
          const on = theme.id === active
          return (
            <button
              key={theme.id}
              className={`th-card ${on ? 'is-on' : ''}`}
              data-nav={`th-card-${theme.id}`}
              data-nav-default={i === 0 ? '' : undefined}
              aria-pressed={on}
              onClick={() => setActiveTheme(theme.id)}
            >
              <ThemePreview theme={theme} />
              <span className="th-card-name">
                {themeName(theme, t)}
                {on && <span className="th-card-on"><CheckCircle size={16} weight="fill" /> {t('themes.inUse')}</span>}
              </span>
              <span className="th-card-sub">{theme.builtIn ? t(`themes.builtin.${theme.id}.sub`) : t('themes.custom')}</span>
            </button>
          )
        })}
      </div>

      <h3 className="label th-label">{t('themes.current', { name: themeName(current, t) })}</h3>
      <div className="th-actions">
        {!current.builtIn && (
          <ActionPill id="th-edit" icon={<PencilSimple size={16} weight="light" />} onClick={() => { setDraft(null); navigate(`/ajustes/temas/${current.id}/editar`) }}>
            {t('themes.edit')}
          </ActionPill>
        )}
        <ActionPill id="th-dup" icon={<Copy size={16} weight="light" />} onClick={duplicate}>{t('themes.duplicate')}</ActionPill>
        <ActionPill id="th-share" icon={<ShareNetwork size={16} weight="light" />} onClick={() => navigate(`/ajustes/temas/${current.id}/compartir`)}>
          {t('themes.share')}
        </ActionPill>
        {!current.builtIn && (
          <ActionPill id="th-del" icon={<Trash size={16} weight="light" />} onClick={() => navigate(`/ajustes/temas/${current.id}/borrar`)}>
            {t('themes.delete')}
          </ActionPill>
        )}
      </div>

      <h3 className="label th-label">{t('themes.more')}</h3>
      <div className="menu-list">
        <button className="menu-item" data-nav="th-new" onClick={() => { setDraft(null); navigate('/ajustes/temas/nuevo') }}>
          <Plus size={24} weight="light" />
          <span className="row-label">{t('themes.create')}<span className="row-sub">{t('themes.create.sub')}</span></span>
          <CaretRight size={18} weight="light" />
        </button>
        <button className="menu-item" data-nav="th-import" onClick={() => navigate('/ajustes/temas/importar')}>
          <DownloadSimple size={24} weight="light" />
          <span className="row-label">{t('themes.import')}<span className="row-sub">{t('themes.import.sub')}</span></span>
          <CaretRight size={18} weight="light" />
        </button>
        <button className="menu-item" data-nav="th-music" onClick={() => navigate('/ajustes/temas/musica')}>
          <MusicNotes size={24} weight="light" />
          <span className="row-label">{t('themes.music')}<span className="row-sub">{t('themes.music.sub')}</span></span>
          <span className="menu-item-value">{music.enabled ? `${music.volume}%` : t('themes.off')}</span>
          <CaretRight size={18} weight="light" />
        </button>
      </div>
    </Page>
  )
}

function ActionPill({ id, icon, onClick, children, dark }: { id: string; icon: ReactNode; onClick: () => void; children: ReactNode; dark?: boolean }) {
  return (
    <button className={`pill ${dark ? 'pill-dark' : ''}`} data-nav={id} onClick={onClick}>
      {children}
      <span className="pill-icon">{icon}</span>
    </button>
  )
}

// ---------------- /ajustes/temas/nuevo · /ajustes/temas/:id/editar ----------------

const SWATCHES = ['#E8643A', '#D9822B', '#E3A35C', '#C9A227', '#6F8F5E', '#3E8A7A', '#2F6F8F', '#B5543A', '#C77D7D', '#8C7F72']
const FONT_SETS: FontSet[] = ['editorial', 'moderna', 'tecnica']
const CORNERS: Corners[] = ['suave', 'redondo', 'recto']
const ANIMATED: BackdropChoice[] = (['ondas', 'aurora', 'particulas', 'plano'] as const).map((style) => ({ kind: 'animated', style }))
const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b)

function Chip({ id, on, onPick, children, style }: { id: string; on: boolean; onPick: () => void; children: ReactNode; style?: CSSProperties }) {
  return (
    <button className={`th-chip ${on ? 'is-on' : ''}`} data-nav={id} aria-pressed={on} onClick={onPick} style={style}>
      {on && <CheckCircle size={16} weight="fill" />}
      {children}
    </button>
  )
}

export function ThemeEditor() {
  const t = useT()
  const navigate = useNavigate()
  const { id } = useParams()
  const draft = useDraft()
  const [hex, setHex] = useState('')
  const [musicUrl, setMusicUrl] = useState('')
  const [error, setError] = useState<string | null>(null)

  // Al entrar: copia del tema a editar, o uno nuevo basado en el que se usa ahora
  useEffect(() => {
    const d = getDraft()
    if (id) {
      if (d?.id === id) return
      const theme = findTheme(id)
      if (theme && !theme.builtIn) setDraft({ ...theme })
      else navigate('/ajustes/temas', { replace: true })
    } else if (!d || findTheme(d.id)) {
      const base = activeTheme()
      setDraft(blankTheme(t('themes.newName'), base.base))
    }
  }, [id]) // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!draft) return
    setHex(draft.accent ?? '')
    setMusicUrl(draft.music.kind === 'url' || draft.music.kind === 'youtube' ? draft.music.url : '')
  }, [draft?.id]) // eslint-disable-line react-hooks/exhaustive-deps

  if (!draft) return <Page title={t('themes.create')}><p className="hero-meta">…</p></Page>

  const patch = (p: Partial<Theme>) => { setError(null); setDraft({ ...draft, ...p }) }
  const pickBase = (base: BuiltinId) => {
    const b = BUILTINS.find((x) => x.id === base)!
    patch({ base, mode: b.mode, backdrop: b.backdrop, music: draft.music.kind === 'preset' ? b.music : draft.music })
  }
  const applyHex = (text: string) => {
    setHex(text)
    if (isHex(text)) patch({ accent: normHex(text) })
  }
  const applyMusicUrl = (text: string) => {
    setMusicUrl(text)
    const yt = parseYouTube(text)
    if (yt) patch({ music: { kind: 'youtube', url: text.trim(), ...yt } })
    else if (isHttpUrl(text)) patch({ music: { kind: 'url', url: text.trim() } })
  }
  const save = () => {
    if (!draft.name.trim()) return setError(t('themes.error.name'))
    if (hex && !isHex(hex)) return setError(t('themes.error.hex'))
    const saved = saveTheme(draft)
    setActiveTheme(saved.id)
    setDraft(null)
    navigate('/ajustes/temas', { replace: true })
  }

  const musicChoices: { key: string; label: string; value: MusicChoice }[] = [
    ...MUSIC_PRESETS.map((p) => ({ key: p, label: t(`themes.music.${p}`), value: { kind: 'preset', preset: p } as MusicChoice })),
    { key: 'off', label: t('themes.music.none'), value: { kind: 'off' } },
  ]
  const currentBackdrop = getBackdrop().choice
  const ownMusic = draft.music.kind === 'url' || draft.music.kind === 'youtube'

  return (
    <Page title={id ? t('themes.editTitle') : t('themes.create')}>
      <div className="th-editor-head">
        <ThemePreview theme={draft} big />
        <label className="th-field">
          <span className="label">{t('themes.name')}</span>
          <input
            className="th-input"
            data-nav="th-name"
            data-nav-default=""
            value={draft.name}
            maxLength={40}
            onChange={(e) => patch({ name: e.target.value })}
            placeholder={t('themes.name.placeholder')}
            aria-label={t('themes.name')}
            autoComplete="off"
            spellCheck={false}
          />
        </label>
      </div>

      <h3 className="label th-label">{t('themes.base')}</h3>
      <div className="th-chips">
        {BUILTINS.map((b) => (
          <Chip key={b.id} id={`th-base-${b.id}`} on={draft.base === b.id} onPick={() => pickBase(b.base)}>{t(b.name)}</Chip>
        ))}
      </div>

      <h3 className="label th-label">{t('themes.mode')}</h3>
      <div className="th-chips">
        <Chip id="th-mode-light" on={draft.mode === 'light'} onPick={() => patch({ mode: 'light' })}>{t('themes.mode.light')}</Chip>
        <Chip id="th-mode-dark" on={draft.mode === 'dark'} onPick={() => patch({ mode: 'dark' })}>{t('themes.mode.dark')}</Chip>
      </div>

      <h3 className="label th-label">{t('themes.accent')}</h3>
      <div className="th-swatches">
        <button className={`th-swatch is-auto ${draft.accent === null ? 'is-on' : ''}`} data-nav="th-acc-auto" aria-pressed={draft.accent === null}
          aria-label={t('themes.accent.base')} onClick={() => { setHex(''); patch({ accent: null }) }}>
          {t('themes.accent.base')}
        </button>
        {SWATCHES.map((c) => (
          <button key={c} className={`th-swatch ${draft.accent === c ? 'is-on' : ''}`} data-nav={`th-acc-${c.slice(1)}`} aria-pressed={draft.accent === c}
            aria-label={c} style={{ ['--sw' as string]: c }} onClick={() => { setHex(c); patch({ accent: c }) }} />
        ))}
      </div>
      <label className="th-field th-hex">
        <span className="row-sub">{t('themes.accent.hex')}</span>
        <input className="th-input mono" data-nav="th-hex" value={hex} maxLength={7} placeholder="#E8643A"
          onChange={(e) => applyHex(e.target.value)} aria-label={t('themes.accent.hex')} autoComplete="off" spellCheck={false} />
      </label>

      <h3 className="label th-label">{t('themes.font')}</h3>
      <div className="th-chips">
        {FONT_SETS.map((f) => (
          <Chip key={f} id={`th-font-${f}`} on={draft.font === f} onPick={() => patch({ font: f })}>
            <span className={`th-font-sample is-${f}`}>Aa</span> {t(`themes.font.${f}`)}
          </Chip>
        ))}
      </div>

      <h3 className="label th-label">{t('themes.corners')}</h3>
      <div className="th-chips">
        {CORNERS.map((c) => (
          <Chip key={c} id={`th-corner-${c}`} on={draft.corners === c} onPick={() => patch({ corners: c })}>
            <span className={`th-corner-sample is-${c}`} /> {t(`themes.corners.${c}`)}
          </Chip>
        ))}
      </div>

      <h3 className="label th-label">{t('themes.background')}</h3>
      <div className="th-chips">
        <Chip id="th-bd-keep" on={draft.backdrop === null} onPick={() => patch({ backdrop: null })}>{t('themes.background.keep')}</Chip>
        {ANIMATED.map((b) => b.kind === 'animated' && (
          <Chip key={b.style} id={`th-bd-${b.style}`} on={same(draft.backdrop, b)} onPick={() => patch({ backdrop: b })}>{t(`themes.background.${b.style}`)}</Chip>
        ))}
        {currentBackdrop.kind !== 'animated' && (
          <Chip id="th-bd-current" on={same(draft.backdrop, currentBackdrop)} onPick={() => patch({ backdrop: currentBackdrop })}>{t('themes.background.current')}</Chip>
        )}
      </div>
      <button className="option th-link" data-nav="th-bd-pick" onClick={() => navigate('/ajustes/fondo')}>
        <span className="th-link-main"><ImageSquare size={22} weight="light" />
          <span className="row-label">{t('themes.background.pick')}<span className="row-sub">{t('themes.background.pick.sub')}</span></span>
        </span>
        <CaretRight size={18} weight="light" />
      </button>

      <h3 className="label th-label">{t('themes.music')}</h3>
      <div className="th-chips">
        {musicChoices.map((m) => (
          <Chip key={m.key} id={`th-mu-${m.key}`} on={same(draft.music, m.value)} onPick={() => patch({ music: m.value })}>{m.label}</Chip>
        ))}
        <Chip id="th-mu-own" on={ownMusic} onPick={() => applyMusicUrl(musicUrl)}>{t('themes.music.own')}</Chip>
      </div>
      <label className="th-field th-url">
        <span className="row-sub">{t('themes.music.url')}</span>
        <input className="th-input" data-nav="th-mu-url" value={musicUrl} placeholder="https://…"
          onChange={(e) => applyMusicUrl(e.target.value)} aria-label={t('themes.music.url')} autoComplete="off" spellCheck={false} />
      </label>
      <p className="hero-meta th-note">{t('themes.music.hint')}</p>

      <div className="th-save">
        <ActionPill id="th-save" dark icon={<FloppyDisk size={16} weight="light" />} onClick={save}>{t('themes.save')}</ActionPill>
      </div>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

// ---------------- /ajustes/temas/:id/borrar ----------------

export function ThemeDelete() {
  const t = useT()
  const navigate = useNavigate()
  const { id = '' } = useParams()
  const theme = findTheme(id)
  if (!theme || theme.builtIn) return <Page title={t('themes.delete')}><p className="hero-meta">{t('themes.notFound')}</p></Page>
  return (
    <Page title={t('themes.delete')}>
      <div className="th-confirm">
        <ThemePreview theme={theme} big />
        <h2 className="th-confirm-title">{t('themes.delete.ask', { name: theme.name })}</h2>
        <p className="hero-meta">{t('themes.delete.sub')}</p>
        <div className="th-actions">
          <button className="pill" data-nav="th-del-no" data-nav-default="" onClick={() => navigate(-1)}>{t('themes.cancel')}</button>
          <button className="pill pill-dark th-danger" data-nav="th-del-yes" onClick={() => { deleteTheme(id); navigate('/ajustes/temas', { replace: true }) }}>
            {t('themes.delete.confirm')}
            <span className="pill-icon"><Trash size={16} weight="light" /></span>
          </button>
        </div>
      </div>
    </Page>
  )
}

// ---------------- /ajustes/temas/:id/compartir ----------------

export function ThemeShare() {
  const t = useT()
  const { id = '' } = useParams()
  const theme = findTheme(id)
  const [copied, setCopied] = useState<'ok' | 'fail' | null>(null)
  if (!theme) return <Page title={t('themes.share')}><p className="hero-meta">{t('themes.notFound')}</p></Page>
  const code = exportCode(theme.builtIn ? { ...theme, name: t(theme.name), builtIn: false } : theme)

  const copy = () => navigator.clipboard.writeText(code).then(() => setCopied('ok'), () => setCopied('fail'))
  const saveFile = () => {
    const blob = new Blob([JSON.stringify({ couchdesk: 'tema', code }, null, 2)], { type: 'application/json' })
    const a = document.createElement('a')
    a.href = URL.createObjectURL(blob)
    a.download = `${(theme.builtIn ? t(theme.name) : theme.name).replace(/[^\p{L}\p{N} _-]/gu, '') || 'tema'}.couchdesk.json`
    a.click()
    setTimeout(() => URL.revokeObjectURL(a.href), 2000)
  }

  return (
    <Page title={t('themes.share')}>
      <p className="hero-meta th-intro">{t('themes.share.sub')}</p>
      <div className="th-code mono">{code}</div>
      <div className="th-actions">
        <ActionPill id="th-copy" dark icon={<ClipboardText size={16} weight="light" />} onClick={copy}>{t('themes.share.copy')}</ActionPill>
        <ActionPill id="th-file" icon={<FloppyDisk size={16} weight="light" />} onClick={saveFile}>{t('themes.share.file')}</ActionPill>
      </div>
      {copied === 'ok' && <p className="notice ok">{t('themes.share.copied')}</p>}
      {copied === 'fail' && <p className="notice error">{t('themes.share.copyFail')}</p>}
    </Page>
  )
}

// ---------------- /ajustes/temas/importar ----------------

export function ThemeImport() {
  const t = useT()
  const navigate = useNavigate()
  const [text, setText] = useState('')
  const [error, setError] = useState<string | null>(null)
  const fileRef = useRef<HTMLInputElement>(null)

  const take = (raw: string) => {
    let value = raw
    try { const j = JSON.parse(raw); if (typeof j?.code === 'string') value = j.code } catch { /* código suelto */ }
    const theme = importCode(value)
    if (!theme) return setError(t('themes.import.bad'))
    const saved = saveTheme(theme)
    setActiveTheme(saved.id)
    navigate('/ajustes/temas', { replace: true })
  }
  const paste = () => navigator.clipboard.readText().then((v) => { setText(v); take(v) }, () => setError(t('themes.import.noClipboard')))
  const onFile = (f: File | undefined) => { if (f) f.text().then(take, () => setError(t('themes.import.bad'))) }

  return (
    <Page title={t('themes.import')}>
      <p className="hero-meta th-intro">{t('themes.import.intro')}</p>
      <div className="th-actions">
        <ActionPill id="th-paste" dark icon={<ClipboardText size={16} weight="light" />} onClick={paste}>{t('themes.import.paste')}</ActionPill>
        <ActionPill id="th-open" icon={<FileArrowUp size={16} weight="light" />} onClick={() => fileRef.current?.click()}>{t('themes.import.file')}</ActionPill>
      </div>
      <input ref={fileRef} type="file" accept=".json,application/json" hidden onChange={(e) => onFile(e.target.files?.[0])} />
      <label className="th-field th-url">
        <span className="row-sub">{t('themes.import.code')}</span>
        <input className="th-input mono" data-nav="th-code" data-nav-default="" value={text} placeholder="COUCHDESK-TEMA-1:…"
          onChange={(e) => { setText(e.target.value); setError(null) }} aria-label={t('themes.import.code')} autoComplete="off" spellCheck={false} />
      </label>
      <div className="th-save">
        <ActionPill id="th-import-go" icon={<DownloadSimple size={16} weight="light" />} onClick={() => take(text)}>{t('themes.import.go')}</ActionPill>
      </div>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

/** Nombre del tema en uso (para el menú de Ajustes) */
export function themeLabel() {
  const theme = activeTheme()
  return theme.builtIn ? tr(theme.name) : theme.name
}

/** Ajustes → Sonido: enlace a la música del menú */
export function MenuMusicLink() {
  const t = useT()
  const navigate = useNavigate()
  const s = useMusicSettings()
  return (
    <button className="option th-link" data-nav="snd-menu-music" onClick={() => navigate('/ajustes/temas/musica')}>
      <span className="th-link-main"><MusicNotes size={22} weight="light" />
        <span className="row-label">{t('themes.music')}<span className="row-sub">{t('themes.music.sub')}</span></span>
      </span>
      <span className="menu-item-value">{s.enabled ? `${s.volume}%` : t('themes.off')}</span>
    </button>
  )
}

// ---------------- /ajustes/temas/musica ----------------

export function MenuMusicSettings() {
  const t = useT()
  const s = useMusicSettings()
  return (
    <Page title={t('themes.music')}>
      <div className="row">
        <span className="row-label">
          {t('themes.music.toggle')}
          <span className="row-sub">{t('themes.music.toggle.sub')}</span>
        </span>
        <button className={`option th-toggle ${s.enabled ? 'is-on' : ''}`} data-nav="mm-on" data-nav-default="" aria-pressed={s.enabled}
          onClick={() => setMusicSettings({ enabled: !s.enabled })}>
          {s.enabled ? <><CheckCircle size={18} weight="fill" /> {t('themes.on')}</> : t('themes.off')}
        </button>
      </div>
      <Stepper id="mm-vol" label={t('themes.music.volume')} value={s.volume} onChange={(volume) => setMusicSettings({ volume })} />
      <p className="notice info">{t('themes.music.when')}</p>
    </Page>
  )
}
