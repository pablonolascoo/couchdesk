import { useEffect, useState } from 'react'
import { ArrowUpRight, ArrowCounterClockwise, Desktop as DesktopIcon } from '@phosphor-icons/react'
import { TopBar } from '../components/TopBar'
import { HintBar } from '../components/HintBar'
import { call, exitToDesktop, isDesktopApp } from '../native'
import { useT } from '../i18n'
import './Desktop.css'

export function Desktop() {
  const t = useT()
  const [consoleBoots, setConsoleBoots] = useState<boolean | null>(null)
  const [confirm, setConfirm] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    call<{ enabled: boolean }>('shell_status', {}, { enabled: true }).then((s) => setConsoleBoots(s.enabled)).catch(() => setConsoleBoots(false))
  }, [])

  const goNormal = () => {
    if (consoleBoots && !confirm) return setConfirm(true) // pulsar dos veces para confirmar
    exitToDesktop(true).catch((e) => setError(String(e)))
  }

  return (
    <div className="screen">
      <TopBar title={t('desktop.title')} />
      <section className="desk">
        <span className="eyebrow reveal">Windows</span>
        <h2 className="hero-title reveal" style={{ ['--i' as string]: 1 }}>{t('desktop.heading')}</h2>
        {!isDesktopApp && <p className="hero-meta">{t('shell.onlyInstalled')}</p>}

        <div className="desk-options">
          <button className="desk-card reveal" style={{ ['--i' as string]: 2 }} data-nav="now" data-nav-default="" onClick={() => exitToDesktop().catch((e) => setError(String(e)))} disabled={!isDesktopApp}>
            <span className="desk-icon"><DesktopIcon size={28} /></span>
            <span className="desk-title">{t('desktop.now.title')}</span>
            <span className="desk-text">
              {t('desktop.now.text')}
              {consoleBoots ? ` ${t('desktop.now.boots')}` : ''}
            </span>
            <span className="desk-go"><ArrowUpRight size={18} /></span>
          </button>

          <button className={`desk-card${confirm ? ' desk-warn' : ''} reveal`} style={{ ['--i' as string]: 3 }} data-nav="normal" onClick={goNormal} disabled={!isDesktopApp}>
            <span className="desk-icon"><ArrowCounterClockwise size={28} /></span>
            <span className="desk-title">{t('desktop.normal.title')}</span>
            <span className="desk-text">
              {confirm
                ? t('desktop.normal.confirm')
                : consoleBoots === false
                  ? t('desktop.normal.already')
                  : t('desktop.normal.text')}
            </span>
            <span className="desk-go"><ArrowUpRight size={18} /></span>
          </button>
        </div>

        <p className="desk-foot reveal" style={{ ['--i' as string]: 4 }}>
          {t('desktop.foot')}
        </p>
        {error && <p className="notice error">{error}</p>}
      </section>
      <HintBar hints={[['A', t('shell.hint.select')], ['B', t('common.back')]]} />
    </div>
  )
}
