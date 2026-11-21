import { useEffect, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import { Power, FolderOpen, Globe, HardDrives, CaretRight, PaperPlaneTilt, Check, X, Folder } from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { Qr } from '../components/Qr'
import { call } from '../native'
import { t as tr, useT, fmt } from '../i18n'
import './Receive.css'

const recibidos = (p: string) => `/archivos/carpeta?ruta=${encodeURIComponent(p)}`
const sizeText = (b?: number | null) => (b ? fmt.bytes(b) : null)

// ---------- Menú: tres formas de enviar ----------

export function Receive() {
  const t = useT()
  const navigate = useNavigate()
  return (
    <Page title={t('receive.title')}>
      <p className="hero-meta receive-intro">{t('receive.intro')}</p>
      <div className="menu-list">
        <button className="menu-item" data-nav="web" data-nav-default="" onClick={() => navigate('/recibir/navegador')}>
          <Globe size={24} weight="light" />
          <span className="row-label">{t('receive.menu.web')} <span className="tag emu receive-tag">{t('receive.menu.easiest')}</span></span>
          <span className="menu-item-value">{t('receive.menu.webDesc')}</span>
          <CaretRight size={18} weight="light" />
        </button>
        <button className="menu-item" data-nav="ftp" onClick={() => navigate('/recibir/ftp')}>
          <HardDrives size={24} weight="light" />
          <span className="row-label">{t('receive.menu.ftp')}</span>
          <span className="menu-item-value">{t('receive.menu.ftpDesc')}</span>
          <CaretRight size={18} weight="light" />
        </button>
        <button className="menu-item" data-nav="localsend" onClick={() => navigate('/recibir/localsend')}>
          <PaperPlaneTilt size={24} weight="light" />
          <span className="row-label">{t('receive.menu.ls')}</span>
          <span className="menu-item-value">{t('receive.menu.lsDesc')}</span>
          <CaretRight size={18} weight="light" />
        </button>
      </div>
    </Page>
  )
}

// ---------- Navegador ----------

interface RemoteStatus { url: string | null; uploadUrl: string | null }
const mockRemote: RemoteStatus = { url: 'http://192.168.1.40:7777/?t=abc', uploadUrl: 'http://192.168.1.40:7777/subir?t=abc' }

export function ReceiveWeb() {
  const t = useT()
  const [params] = useSearchParams()
  const dest = params.get('d')
  const [st, setSt] = useState<RemoteStatus | null>(null)
  useEffect(() => { call<RemoteStatus>('remote_status', {}, mockRemote).then(setSt) }, [])
  const url = st?.uploadUrl ? `${st.uploadUrl}${dest ? `&d=${encodeURIComponent(dest)}` : ''}` : null

  return (
    <Page title={t('receive.web.title')}>
      <div className="receive">
        <div>
          <ol className="receive-steps">
            <li><b>{t('receive.web.step1.title')}</b> {t('receive.web.step1.text')}</li>
            <li><b>{t('receive.web.step2.title')}</b> {t('receive.web.step2.text')}</li>
            <li><b>{t('receive.web.step3.title')}</b> {t('receive.web.step3.text')}</li>
          </ol>
          {url ? <p className="receive-url mono">{url.replace(/\?t=.*/, '')}</p> : <p className="notice error">{t('receive.web.noNetwork')}</p>}
          <p className="notice info">{t('receive.web.note')}</p>
        </div>
        {url && (
          <figure className="receive-qr">
            <Qr text={url} size={240} />
            <figcaption>{t('receive.web.scan')}</figcaption>
          </figure>
        )}
      </div>
    </Page>
  )
}

// ---------- FTP ----------

interface Status {
  running: boolean; ip: string | null; port: number; user: string
  password: string | null; folder: string; url: string | null
}

const mockOff: Status = { running: false, ip: '192.168.1.40', port: 2121, user: 'consola', password: null, folder: 'C:\\Users\\CASA\\Recibidos', url: null }
const mockOn: Status = { ...mockOff, running: true, password: 'k7mq2xvh4a', url: 'ftp://consola:k7mq2xvh4a@192.168.1.40:2121/' }

export function ReceiveFtp() {
  const t = useT()
  const navigate = useNavigate()
  const [st, setSt] = useState<Status | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => { call<Status>('receive_status', {}, mockOff).then(setSt) }, [])

  const toggle = async () => {
    setError(null)
    try {
      setSt(st?.running ? await call<Status>('receive_stop', {}, mockOff) : await call<Status>('receive_start', {}, mockOn))
    } catch (e) { setError(String(e)) }
  }

  return (
    <Page title={t('receive.ftp.title')}>
      <div className="receive">
        <div>
          <p className="hero-meta" style={{ maxWidth: 460 }}>
            {t('receive.ftp.intro')}
          </p>
          <div className="hero-actions">
            <button className="btn-primary" data-nav="toggle" data-nav-default="" onClick={toggle}>
              <Power size={18} weight="light" /> {st?.running ? t('receive.ftp.turnOff') : t('receive.ftp.turnOn')}
            </button>
            <button className="btn-ghost" data-nav="folder" onClick={() => st && navigate(recibidos(st.folder))}>
              <FolderOpen size={18} weight="light" /> {t('receive.ftp.seeReceived')}
            </button>
          </div>

          {st?.running && (
            <dl className="detail-facts creds">
              <dt>{t('receive.ftp.server')}</dt><dd className="mono">{st.ip ?? '—'}</dd>
              <dt>{t('receive.ftp.port')}</dt><dd className="mono">{st.port}</dd>
              <dt>{t('receive.ftp.user')}</dt><dd className="mono">{st.user}</dd>
              <dt>{t('receive.ftp.password')}</dt><dd className="mono">{st.password}</dd>
              <dt>{t('receive.ftp.folder')}</dt><dd className="mono">{st.folder}</dd>
            </dl>
          )}
          {st?.running && (
            <p className="notice info">{t('receive.ftp.note')}</p>
          )}
          {error && <p className="notice error">{error}</p>}
        </div>

        {st?.running && st.url && (
          <figure className="receive-qr">
            <Qr text={st.url} size={240} />
            <figcaption>{t('receive.ftp.scan')}</figcaption>
          </figure>
        )}
      </div>
    </Page>
  )
}

// ---------- LocalSend ----------

interface LsItem { name: string; path: string; from: string }
interface LsStatus {
  running: boolean; error: string | null; alias: string; ip: string | null; port: number
  dest: string; destLabel: string; destPath: string
  active: { from: string; done: number; total: number; size: number } | null
  pending: { id: string; from: string; count: number; size: number; names: string[] } | null
  recent: LsItem[]
}

const mockLs = (): LsStatus => ({
  running: true, error: null, alias: 'CouchDesk (SALON)', ip: '192.168.1.40', port: 53317,
  dest: 'recibidos', destLabel: tr('receive.received'), destPath: 'C:\\Users\\CASA\\Recibidos',
  active: null, pending: null,
  recent: [
    { name: 'Crash Bandicoot (Europe).chd', path: 'C:\\Users\\CASA\\Recibidos\\Crash Bandicoot (Europe).chd', from: 'Pixel de Pablo' },
    { name: 'scph5501.bin', path: 'C:\\Users\\CASA\\Recibidos\\scph5501.bin', from: 'Pixel de Pablo' },
  ],
})

const fileCount = (n: number) => tr('receive.fileCount', { n })
const parentOf = (p: string) => p.replace(/[\\/][^\\/]*$/, '')

export function ReceiveLocalSend() {
  const t = useT()
  const navigate = useNavigate()
  const [st, setSt] = useState<LsStatus | null>(null)
  // Avisa cada segundo de que la pantalla está abierta: mientras lo esté, se acepta solo
  useEffect(() => {
    const tick = () => call<LsStatus>('localsend_status', { screen: true }, mockLs()).then(setSt).catch(() => {})
    tick()
    const timer = setInterval(tick, 1000)
    return () => clearInterval(timer)
  }, [])
  const answer = async (accept: boolean) => {
    if (!st?.pending) return
    await call('localsend_answer', { id: st.pending.id, accept }, null)
    setSt({ ...st, pending: null })
  }

  return (
    <Page title={t('receive.ls.title')}>
      <div className="receive">
        <div>
          {st?.pending && (
            <div className="ls-ask">
              <p><b>{st.pending.from}</b> {t('receive.ls.wantsToSend', { files: fileCount(st.pending.count), size: sizeText(st.pending.size) ? ` (${sizeText(st.pending.size)})` : '' })}</p>
              <p className="ls-names mono">{st.pending.names.join(' · ')}{st.pending.count > st.pending.names.length ? ' …' : ''}</p>
              <div className="hero-actions">
                <button className="btn-primary" data-nav="accept" data-nav-default="" onClick={() => answer(true)}>
                  <Check size={18} weight="light" /> {t('receive.ls.accept')}
                </button>
                <button className="btn-ghost" data-nav="reject" onClick={() => answer(false)}>
                  <X size={18} weight="light" /> {t('receive.ls.reject')}
                </button>
              </div>
            </div>
          )}
          <ol className="receive-steps">
            <li><b>{t('receive.ls.step1.title')}</b> {t('receive.ls.step1.text')}</li>
            <li><b>{t('receive.ls.step2.title')}</b> {t('receive.ls.step2.text')} <b>{st?.alias ?? t('app.name')}</b>.</li>
            <li><b>{t('receive.ls.step3.title')}</b> {t('receive.ls.step3.text')}</li>
          </ol>
          <div className="hero-actions ls-actions">
            <button className="btn-primary" data-nav="dest" data-nav-default={st?.pending ? undefined : ''} onClick={() => navigate('/recibir/localsend/destino')}>
              <Folder size={18} weight="light" /> {t('receive.ls.saveIn', { place: st?.destLabel ?? t('receive.received') })}
            </button>
            <button className="btn-ghost" data-nav="folder" onClick={() => st && navigate(recibidos(st.destPath))}>
              <FolderOpen size={18} weight="light" /> {t('receive.ls.openFolder')}
            </button>
          </div>
          {st?.error && <p className="notice error">{st.error}</p>}
          {st?.active && (
            <p className="notice info">{t('receive.ls.receiving', { from: st.active.from, done: st.active.done, files: fileCount(st.active.total), size: sizeText(st.active.size) ? ` · ${sizeText(st.active.size)}` : '' })}</p>
          )}
          {st?.running && !st.active && <p className="notice ok">{t('receive.ls.visible', { alias: st.alias, ip: st.ip ? ` · ${st.ip}` : '' })}</p>}

          {!!st?.recent.length && (
            <>
              <p className="eyebrow ls-recent-title">{t('receive.ls.recent')}</p>
              <ul className="ls-recent">
                {st.recent.slice(0, 6).map((r) => (
                  <li key={r.path}>
                    <button className="row" data-nav={`r-${r.path}`} onClick={() => navigate(recibidos(parentOf(r.path)))}>
                      <span className="row-label">{r.name}</span>
                      <span className="menu-item-value">{t('receive.ls.from', { from: r.from })}</span>
                    </button>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
        <figure className="receive-qr ls-qr">
          <Qr text="https://localsend.org/download" size={200} />
          <figcaption>{t('receive.ls.qr')}</figcaption>
        </figure>
      </div>
    </Page>
  )
}

interface LsDest { id: string; group: string; label: string }
const mockDests = (): LsDest[] => [
  { id: 'recibidos', group: tr('receive.received'), label: tr('receive.received') },
  { id: 'duckstation.juegos', group: 'DuckStation', label: tr('receive.dest.games') },
  { id: 'duckstation.bios', group: 'DuckStation', label: 'BIOS' },
  { id: 'pcsx2.juegos', group: 'PCSX2', label: tr('receive.dest.games') },
  { id: 'pcsx2.bios', group: 'PCSX2', label: tr('receive.dest.biosKeys') },
]

export function ReceiveLocalSendDest() {
  const t = useT()
  const navigate = useNavigate()
  const [dests, setDests] = useState<LsDest[]>([])
  const [current, setCurrent] = useState<string | null>(null)
  useEffect(() => {
    call<LsDest[]>('localsend_destinations', {}, mockDests()).then(setDests).catch(() => {})
    call<LsStatus>('localsend_status', { screen: true }, mockLs()).then((s) => setCurrent(s.dest)).catch(() => {})
  }, [])
  const pick = async (id: string) => {
    await call('localsend_set_dest', { id }, null)
    navigate(-1)
  }
  return (
    <Page title={t('receive.dest.title')}>
      <div className="menu-list">
        {dests.map((d, i) => (
          <button key={d.id} className="menu-item" data-nav={d.id} data-nav-default={(current ? d.id === current : i === 0) ? '' : undefined} onClick={() => pick(d.id)}>
            <Folder size={24} weight="light" />
            <span className="row-label">{d.group === d.label ? d.label : d.group}</span>
            <span className="menu-item-value">{d.group === d.label ? t('receive.dest.general') : d.label}</span>
            {d.id === current ? <Check size={18} weight="light" /> : <CaretRight size={18} weight="light" />}
          </button>
        ))}
      </div>
    </Page>
  )
}
