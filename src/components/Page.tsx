import type { ReactNode } from 'react'
import { TopBar } from './TopBar'
import { HintBar } from './HintBar'
import { useT } from '../i18n'
import './Settings.css'

// Esqueleto común: barra superior, contenido con scroll y leyenda del mando
// `wide`: para pantallas de cuadrícula (navegador, ambiente, temas…), que usan todo el ancho
export function Page({ title, children, hints, wide }: { title: string; children: ReactNode; hints?: [string, string][]; wide?: boolean }) {
  const t = useT()
  return (
    <div className="screen">
      <TopBar title={title} />
      <div className="page-scroll">
        <div className={`panel reveal${wide ? ' panel-wide' : ''}`}>{children}</div>
      </div>
      <HintBar hints={hints ?? [['A', t('shell.hint.select')], ['B', t('common.back')]]} />
    </div>
  )
}
