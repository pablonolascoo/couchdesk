import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { PuzzlePiece, Storefront, Trash, DownloadSimple, Info } from '@phosphor-icons/react'
import { Page } from '../../components/Page'
import { call } from '../../native'
import { useT } from '../../i18n'
import { mocks, type Plugin } from '../../data/claude'
import '../Claude.css'

interface Plugins { installed: Plugin[]; available: Plugin[] }

function usePlugins() {
  const [data, setData] = useState<Plugins | null>(null)
  const [error, setError] = useState<string | null>(null)
  const reload = useCallback(() => {
    setError(null)
    call<Plugins>('claude_plugins', {}, mocks.plugins).then(setData).catch((e) => { setError(String(e)); setData({ installed: [], available: [] }) })
  }, [])
  useEffect(reload, [reload])
  return { data, error, setError, reload }
}

const Note = () => {
  const t = useT()
  return <p className="notice info c-note"><Info size={18} weight="light" /> {t('claude.ext.note')}</p>
}

/** /claude/extensiones: extensiones (plugins) instaladas en Claude Code */
export function ClaudeExtensions() {
  const t = useT()
  const navigate = useNavigate()
  const { data, error, setError, reload } = usePlugins()
  const [busy, setBusy] = useState<string | null>(null)
  const [done, setDone] = useState<string | null>(null)
  const remove = async (p: Plugin) => {
    setBusy(p.id); setDone(null)
    try { setDone(await call<string>('claude_plugin_remove', { id: p.id }, t('claude.ext.removed'))); reload() } catch (e) { setError(String(e)) }
    setBusy(null)
  }
  return (
    <Page title={t('claude.extensions')}>
      <Note />
      <div className="c-top">
        <button className="btn-primary" data-nav="ext-store" data-nav-default="" onClick={() => navigate('/claude/extensiones/tienda')}>
          <Storefront size={18} weight="light" /> {t('claude.ext.store')}
        </button>
      </div>
      {data === null && <p className="hero-meta">{t('claude.ext.asking')}</p>}
      {data?.installed.length === 0 && !error && (
        <div className="empty-state">
          <span className="empty-icon"><PuzzlePiece size={26} weight="light" /></span>
          <h2>{t('claude.ext.emptyTitle')}</h2>
          <p>{t('claude.ext.emptyText')}</p>
        </div>
      )}
      <div className="c-list">
        {data?.installed.map((p) => (
          <div className="c-row" key={p.id}>
            <div className="option">
              <span className="option-main">
                <span className="row-label">{p.id.split('@')[0]}</span>
                <span className="row-sub">{p.managed ? t('claude.ext.managed') : t('claude.ext.version', { version: p.version ?? '—' })}{p.enabled === false ? ` · ${t('claude.ext.disabled')}` : ''}</span>
              </span>
            </div>
            <div className="c-actions">
              <button className="c-icon-btn danger" data-nav={`ext-del-${p.id}`} disabled={!!p.managed || busy !== null} onClick={() => remove(p)}>
                <Trash size={18} weight="light" /> {busy === p.id ? t('claude.ext.removing') : t('claude.remove')}
              </button>
            </div>
          </div>
        ))}
      </div>
      {done && <p className="notice ok">{done}.</p>}
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}

/** /claude/extensiones/tienda: extensiones de los catálogos de Claude Code */
export function ClaudeExtensionStore() {
  const t = useT()
  const { data, error, setError, reload } = usePlugins()
  const [busy, setBusy] = useState<string | null>(null)
  const [done, setDone] = useState<string | null>(null)
  const install = async (p: Plugin) => {
    setBusy(p.id); setDone(null); setError(null)
    try { setDone(await call<string>('claude_plugin_install', { id: p.id }, t('claude.ext.installed'))); reload() } catch (e) { setError(String(e)) }
    setBusy(null)
  }
  return (
    <Page title={t('claude.ext.storeTitle')}>
      <Note />
      {data === null && <p className="hero-meta">{t('claude.ext.storeLoading')}</p>}
      {data?.available.length === 0 && !error && <p className="hero-meta">{t('claude.ext.storeEmpty')}</p>}
      <div className="c-list">
        {data?.available.map((p, i) => (
          <div className="c-row" key={p.id}>
            <div className="option">
              <span className="option-main">
                <span className="row-label">{p.name ?? p.id}</span>
                <span className="row-sub">{p.description}</span>
              </span>
            </div>
            <div className="c-actions">
              <button className="c-icon-btn" data-nav={`ext-add-${p.id}`} data-nav-default={i === 0 ? '' : undefined} disabled={busy !== null} onClick={() => install(p)}>
                <DownloadSimple size={18} weight="light" /> {busy === p.id ? t('claude.ext.installing') : t('claude.ext.install')}
              </button>
            </div>
          </div>
        ))}
      </div>
      {done && <p className="notice ok">{done}.</p>}
      {error && <p className="notice error">{error}</p>}
    </Page>
  )
}
