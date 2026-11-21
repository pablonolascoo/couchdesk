import { useCallback, useEffect, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import {
  FolderSimple, File, HardDrives, ArrowUp, ClipboardText, FolderPlus, ArrowSquareOut,
  Copy, Scissors, Trash, CaretRight,
} from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { call } from '../native'
import { sizeText } from '../data/games'
import {
  clipboard, useClipboard, mockPlaces, mockEntries, parentOf, nameOf, type Entry, type Place,
} from '../data/files'
import './Files.css'

const folderUrl = (p: string) => `/archivos/carpeta?ruta=${encodeURIComponent(p)}`
const itemUrl = (p: string) => `/archivos/elemento?ruta=${encodeURIComponent(p)}`

// ---------- Lugares ----------

export function FilesHome() {
  const navigate = useNavigate()
  const [places, setPlaces] = useState<Place[] | null>(null)
  useEffect(() => { call<Place[]>('list_places', {}, mockPlaces).then(setPlaces) }, [])
  return (
    <Page title="Archivos">
      <div className="menu-list">
        {places?.map((p, i) => (
          <button key={p.path} className="menu-item" data-nav={`place-${p.path}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => navigate(folderUrl(p.path))}>
            {p.label.startsWith('Disco') ? <HardDrives size={24} weight="light" /> : <FolderSimple size={24} weight="light" />}
            <span className="row-label">{p.label}</span>
            <span className="menu-item-value mono">{p.path}</span>
            <CaretRight size={18} weight="light" />
          </button>
        ))}
      </div>
    </Page>
  )
}

// ---------- Carpeta ----------

export function Folder() {
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const path = params.get('ruta') ?? 'C:\\'
  const [entries, setEntries] = useState<Entry[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [msg, setMsg] = useState<string | null>(null)
  const clip = useClipboard()
  const parent = parentOf(path)

  const load = useCallback(() => {
    setError(null)
    call<Entry[]>('list_dir', { path }, mockEntries(path)).then(setEntries).catch((e) => setError(String(e)))
  }, [path])
  useEffect(load, [load])

  const paste = async () => {
    if (!clip) return
    try {
      const n = await call<number>('paste_items', { items: clip.items, dest: path, cut: clip.cut }, clip.items.length)
      setMsg(`${n} ${n === 1 ? 'elemento' : 'elementos'} ${clip.cut ? 'movido' : 'copiado'}${n === 1 ? '' : 's'}`)
      if (clip.cut) clipboard.set(null)
      load()
    } catch (e) { setError(String(e)) }
  }
  const mkdir = () => call('new_folder', { dir: path }, '').then(() => { setMsg('Carpeta creada'); load() }).catch((e) => setError(String(e)))

  return (
    <Page title={nameOf(path)} hints={[['A', 'Abrir'], ['B', 'Atrás']]}>
      <p className="path mono">{path}</p>
      <div className="toolbar">
        {parent && (
          <button className="option tool" data-nav="up" onClick={() => navigate(folderUrl(parent))}>
            <ArrowUp size={18} weight="light" /> Subir
          </button>
        )}
        <button className="option tool" data-nav="mkdir" onClick={mkdir}>
          <FolderPlus size={18} weight="light" /> Nueva carpeta
        </button>
        {clip && (
          <button className="option tool" data-nav="paste" onClick={paste}>
            <ClipboardText size={18} weight="light" /> Pegar {clip.items.length} ({clip.cut ? 'mover' : 'copiar'})
          </button>
        )}
      </div>
      {msg && <p className="notice ok">{msg}</p>}
      {error && <p className="notice error">{error}</p>}
      {entries?.length === 0 && <p className="hero-meta" style={{ marginTop: 20 }}>Carpeta vacía.</p>}
      <div className="file-list">
        {entries?.map((e, i) => (
          <button
            key={e.path}
            className="file-row"
            data-nav={`f-${e.path}`}
            data-nav-default={i === 0 ? '' : undefined}
            onClick={() => navigate(e.isDir ? folderUrl(e.path) : itemUrl(e.path))}
          >
            {e.isDir ? <FolderSimple size={22} weight="fill" /> : <File size={22} weight="light" />}
            <span className="file-name">{e.name}</span>
            <span className="file-meta">{e.isDir ? 'Carpeta' : sizeText(e.size) ?? `${e.size ?? 0} B`}</span>
          </button>
        ))}
      </div>
    </Page>
  )
}

// ---------- Elemento (acciones) ----------

export function FileItem() {
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const path = params.get('ruta') ?? ''
  const [confirmTrash, setConfirmTrash] = useState(false)
  const [msg, setMsg] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const open = () => call('open_path', { path }).then(() => setMsg('Abriendo…')).catch((e) => setError(String(e)))
  const put = (cut: boolean) => { clipboard.set({ items: [path], cut }); setMsg(cut ? 'Listo para mover: ve a otra carpeta y pulsa Pegar' : 'Copiado: ve a otra carpeta y pulsa Pegar') }
  const toTrash = async () => {
    if (!confirmTrash) return setConfirmTrash(true)
    try { await call('trash_items', { items: [path] }); navigate(-1) } catch (e) { setError(String(e)) }
  }

  return (
    <Page title={nameOf(path)}>
      <p className="path mono">{path}</p>
      <div className="menu-list" style={{ marginTop: 20 }}>
        <button className="menu-item" data-nav="open" data-nav-default="" onClick={open}><ArrowSquareOut size={22} weight="light" /><span className="row-label">Abrir</span></button>
        <button className="menu-item" data-nav="copy" onClick={() => put(false)}><Copy size={22} weight="light" /><span className="row-label">Copiar</span></button>
        <button className="menu-item" data-nav="cut" onClick={() => put(true)}><Scissors size={22} weight="light" /><span className="row-label">Mover</span></button>
        <button className="menu-item" data-nav="trash" onClick={toTrash}>
          <Trash size={22} weight="light" />
          <span className="row-label">{confirmTrash ? 'Pulsa A otra vez para enviar a la papelera' : 'Enviar a la papelera'}</span>
        </button>
      </div>
      {msg && <p className="notice ok">{msg}</p>}
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}
