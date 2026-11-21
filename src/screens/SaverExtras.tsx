import { useEffect, useState } from 'react'
import { CheckCircle } from '@phosphor-icons/react'
import { Stepper } from '../components/Stepper'
import { useT } from '../i18n'
import { windowsSaverSet, windowsSaverStatus, type SaverSettings } from '../data/screensaver'

/** Ajustes → Salvapantallas: sonido de ambiente y usarlo como salvapantallas de Windows */
export function SaverExtras({ s, update }: { s: SaverSettings; update: (p: Partial<SaverSettings>) => void }) {
  const t = useT()
  const [win, setWin] = useState<boolean | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => { windowsSaverStatus().then((w) => setWin(w.enabled)).catch(() => setWin(false)) }, [])
  // Si cambia el tiempo de espera, Windows también lo usa
  useEffect(() => {
    if (win) windowsSaverSet(true, s.afterMin).catch((e) => setError(String(e)))
  }, [s.afterMin]) // eslint-disable-line react-hooks/exhaustive-deps

  const toggleWin = () => {
    setError(null)
    windowsSaverSet(!win, s.afterMin).then((w) => setWin(w.enabled)).catch((e) => setError(String(e)))
  }

  const Toggle = ({ id, on, onClick }: { id: string; on: boolean; onClick: () => void }) => (
    <button className={`option sv-toggle ${on ? 'is-on' : ''}`} data-nav={id} aria-pressed={on} onClick={onClick}>
      {on ? <><CheckCircle size={18} weight="fill" /> {t('themes.on')}</> : t('themes.off')}
    </button>
  )

  return (
    <>
      <h3 className="label sv-label">{t('themes.saver.sound')}</h3>
      <div className="row">
        <span className="row-label">{t('themes.saver.sound.toggle')}<span className="row-sub">{t('themes.saver.sound.sub')}</span></span>
        <Toggle id="sv-sound" on={!!s.sound} onClick={() => update({ sound: !s.sound })} />
      </div>
      {s.sound && (
        <Stepper id="sv-sound-vol" label={t('themes.saver.sound.volume')} value={s.soundVolume ?? 50} onChange={(soundVolume) => update({ soundVolume })} />
      )}

      <h3 className="label sv-label">{t('themes.saver.windows')}</h3>
      <div className="row">
        <span className="row-label">{t('themes.saver.windows.toggle')}<span className="row-sub">{t('themes.saver.windows.sub')}</span></span>
        <Toggle id="sv-windows" on={!!win} onClick={toggleWin} />
      </div>
      {error && <p className="notice error">{error}</p>}
    </>
  )
}
