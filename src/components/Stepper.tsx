import { Minus, Plus } from '@phosphor-icons/react'
import { useT } from '../i18n'
import './Settings.css'

interface Props {
  id: string
  label: string
  value: number | null
  onChange: (v: number) => void
  step?: number
  isDefault?: boolean
}

// Control de valor 0-100 manejable con mando: botones − y + con barra
export function Stepper({ id, label, value, onChange, step = 10, isDefault }: Props) {
  const t = useT()
  const v = value ?? 0
  const set = (n: number) => onChange(Math.max(0, Math.min(100, n)))
  return (
    <div className="row">
      <span className="row-label">{label}</span>
      <div className="stepper">
        <button className="step-btn" data-nav={`${id}-minus`} onClick={() => set(v - step)} aria-label={t('settings.stepper.down', { label })}>
          <Minus size={18} weight="light" />
        </button>
        <div className="bar" role="meter" aria-valuenow={v} aria-valuemin={0} aria-valuemax={100} aria-label={label}>
          <div className="bar-fill" style={{ width: `${v}%` }} />
        </div>
        <button className="step-btn" data-nav={`${id}-plus`} data-nav-default={isDefault ? '' : undefined} onClick={() => set(v + step)} aria-label={t('settings.stepper.up', { label })}>
          <Plus size={18} weight="light" />
        </button>
        <span className="row-value">{value === null ? '—' : `${v}%`}</span>
      </div>
    </div>
  )
}
