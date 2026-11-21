import { useState, type CSSProperties } from 'react'
import { ArrowUpRight, Cloud as CloudIcon, GameController } from '@phosphor-icons/react'
import { BrandLogo } from '../ui/brands'
import './Cloud.css'
import { TopBar } from '../components/TopBar'
import { HintBar } from '../components/HintBar'
import { call, isDesktopApp } from '../native'
import { useT } from '../i18n'
import './Desktop.css'

// Jugar sin instalar: Xbox Cloud Gaming y tu PS5 por la red de casa
export function Cloud() {
  const t = useT()
  const [busy, setBusy] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const run = (id: string, cmd: string, msg: string) => {
    setError(null)
    setBusy(id)
    call(cmd, {}).catch((e) => setError(`${msg}: ${e}`)).finally(() => setBusy(null))
  }
  const i = (n: number) => ({ '--i': n }) as CSSProperties

  return (
    <div className="screen">
      <TopBar title={t('cloud.title')} />
      <section className="desk">
        <span className="eyebrow reveal">{t('cloud.eyebrow')}</span>
        <h2 className="hero-title reveal" style={i(1)}>{t('cloud.heading')}</h2>
        {!isDesktopApp && <p className="hero-meta">{t('shell.onlyInstalled')}</p>}

        <div className="desk-options">
          <button className="desk-card cloud-card xbox reveal" style={i(2)} data-nav="xcloud" data-nav-default="" disabled={!isDesktopApp} onClick={() => run('xcloud', 'cloud_open', t('cloud.xbox.error'))}>
            <span className="cloud-art" aria-hidden><GameController size={240} weight="fill" /></span>
            <span className="desk-icon"><CloudIcon size={28} weight="fill" /></span>
            <span className="desk-title">Xbox Cloud Gaming</span>
            <span className="desk-text">
              {busy === 'xcloud' ? t('shell.opening') : t('cloud.xbox.text')}
            </span>
            <span className="desk-go"><ArrowUpRight size={18} /></span>
          </button>

          <button className="desk-card cloud-card ps reveal" style={i(3)} data-nav="ps5" disabled={!isDesktopApp || busy === 'ps5'} onClick={() => run('ps5', 'open_remote_play', t('cloud.ps.error'))}>
            <span className="cloud-art" aria-hidden><BrandLogo name="playstation" size={230} color="currentColor" /></span>
            <span className="desk-icon"><BrandLogo name="playstation" size={26} color="currentColor" /></span>
            <span className="desk-title">PS5 Remote Play</span>
            <span className="desk-text">
              {busy === 'ps5' ? t('cloud.ps.busy') : t('cloud.ps.text')}
            </span>
            <span className="desk-go"><ArrowUpRight size={18} /></span>
          </button>
        </div>

        <p className="desk-foot reveal" style={i(4)}>
          {t('cloud.foot')}
        </p>
        {error && <p className="notice error">{error}</p>}
      </section>
      <HintBar hints={[['A', t('cloud.hint.play')], ['B', t('common.back')]]} />
    </div>
  )
}
