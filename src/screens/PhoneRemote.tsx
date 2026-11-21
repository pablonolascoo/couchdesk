import { DeviceMobile } from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { Qr } from '../components/Qr'
import { useRemote } from '../data/remote'
import { useT } from '../i18n'
import './Receive.css'

export function PhoneRemote() {
  const t = useT()
  const remote = useRemote()
  return (
    <Page title={t('remote.title')}>
      <div className="receive">
        <div>
          <p className="hero-meta" style={{ maxWidth: 480 }}>
            {t('remote.intro')}
          </p>
          <h3 className="label" style={{ marginTop: 28 }}>{t('remote.connected')}</h3>
          {remote?.clients.length ? (
            remote.clients.map((c) => (
              <div key={c.id} className="option" style={{ justifyContent: 'flex-start', gap: 12 }}>
                <DeviceMobile size={20} weight="light" color={c.color} /> {c.name}
              </div>
            ))
          ) : (
            <p className="hero-meta">{t('remote.none')}</p>
          )}
          <p className="notice info">{t('remote.note')}</p>
          <button className="btn-ghost" style={{ marginTop: 18 }} data-nav="back" data-nav-default="" onClick={() => history.back()}>{t('remote.back')}</button>
        </div>
        {remote?.url ? (
          <figure className="receive-qr">
            <Qr text={remote.url} size={260} />
            <figcaption className="mono">{remote.url.split('?')[0]}</figcaption>
          </figure>
        ) : (
          <p className="notice error">{t('remote.noNetwork')}</p>
        )}
      </div>
    </Page>
  )
}
