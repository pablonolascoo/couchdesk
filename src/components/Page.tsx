import type { ReactNode } from 'react'
import { TopBar } from './TopBar'
import { HintBar } from './HintBar'
import './Settings.css'

// Esqueleto común: barra superior, contenido con scroll y leyenda del mando
export function Page({ title, children, hints }: { title: string; children: ReactNode; hints?: [string, string][] }) {
  return (
    <div className="screen">
      <TopBar title={title} />
      <div className="page-scroll">
        <div className="panel reveal">{children}</div>
      </div>
      <HintBar hints={hints ?? [['A', 'Seleccionar'], ['B', 'Atrás']]} />
    </div>
  )
}
