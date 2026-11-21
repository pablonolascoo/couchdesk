import { DeviceMobile } from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { Qr } from '../components/Qr'
import { useRemote } from '../data/remote'
import './Receive.css'

export function PhoneRemote() {
  const remote = useRemote()
  return (
    <Page title="Mando móvil">
      <div className="receive">
        <div>
          <p className="hero-meta" style={{ maxWidth: 480 }}>
            Escanea el código con la cámara del móvil. Se abre una página que convierte el móvil en ratón, teclado o mando.
            Pueden conectarse varios a la vez.
          </p>
          <h3 className="label" style={{ marginTop: 28 }}>Conectados</h3>
          {remote?.clients.length ? (
            remote.clients.map((c) => (
              <div key={c.id} className="option" style={{ justifyContent: 'flex-start', gap: 12 }}>
                <DeviceMobile size={20} weight="light" color={c.color} /> {c.name}
              </div>
            ))
          ) : (
            <p className="hero-meta">Nadie todavía.</p>
          )}
          <p className="notice info">El móvil y el PC deben estar en la misma red Wi-Fi. La primera vez, Windows puede pedir permiso para "Consola": acepta en redes privadas.</p>
          <button className="btn-ghost" style={{ marginTop: 18 }} data-nav="back" data-nav-default="" onClick={() => history.back()}>Volver</button>
        </div>
        {remote?.url ? (
          <figure className="receive-qr">
            <Qr text={remote.url} size={260} />
            <figcaption className="mono">{remote.url.split('?')[0]}</figcaption>
          </figure>
        ) : (
          <p className="notice error">No hay conexión de red.</p>
        )}
      </div>
    </Page>
  )
}
