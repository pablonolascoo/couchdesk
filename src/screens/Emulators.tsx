import { useCallback, useEffect, useState, type CSSProperties } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { convertFileSrc } from '@tauri-apps/api/core'
import {
  DownloadSimple, Play, ArrowCounterClockwise, Cpu, FolderOpen, Disc, Key, FloppyDisk,
  GameController, DeviceMobile, Cube, ArrowSquareOut, type Icon,
} from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { call, isDesktopApp } from '../native'
import { BrandLogo, type Brand } from '../ui/brands'
import './Emulators.css'

interface Hardware { cpu: string; threads: number; avx2: boolean; ramGb: number; gpu: string; vramGb: number; gpuTier: number; vulkan: boolean }
interface Folder { key: 'juegos' | 'bios' | 'datos'; label: string; hint: string; path: string }
interface Emu {
  id: string; name: string; console: string; trust: 'fiable' | 'experimental'; verdict: 'bien' | 'justo' | 'no'
  bios: string; note: string; installed: string | null; canRollback: boolean; folder: string | null
  icon?: string | null; folders: Folder[]
}

const verdictText = { bien: 'Irá bien', justo: 'Irá justo', no: 'No funcionará bien' }
const verdictClass = { bien: 'emu', justo: 'epic', no: 'xbox' } // colores de las etiquetas existentes

// Identidad de cada consola: su color y su logo (oficial en PlayStation; Nintendo no permite usar el suyo)
interface Look { tone: string; brand?: Brand; art?: Icon }
const looks: Record<string, Look> = {
  duckstation: { tone: '#6b7480', brand: 'playstation' },
  pcsx2: { tone: '#1c3f94', brand: 'ps2' },
  rpcs3: { tone: '#26292f', brand: 'ps3' },
  shadps4: { tone: '#003791', brand: 'ps4' },
  kytyps5: { tone: '#0b2a6b', brand: 'ps5' },
  sharpemu: { tone: '#0b2a6b', brand: 'ps5' },
  anyps5: { tone: '#0b2a6b', brand: 'ps5' },
  ares: { tone: '#2e7d32', art: GameController },
  melonds: { tone: '#7d8590', art: DeviceMobile },
  dolphin: { tone: '#2b5aa8', art: Cube },
  eden: { tone: '#d0021b', art: GameController },
  novas: { tone: '#9e0a1e', art: GameController },
  'eden-nx2': { tone: '#9e0a1e', art: GameController },
}
const lookOf = (id: string): Look => looks[id] ?? { tone: '#5b5148', art: GameController }

function ConsoleArt({ look, size }: { look: Look; size: number }) {
  if (look.brand) return <BrandLogo name={look.brand} size={size} color="currentColor" />
  const Art = look.art ?? GameController
  return <Art size={size} weight="fill" />
}

/** Icono real del emulador si está instalado; si no, el logo de su consola */
function EmuIcon({ e, size }: { e: Emu; size: number }) {
  const look = lookOf(e.id)
  if (e.icon && isDesktopApp) return <img src={convertFileSrc(e.icon)} alt="" width={size} height={size} />
  return <ConsoleArt look={look} size={size * 0.8} />
}

const mockHw: Hardware = { cpu: 'Intel Core i7-10710U', threads: 12, avx2: true, ramGb: 15.8, gpu: 'NVIDIA GeForce GTX 1650 with Max-Q Design', vramGb: 4, gpuTier: 2, vulkan: true }
const f = (key: Folder['key'], label: string, path: string): Folder => ({ key, label, hint: 'Ejemplo', path })
const mockList: Emu[] = [
  { id: 'duckstation', name: 'DuckStation', console: 'PlayStation', trust: 'fiable', verdict: 'bien', bios: 'Necesita la BIOS de tu PS1…', note: '', installed: 'v0.1-9000', canRollback: false, folder: 'C:\\emus\\duckstation', folders: [f('juegos', 'Tus juegos', 'C:\\Users\\tu\\Juegos\\PlayStation'), f('bios', 'BIOS, firmware y claves', 'C:\\emus\\duckstation\\bios'), f('datos', 'Partidas y ajustes', 'C:\\emus\\duckstation')] },
  { id: 'pcsx2', name: 'PCSX2', console: 'PlayStation 2', trust: 'fiable', verdict: 'bien', bios: 'Necesita la BIOS de tu PS2…', note: '', installed: 'v2.8.2', canRollback: true, folder: 'C:\\emus\\pcsx2', folders: [f('juegos', 'Tus juegos', 'C:\\Users\\tu\\Juegos\\PlayStation 2')] },
  { id: 'rpcs3', name: 'RPCS3', console: 'PlayStation 3', trust: 'fiable', verdict: 'justo', bios: 'Firmware oficial PS3UPDAT.PUP…', note: 'Tira mucho del procesador.', installed: null, canRollback: false, folder: null, folders: [] },
  { id: 'shadps4', name: 'shadPS4', console: 'PlayStation 4', trust: 'experimental', verdict: 'justo', bios: 'No necesita BIOS.', note: '', installed: null, canRollback: false, folder: null, folders: [] },
  { id: 'kytyps5', name: 'KytyPS5', console: 'PlayStation 5', trust: 'experimental', verdict: 'no', bios: 'No necesita BIOS.', note: '', installed: null, canRollback: false, folder: null, folders: [] },
  { id: 'ares', name: 'ares', console: 'Nintendo 64', trust: 'fiable', verdict: 'bien', bios: 'No necesita BIOS.', note: '', installed: null, canRollback: false, folder: null, folders: [] },
  { id: 'melonds', name: 'melonDS', console: 'Nintendo DS', trust: 'fiable', verdict: 'bien', bios: 'Opcional…', note: '', installed: null, canRollback: false, folder: null, folders: [] },
  { id: 'dolphin', name: 'Dolphin', console: 'Wii y GameCube', trust: 'fiable', verdict: 'bien', bios: 'No necesita BIOS.', note: '', installed: null, canRollback: false, folder: null, folders: [] },
  { id: 'eden', name: 'Eden', console: 'Nintendo Switch', trust: 'fiable', verdict: 'justo', bios: 'prod.keys y firmware de tu Switch…', note: '', installed: null, canRollback: false, folder: null, folders: [] },
]

function useEmulators() {
  const [data, setData] = useState<[Hardware, Emu[]] | null>(null)
  const reload = useCallback(() => { call<[Hardware, Emu[]]>('list_emulators', {}, [mockHw, mockList]).then(setData) }, [])
  useEffect(reload, [reload])
  return { hw: data?.[0], list: data?.[1], reload }
}

export function Emulators() {
  const navigate = useNavigate()
  const { hw, list } = useEmulators()
  return (
    <Page title="Emuladores">
      {hw && (
        <p className="hw"><Cpu size={18} weight="light" /> Tu PC: {hw.cpu.replace(/\(R\)|\(TM\)|CPU|@.*$/g, '').trim()} · {hw.gpu.replace('NVIDIA GeForce ', '')} ({hw.vramGb} GB) · {Math.round(hw.ramGb)} GB de RAM</p>
      )}
      <div className="emu-grid">
        {list?.map((e, i) => {
          const look = lookOf(e.id)
          return (
            <button
              key={e.id}
              className={`emu-card reveal${e.installed ? ' is-installed' : ''}`}
              style={{ '--tone': look.tone, '--i': Math.min(i, 12) } as CSSProperties}
              data-nav={`emu-${e.id}`}
              data-nav-default={i === 0 ? '' : undefined}
              onClick={() => navigate(`/emuladores/${e.id}`)}
            >
              <span className="emu-art" aria-hidden><ConsoleArt look={look} size={150} /></span>
              <span className="emu-top">
                <span className="emu-icon"><EmuIcon e={e} size={34} /></span>
                <span className={`tag ${verdictClass[e.verdict]}`}>{verdictText[e.verdict]}</span>
              </span>
              <span className="emu-text">
                <span className="emu-name">{e.name}</span>
                <span className="emu-sub">
                  {e.console}
                  {e.trust === 'experimental' && ' · experimental'}
                </span>
                <span className="emu-state">{e.installed ? `Instalado · ${e.installed}` : 'Sin instalar'}</span>
              </span>
            </button>
          )
        })}
      </div>
      <p className="notice info">La consola no descarga juegos, BIOS ni claves. Usa los de tus propias consolas.</p>
    </Page>
  )
}

const folderIcon = { juegos: Disc, bios: Key, datos: FloppyDisk }

export function EmulatorDetail() {
  const { id } = useParams()
  const navigate = useNavigate()
  const { list, reload } = useEmulators()
  const e = list?.find((x) => x.id === id)
  const [busy, setBusy] = useState(false)
  const [msg, setMsg] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  if (!e) return <Page title="Emulador"><p className="hero-meta">Cargando…</p></Page>
  const look = lookOf(e.id)

  const run = async (fn: () => Promise<string | void>, okText: (r: string | void) => string) => {
    setBusy(true); setError(null); setMsg(null)
    try { const r = await fn(); setMsg(okText(r)); reload() } catch (err) { setError(String(err)) }
    setBusy(false)
  }
  const install = () => run(() => call<string>('install_emulator', { id: e.id }, 'v1.0'), (v) => `Listo: versión ${v}`)
  const open = () => run(() => call('launch_emulator', { id: e.id }), () => 'Abriendo…')
  const rollback = () => run(() => call<string>('rollback_emulator', { id: e.id }, 'anterior'), (v) => `Vuelto a la versión ${v}`)
  // Las carpetas se crean al abrirlas por primera vez
  const folder = async (key: Folder['key'], inWindows = false) => {
    setError(null)
    try {
      const path = await call<string>('emu_folder', { id: e.id, key }, e.folders.find((x) => x.key === key)?.path ?? '')
      if (inWindows) await call('open_path', { path })
      else navigate(`/archivos/carpeta?ruta=${encodeURIComponent(path)}`)
    } catch (err) { setError(String(err)) }
  }

  return (
    <Page title={e.name}>
      <section className="emu-hero reveal" style={{ '--tone': look.tone } as CSSProperties}>
        <span className="emu-art" aria-hidden><ConsoleArt look={look} size={260} /></span>
        <span className="emu-icon big"><EmuIcon e={e} size={52} /></span>
        <div>
          <p className="emu-hero-console">{e.console}</p>
          <h2 className="emu-hero-name">{e.name}</h2>
          <div className="emu-hero-tags">
            <span className={`tag ${verdictClass[e.verdict]}`}>{verdictText[e.verdict]} en tu PC</span>
            {e.trust === 'experimental' && <span className="tag steam">Experimental</span>}
            <span className="tag">{e.installed ? `Instalado · ${e.installed}` : 'Sin instalar'}</span>
          </div>
        </div>
      </section>

      {e.note && <p className="notice info">{e.note}</p>}
      {e.trust === 'experimental' && !e.installed && (
        <p className="notice error">Proyecto experimental: puede fallar o cambiar mucho entre versiones.</p>
      )}

      <div className="hero-actions">
        {e.installed && (
          <button className="btn-primary" data-nav="open" data-nav-default="" onClick={open} disabled={busy}><Play size={18} weight="fill" /> Jugar</button>
        )}
        <button className={e.installed ? 'btn-ghost' : 'btn-primary'} data-nav="install" data-nav-default={e.installed ? undefined : ''} onClick={install} disabled={busy}>
          <DownloadSimple size={18} weight="light" /> {busy ? 'Descargando…' : e.installed ? 'Buscar actualización' : 'Instalar'}
        </button>
        {e.canRollback && (
          <button className="btn-ghost" data-nav="rollback" onClick={rollback} disabled={busy}><ArrowCounterClockwise size={18} weight="light" /> Versión anterior</button>
        )}
      </div>
      {msg && <p className="notice ok">{msg}</p>}
      {error && <p className="notice error">{error}</p>}

      <h3 className="label" style={{ marginTop: 32 }}>Carpetas</h3>
      <div className="emu-folders">
        {e.folders.map((fo) => {
          const Icon = folderIcon[fo.key]
          return (
            <div key={fo.key} className="emu-folder">
              <button className="option emu-folder-main" data-nav={`folder-${fo.key}`} onClick={() => folder(fo.key)}>
                <span className="emu-folder-icon"><Icon size={22} /></span>
                <span className="emu-folder-text">
                  <span className="emu-folder-label">{fo.label}</span>
                  <span className="emu-folder-hint">{fo.hint}</span>
                  <span className="emu-folder-path mono">{fo.path}</span>
                </span>
                <FolderOpen size={20} />
              </button>
              <button className="option emu-folder-win" data-nav={`folder-${fo.key}-win`} onClick={() => folder(fo.key, true)} aria-label={`Abrir «${fo.label}» en el explorador de Windows`}>
                <ArrowSquareOut size={18} /> Windows
              </button>
            </div>
          )
        })}
      </div>
      {!e.installed && <p className="hero-meta" style={{ marginTop: 10, fontSize: 13 }}>Al instalarlo aparecen también las carpetas de BIOS y de partidas.</p>}

      <h3 className="label" style={{ marginTop: 28 }}>Tu BIOS, firmware o claves</h3>
      <p className="bios-text">{e.bios}</p>
      <p className="hero-meta" style={{ marginTop: 14, fontSize: 13 }}>Se actualiza solo al arrancar la consola. Tus partidas, BIOS y ajustes pasan a la versión nueva, y la anterior se guarda por si falla.</p>
    </Page>
  )
}
