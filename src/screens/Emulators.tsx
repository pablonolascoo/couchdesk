import { useCallback, useEffect, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { CaretRight, DownloadSimple, Play, ArrowCounterClockwise, FolderOpen, Cpu } from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { call } from '../native'
import './Emulators.css'

interface Hardware { cpu: string; threads: number; avx2: boolean; ramGb: number; gpu: string; vramGb: number; gpuTier: number; vulkan: boolean }
interface Emu {
  id: string; name: string; console: string; trust: 'fiable' | 'experimental'; verdict: 'bien' | 'justo' | 'no'
  bios: string; note: string; installed: string | null; canRollback: boolean; folder: string | null
}

const verdictText = { bien: 'Irá bien', justo: 'Irá justo', no: 'No funcionará bien' }
const verdictClass = { bien: 'emu', justo: 'epic', no: 'xbox' } // colores de las etiquetas existentes

const mockHw: Hardware = { cpu: 'Intel Core i7-10710U', threads: 12, avx2: true, ramGb: 15.8, gpu: 'NVIDIA GeForce GTX 1650 with Max-Q Design', vramGb: 4, gpuTier: 2, vulkan: true }
const mockList: Emu[] = [
  { id: 'duckstation', name: 'DuckStation', console: 'PlayStation', trust: 'fiable', verdict: 'bien', bios: 'Necesita la BIOS de tu PS1…', note: '', installed: null, canRollback: false, folder: null },
  { id: 'pcsx2', name: 'PCSX2', console: 'PlayStation 2', trust: 'fiable', verdict: 'bien', bios: 'Necesita la BIOS de tu PS2…', note: '', installed: 'v2.8.2', canRollback: false, folder: 'C:\\Consola\\pcsx2' },
  { id: 'rpcs3', name: 'RPCS3', console: 'PlayStation 3', trust: 'fiable', verdict: 'justo', bios: 'Firmware oficial PS3UPDAT.PUP…', note: 'Tira mucho del procesador.', installed: null, canRollback: false, folder: null },
  { id: 'kytyps5', name: 'KytyPS5', console: 'PlayStation 5', trust: 'experimental', verdict: 'no', bios: 'No necesita BIOS.', note: '', installed: null, canRollback: false, folder: null },
  { id: 'eden', name: 'Eden', console: 'Nintendo Switch', trust: 'fiable', verdict: 'justo', bios: 'prod.keys y firmware de tu Switch…', note: '', installed: null, canRollback: false, folder: null },
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
      <div className="menu-list" style={{ marginTop: 18 }}>
        {list?.map((e, i) => (
          <button key={e.id} className="menu-item" data-nav={`emu-${e.id}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => navigate(`/emuladores/${e.id}`)}>
            <span className="row-label">
              {e.name}
              <span className="row-sub">{e.console}{e.installed ? ` · instalado ${e.installed}` : ''}</span>
            </span>
            {e.trust === 'experimental' && <span className="tag steam">Experimental</span>}
            <span className={`tag ${verdictClass[e.verdict]}`}>{verdictText[e.verdict]}</span>
            <CaretRight size={18} weight="light" />
          </button>
        ))}
      </div>
      <p className="notice info">La consola no descarga juegos, BIOS ni claves. Usa los de tus propias consolas.</p>
    </Page>
  )
}

export function EmulatorDetail() {
  const { id } = useParams()
  const navigate = useNavigate()
  const { list, reload } = useEmulators()
  const e = list?.find((x) => x.id === id)
  const [busy, setBusy] = useState(false)
  const [msg, setMsg] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  if (!e) return <Page title="Emulador"><p className="hero-meta">Cargando…</p></Page>

  const run = async (fn: () => Promise<string | void>, okText: (r: string | void) => string) => {
    setBusy(true); setError(null); setMsg(null)
    try { const r = await fn(); setMsg(okText(r)); reload() } catch (err) { setError(String(err)) }
    setBusy(false)
  }
  const install = () => run(() => call<string>('install_emulator', { id: e.id }, 'v1.0'), (v) => `Listo: versión ${v}`)
  const open = () => run(() => call('launch_emulator', { id: e.id }), () => 'Abriendo…')
  const rollback = () => run(() => call<string>('rollback_emulator', { id: e.id }, 'anterior'), (v) => `Vuelto a la versión ${v}`)

  return (
    <Page title={e.name}>
      <p className="hero-meta">{e.console}</p>
      <div style={{ display: 'flex', gap: 8, marginTop: 10 }}>
        <span className={`tag ${verdictClass[e.verdict]}`}>{verdictText[e.verdict]} en tu PC</span>
        {e.trust === 'experimental' && <span className="tag steam">Experimental</span>}
      </div>
      {e.note && <p className="notice info">{e.note}</p>}
      {e.trust === 'experimental' && !e.installed && (
        <p className="notice error">Proyecto experimental: puede fallar o cambiar mucho entre versiones.</p>
      )}

      <div className="hero-actions">
        {e.installed && (
          <button className="btn-primary" data-nav="open" data-nav-default="" onClick={open} disabled={busy}><Play size={18} weight="fill" /> Abrir</button>
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

      <h3 className="label" style={{ marginTop: 32 }}>Tu BIOS, firmware o claves</h3>
      <p className="bios-text">{e.bios}</p>
      {e.folder && (
        <button className="option tool" style={{ marginTop: 14 }} data-nav="folder" onClick={() => navigate(`/archivos/carpeta?ruta=${encodeURIComponent(e.folder!)}`)}>
          <FolderOpen size={18} weight="light" /> Abrir la carpeta del emulador
        </button>
      )}
      <p className="hero-meta" style={{ marginTop: 14, fontSize: 13 }}>Se actualiza solo al arrancar la consola. La versión anterior se guarda por si la nueva falla.</p>
    </Page>
  )
}
