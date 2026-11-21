import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Power, FolderOpen } from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { Qr } from '../components/Qr'
import { call } from '../native'
import './Receive.css'

interface Status {
  running: boolean; ip: string | null; port: number; user: string
  password: string | null; folder: string; url: string | null
}

const mockOff: Status = { running: false, ip: '192.168.1.40', port: 2121, user: 'consola', password: null, folder: 'C:\\Users\\CASA\\Recibidos', url: null }
const mockOn: Status = { ...mockOff, running: true, password: 'k7mq2xvh4a', url: 'ftp://consola:k7mq2xvh4a@192.168.1.40:2121/' }

export function Receive() {
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
    <Page title="Recibir archivos">
      <div className="receive">
        <div>
          <p className="hero-meta" style={{ maxWidth: 460 }}>
            Envía archivos a este PC desde el móvil u otro ordenador de tu casa, con cualquier app de FTP.
          </p>
          <div className="hero-actions">
            <button className="btn-primary" data-nav="toggle" data-nav-default="" onClick={toggle}>
              <Power size={18} weight="light" /> {st?.running ? 'Apagar' : 'Encender'}
            </button>
            <button className="btn-ghost" data-nav="folder" onClick={() => st && navigate(`/archivos/carpeta?ruta=${encodeURIComponent(st.folder)}`)}>
              <FolderOpen size={18} weight="light" /> Ver recibidos
            </button>
          </div>

          {st?.running && (
            <dl className="detail-facts creds">
              <dt>Servidor</dt><dd className="mono">{st.ip ?? '—'}</dd>
              <dt>Puerto</dt><dd className="mono">{st.port}</dd>
              <dt>Usuario</dt><dd className="mono">{st.user}</dd>
              <dt>Contraseña</dt><dd className="mono">{st.password}</dd>
              <dt>Carpeta</dt><dd className="mono">{st.folder}</dd>
            </dl>
          )}
          {st?.running && (
            <p className="notice info">Solo funciona en tu red de casa. Admite cifrado (FTPS). La contraseña cambia cada vez que lo enciendes.</p>
          )}
          {error && <p className="notice error">{error}</p>}
        </div>

        {st?.running && st.url && (
          <figure className="receive-qr">
            <Qr text={st.url} size={240} />
            <figcaption>Escanéalo con tu app de FTP</figcaption>
          </figure>
        )}
      </div>
    </Page>
  )
}
