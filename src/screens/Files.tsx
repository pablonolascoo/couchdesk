import { useCallback, useEffect, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import {
  FolderSimple, File, HardDrives, ArrowUp, ClipboardText, FolderPlus, ArrowSquareOut,
  Copy, Scissors, Trash, CaretRight, Usb, ArrowLineDown, CheckSquare, Square, Checks,
} from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { call } from '../native'
import { useT, fmt } from '../i18n'
import {
  clipboard, useClipboard, mockPlaces, mockEntries, parentOf, nameOf, type Entry, type Place,
} from '../data/files'
import './Files.css'

// Modo «traer»: si la dirección lleva ?destino=, los archivos se marcan y se copian allí.
// Así cada paso tiene su propia URL y «atrás» no pierde nada.
type Bring = { dest: string; name: string } | null
const bringQuery = (b: Bring) => (b ? `&destino=${encodeURIComponent(b.dest)}&nombre=${encodeURIComponent(b.name)}` : '')
const folderUrl = (p: string, b: Bring = null) => `/archivos/carpeta?ruta=${encodeURIComponent(p)}${bringQuery(b)}`
const itemUrl = (p: string) => `/archivos/elemento?ruta=${encodeURIComponent(p)}`
/** Abre el selector para traer archivos a una carpeta (desde un USB u otra carpeta) */
export const bringUrl = (dest: string, name = nameOf(dest)) =>
  `/archivos?destino=${encodeURIComponent(dest)}&nombre=${encodeURIComponent(name)}`

function useBring(): Bring {
  const [params] = useSearchParams()
  const dest = params.get('destino')
  return dest ? { dest, name: params.get('nombre') ?? nameOf(dest) } : null
}

const placeIcon = (k: Place['kind']) => (k === 'usb' ? Usb : k === 'disco' ? HardDrives : FolderSimple)

// ---------- Lugares ----------

export function FilesHome() {
  const t = useT()
  const navigate = useNavigate()
  const bring = useBring()
  const [places, setPlaces] = useState<Place[] | null>(null)
  useEffect(() => { call<Place[]>('list_places', {}, mockPlaces()).then(setPlaces) }, [])
  return (
    <Page title={bring ? t('files.bringTitle', { name: bring.name }) : t('files.title')}>
      {bring && <p className="hero-meta bring-intro">{t('files.bringIntro')}</p>}
      <div className="menu-list">
        {places?.map((p, i) => {
          const Icon = placeIcon(p.kind)
          return (
            <button key={p.path} className="menu-item" data-nav={`place-${p.path}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => navigate(folderUrl(p.path, bring))}>
              <Icon size={24} weight={p.kind === 'usb' ? 'regular' : 'light'} />
              <span className="row-label">{p.label}</span>
              <span className="menu-item-value mono">{p.path}</span>
              <CaretRight size={18} weight="light" />
            </button>
          )
        })}
      </div>
    </Page>
  )
}

// ---------- Carpeta ----------

export function Folder() {
  const t = useT()
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const bring = useBring()
  const path = params.get('ruta') ?? 'C:\\'
  const [entries, setEntries] = useState<Entry[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [msg, setMsg] = useState<string | null>(null)
  const [picked, setPicked] = useState<Set<string>>(new Set())
  const [busy, setBusy] = useState(false)
  const clip = useClipboard()
  const parent = parentOf(path)

  const load = useCallback(() => {
    setError(null)
    setPicked(new Set())
    call<Entry[]>('list_dir', { path }, mockEntries(path)).then(setEntries).catch((e) => setError(String(e)))
  }, [path])
  useEffect(load, [load])

  const paste = async () => {
    if (!clip) return
    try {
      const n = await call<number>('paste_items', { items: clip.items, dest: path, cut: clip.cut }, clip.items.length)
      setMsg(t(clip.cut ? 'files.pastedMoved' : 'files.pastedCopied', { n }))
      if (clip.cut) clipboard.set(null)
      load()
    } catch (e) { setError(String(e)) }
  }
  const mkdir = () => call('new_folder', { dir: path }, '').then(() => { setMsg(t('files.folderCreated')); load() }).catch((e) => setError(String(e)))

  // --- Modo traer ---
  const files = entries?.filter((e) => !e.isDir) ?? []
  const toggle = (p: string) => setPicked((s) => { const n = new Set(s); if (n.has(p)) n.delete(p); else n.add(p); return n })
  const allPicked = files.length > 0 && files.every((f) => picked.has(f.path))
  const pickAll = () => setPicked(allPicked ? new Set() : new Set(files.map((f) => f.path)))
  const pickedSize = files.filter((f) => picked.has(f.path)).reduce((sum, f) => sum + (f.size ?? 0), 0)
  const bringHere = async (cut: boolean) => {
    if (!bring || picked.size === 0) return
    setBusy(true); setError(null); setMsg(null)
    try {
      const n = await call<number>('paste_items', { items: [...picked], dest: bring.dest, cut }, picked.size)
      setMsg(t(cut ? 'files.broughtMoved' : 'files.broughtCopied', { n, name: bring.name }))
      load()
    } catch (e) { setError(String(e)) }
    setBusy(false)
  }

  const hints: [string, string][] = bring ? [['A', t('files.pick')], ['B', t('common.back')]] : [['A', t('common.open')], ['B', t('common.back')]]

  return (
    <Page title={bring ? t('files.bringTitle', { name: bring.name }) : nameOf(path)} hints={hints}>
      <p className="path mono">{path}</p>
      <div className="toolbar">
        {parent && (
          <button className="option tool" data-nav="up" onClick={() => navigate(folderUrl(parent, bring))}>
            <ArrowUp size={18} weight="light" /> {t('files.up')}
          </button>
        )}
        {bring ? (
          <>
            {files.length > 0 && (
              <button className="option tool" data-nav="all" onClick={pickAll}>
                <Checks size={18} weight="light" /> {allPicked ? t('files.unpickAll') : t('files.pickAll')}
              </button>
            )}
            <button className="option tool" data-nav="see-dest" onClick={() => navigate(folderUrl(bring.dest))}>
              <FolderSimple size={18} weight="light" /> {t('files.seeDest', { name: bring.name })}
            </button>
          </>
        ) : (
          <>
            <button className="option tool" data-nav="bring" onClick={() => navigate(bringUrl(path))}>
              <ArrowLineDown size={18} weight="light" /> {t('files.bringHere')}
            </button>
            <button className="option tool" data-nav="mkdir" onClick={mkdir}>
              <FolderPlus size={18} weight="light" /> {t('files.newFolder')}
            </button>
            {clip && (
              <button className="option tool" data-nav="paste" onClick={paste}>
                <ClipboardText size={18} weight="light" /> {t(clip.cut ? 'files.pasteMove' : 'files.pasteCopy', { count: clip.items.length })}
              </button>
            )}
          </>
        )}
      </div>

      {bring && picked.size > 0 && (
        <div className="bring-bar">
          <span className="bring-count">{t('files.fileCount', { n: picked.size })}{pickedSize ? ` · ${fmt.bytes(pickedSize)}` : ''}</span>
          <button className="btn-primary" data-nav="bring-copy" onClick={() => bringHere(false)} disabled={busy}>
            <Copy size={18} weight="light" /> {busy ? t('files.copying') : t('files.copyTo', { name: bring.name })}
          </button>
          <button className="btn-ghost" data-nav="bring-move" onClick={() => bringHere(true)} disabled={busy}>
            <Scissors size={18} weight="light" /> {t('files.move')}
          </button>
        </div>
      )}

      {msg && <p className="notice ok">{msg}</p>}
      {error && <p className="notice error">{error}</p>}
      {entries?.length === 0 && <p className="hero-meta" style={{ marginTop: 20 }}>{t('files.empty')}</p>}
      <div className="file-list">
        {entries?.map((e, i) => {
          const on = picked.has(e.path)
          const click = () => {
            if (e.isDir) navigate(folderUrl(e.path, bring))
            else if (bring) toggle(e.path)
            else navigate(itemUrl(e.path))
          }
          return (
            <button
              key={e.path}
              className={`file-row${on ? ' picked' : ''}`}
              data-nav={`f-${e.path}`}
              data-nav-default={i === 0 ? '' : undefined}
              aria-pressed={bring && !e.isDir ? on : undefined}
              onClick={click}
            >
              {bring && (e.isDir ? <span className="pick-gap" aria-hidden /> : on ? <CheckSquare size={22} weight="fill" className="pick-on" /> : <Square size={22} weight="light" />)}
              {e.isDir ? <FolderSimple size={22} weight="fill" /> : <File size={22} weight="light" />}
              <span className="file-name">{e.name}</span>
              <span className="file-meta">{e.isDir ? t('files.folder') : fmt.bytes(e.size ?? 0)}</span>
            </button>
          )
        })}
      </div>
    </Page>
  )
}

// ---------- Elemento (acciones) ----------

export function FileItem() {
  const t = useT()
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const path = params.get('ruta') ?? ''
  const [confirmTrash, setConfirmTrash] = useState(false)
  const [msg, setMsg] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const open = () => call('open_path', { path }).then(() => setMsg(t('files.opening'))).catch((e) => setError(String(e)))
  const put = (cut: boolean) => { clipboard.set({ items: [path], cut }); setMsg(cut ? t('files.readyMove') : t('files.readyCopy')) }
  const toTrash = async () => {
    if (!confirmTrash) return setConfirmTrash(true)
    try { await call('trash_items', { items: [path] }); navigate(-1) } catch (e) { setError(String(e)) }
  }

  return (
    <Page title={nameOf(path)}>
      <p className="path mono">{path}</p>
      <div className="menu-list" style={{ marginTop: 20 }}>
        <button className="menu-item" data-nav="open" data-nav-default="" onClick={open}><ArrowSquareOut size={22} weight="light" /><span className="row-label">{t('common.open')}</span></button>
        <button className="menu-item" data-nav="copy" onClick={() => put(false)}><Copy size={22} weight="light" /><span className="row-label">{t('files.copy')}</span></button>
        <button className="menu-item" data-nav="cut" onClick={() => put(true)}><Scissors size={22} weight="light" /><span className="row-label">{t('files.move')}</span></button>
        <button className="menu-item" data-nav="trash" onClick={toTrash}>
          <Trash size={22} weight="light" />
          <span className="row-label">{confirmTrash ? t('files.trashConfirm') : t('files.trash')}</span>
        </button>
      </div>
      {msg && <p className="notice ok">{msg}</p>}
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}
