import { useEffect, useState } from 'react'
import { CheckCircle, Circle, MoonStars, LockKeyOpen, Lightning, ArrowSquareOut } from '@phosphor-icons/react'
import { useT } from '../i18n'
import { call } from '../native'
import { loadSettings, windowsSaverSet, windowsSaverStatus } from '../data/screensaver'
import './Themes.css'

interface ShellStatus { enabled: boolean }

/** Ajustes → Arranque: lista corta para que CouchDesk sea "tu Windows" */
export function WindowsChecklist() {
  const t = useT()
  const [shell, setShell] = useState<boolean | null>(null)
  const [saver, setSaver] = useState<boolean | null>(null)
  const [msg, setMsg] = useState<{ kind: 'ok' | 'error' | 'info'; text: string } | null>(null)

  useEffect(() => {
    call<ShellStatus>('shell_status', {}, { enabled: false }).then((s) => setShell(s.enabled)).catch(() => setShell(false))
    windowsSaverStatus().then((s) => setSaver(s.enabled)).catch(() => setSaver(false))
    // El interruptor de arriba cambia el arranque: se vuelve a mirar al pulsarlo
    const again = () => window.setTimeout(() => call<ShellStatus>('shell_status', {}, { enabled: false }).then((s) => setShell(s.enabled)).catch(() => {}), 400)
    document.addEventListener('click', again)
    return () => document.removeEventListener('click', again)
  }, [])

  const toggleSaver = async () => {
    try {
      const s = await loadSettings()
      const r = await windowsSaverSet(!saver, s.afterMin)
      setSaver(r.enabled)
      setMsg(null)
    } catch (e) { setMsg({ kind: 'error', text: String(e) }) }
  }
  const openSignIn = () => call('open_windows_settings', { page: 'signinoptions' }).catch((e) => setMsg({ kind: 'error', text: String(e) }))
  const optimize = () => call('run_optimizer', { undo: false })
    .then(() => setMsg({ kind: 'info', text: t('themes.win.optimize.started') }))
    .catch((e) => setMsg({ kind: 'error', text: String(e) }))

  const Mark = ({ on }: { on: boolean | null }) => on
    ? <span className="option-check"><CheckCircle size={18} weight="fill" /> {t('themes.win.done')}</span>
    : <span className="wc-todo"><Circle size={18} weight="light" /> {t('themes.win.todo')}</span>

  return (
    <section className="wc">
      <h3 className="label th-label">{t('themes.win.title')}</h3>
      <p className="hero-meta wc-intro">{t('themes.win.intro')}</p>

      <div className="wc-item">
        <span className="row-label">{t('themes.win.shell')}<span className="row-sub">{t('themes.win.shell.sub')}</span></span>
        <Mark on={shell} />
      </div>

      <button className="option wc-item" data-nav="wc-saver" aria-pressed={!!saver} onClick={toggleSaver}>
        <span className="th-link-main"><MoonStars size={22} weight="light" />
          <span className="row-label">{t('themes.win.saver')}<span className="row-sub">{t('themes.win.saver.sub')}</span></span>
        </span>
        <Mark on={saver} />
      </button>

      <button className="option wc-item" data-nav="wc-signin" onClick={openSignIn}>
        <span className="th-link-main"><LockKeyOpen size={22} weight="light" />
          <span className="row-label">{t('themes.win.signin')}<span className="row-sub">{t('themes.win.signin.sub')}</span></span>
        </span>
        <ArrowSquareOut size={18} weight="light" />
      </button>

      <button className="option wc-item" data-nav="wc-optimize" onClick={optimize}>
        <span className="th-link-main"><Lightning size={22} weight="light" />
          <span className="row-label">{t('themes.win.optimize')}<span className="row-sub">{t('themes.win.optimize.sub')}</span></span>
        </span>
        <ArrowSquareOut size={18} weight="light" />
      </button>
      {msg && <p className={`notice ${msg.kind}`}>{msg.text}</p>}
    </section>
  )
}
