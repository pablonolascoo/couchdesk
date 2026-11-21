import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { PlugsConnected, Storefront, LinkSimple, Power, Trash, Plus, Check } from '@phosphor-icons/react'
import { Page } from '../../components/Page'
import { call } from '../../native'
import { useT } from '../../i18n'
import { mocks, connectorDescription, type Connector, type CatalogItem } from '../../data/claude'
import '../Claude.css'

interface View { mine: Connector[]; catalog: CatalogItem[] }

function useConnectors() {
  const [view, setView] = useState<View | null>(null)
  const [error, setError] = useState<string | null>(null)
  const reload = useCallback(() => {
    call<View>('claude_connectors', {}, mocks.connectors).then(setView).catch((e) => setError(String(e)))
  }, [])
  useEffect(reload, [reload])
  return { view, setView, error, setError, reload }
}

/** /claude/conectores: los conectores del usuario (encender, apagar, quitar) */
export function ClaudeConnectors() {
  const t = useT()
  const navigate = useNavigate()
  const { view, setView, error, setError, reload } = useConnectors()

  const toggle = async (c: Connector) => {
    setView((v) => v && { ...v, mine: v.mine.map((x) => (x.name === c.name ? { ...x, enabled: !c.enabled } : x)) })
    call('claude_connector_toggle', { name: c.name, enabled: !c.enabled }).catch((e) => { setError(String(e)); reload() })
  }
  const remove = async (c: Connector) => {
    setView((v) => v && { ...v, mine: v.mine.filter((x) => x.name !== c.name) })
    call('claude_connector_remove', { name: c.name }).catch((e) => { setError(String(e)); reload() })
  }

  return (
    <Page title={t('claude.connectors')}>
      <p className="c-intro">{t('claude.conn.intro')}</p>
      <div className="c-top">
        <button className="btn-primary" data-nav="conn-store" data-nav-default="" onClick={() => navigate('/claude/conectores/tienda')}>
          <Storefront size={18} weight="light" /> {t('claude.conn.store')}
        </button>
        <button className="btn-ghost" data-nav="conn-url" onClick={() => navigate('/claude/conectores/nuevo')}>
          <LinkSimple size={18} weight="light" /> {t('claude.conn.byUrl')}
        </button>
      </div>
      {view?.mine.length === 0 && (
        <div className="empty-state">
          <span className="empty-icon"><PlugsConnected size={26} weight="light" /></span>
          <h2>{t('claude.conn.emptyTitle')}</h2>
          <p>{t('claude.conn.emptyText')}</p>
        </div>
      )}
      <div className="c-list">
        {view?.mine.map((c) => (
          <div className="c-row" key={c.name}>
            <div className="option" aria-label={c.name}>
              <span className="option-main">
                <span className="row-label">{c.name}</span>
                <span className="row-sub">{c.description ? connectorDescription(c.name, c.description) : c.url}</span>
              </span>
            </div>
            <div className="c-actions">
              <button className={`c-icon-btn ${c.enabled ? 'on' : ''}`} data-nav={`conn-tog-${c.name}`} aria-pressed={c.enabled} onClick={() => toggle(c)}>
                <Power size={18} weight="light" /> {c.enabled ? t('claude.conn.on') : t('claude.conn.off')}
              </button>
              <button className="c-icon-btn danger" data-nav={`conn-del-${c.name}`} onClick={() => remove(c)}>
                <Trash size={18} weight="light" /> {t('claude.remove')}
              </button>
            </div>
          </div>
        ))}
      </div>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

/** /claude/conectores/tienda: conectores probados que no piden cuenta */
export function ClaudeConnectorStore() {
  const t = useT()
  const { view, setView, error, setError } = useConnectors()
  const [added, setAdded] = useState<string | null>(null)
  const add = async (item: CatalogItem) => {
    try {
      const c = await call<Connector>('claude_connector_add', { name: item.name, url: item.url, description: item.description },
        { name: item.name, url: item.url, description: item.description, enabled: true })
      setView((v) => v && { ...v, mine: [...v.mine, c] })
      setAdded(item.title)
      setError(null)
    } catch (e) { setError(String(e)) }
  }
  return (
    <Page title={t('claude.conn.storeTitle')}>
      <p className="c-intro">{t('claude.conn.storeIntro')}</p>
      <div className="c-list">
        {view?.catalog.map((item) => {
          const have = view.mine.some((m) => m.url === item.url)
          const first = view.catalog.find((x) => !view.mine.some((m) => m.url === x.url)) === item
          return (
            <div className="c-row" key={item.name}>
              <div className="option">
                <span className="option-main">
                  <span className="row-label">{item.title}</span>
                  <span className="row-sub">{connectorDescription(item.name, item.description)}</span>
                </span>
              </div>
              <div className="c-actions">
                <button className="c-icon-btn" data-nav={`store-${item.name}`} data-nav-default={first ? '' : undefined} disabled={have} onClick={() => add(item)}>
                  {have ? <><Check size={18} weight="light" /> {t('claude.conn.added')}</> : <><Plus size={18} weight="light" /> {t('claude.conn.add')}</>}
                </button>
              </div>
            </div>
          )
        })}
      </div>
      {added && <p className="notice ok">{t('claude.conn.addedNotice', { name: added })}</p>}
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

/** /claude/conectores/nuevo: añadir un servidor MCP remoto por su dirección */
export function ClaudeConnectorNew() {
  const t = useT()
  const navigate = useNavigate()
  const [name, setName] = useState('')
  const [url, setUrl] = useState('')
  const [error, setError] = useState<string | null>(null)
  const submit = async () => {
    try {
      await call('claude_connector_add', { name, url, description: null }, { name, url, description: '', enabled: true })
      navigate('/claude/conectores', { replace: true })
    } catch (e) { setError(String(e)) }
  }
  return (
    <Page title={t('claude.conn.newTitle')}>
      <p className="c-intro">{t('claude.conn.newIntro')}</p>
      <form className="c-form" onSubmit={(e) => { e.preventDefault(); submit() }}>
        <label>{t('claude.conn.urlLabel')}
          <input className="ask-input" data-nav="new-url" data-nav-default="" value={url} onChange={(e) => setUrl(e.target.value)} placeholder={t('claude.conn.urlPlaceholder')} inputMode="url" />
        </label>
        <label>{t('claude.conn.nameLabel')}
          <input className="ask-input" data-nav="new-name" value={name} onChange={(e) => setName(e.target.value)} placeholder={t('claude.conn.namePlaceholder')} />
        </label>
        <div className="hero-actions">
          <button className="btn-primary" data-nav="new-add" type="submit" disabled={!url.trim()}><Plus size={18} weight="light" /> {t('claude.conn.newTitle')}</button>
        </div>
      </form>
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}
