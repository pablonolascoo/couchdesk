import type { Icon } from '@phosphor-icons/react'
import {
  SquaresFour, GameController, FolderSimple, Globe, DownloadSimple,
  GearSix, AppWindow, Cloud, WindowsLogo,
} from '@phosphor-icons/react'
import type { Brand } from '../ui/brands'

/** Cada sección tiene su propio color (`tone`) y su dibujo; nunca violeta */
export interface Section { path: string; label: string; hint: string; icon: Icon; tone: string; brand?: Brand }

export const sections: Section[] = [
  { path: '/biblioteca', label: 'Biblioteca', hint: 'Todos tus juegos', icon: SquaresFour, tone: '#b7791f' },
  { path: '/nube', label: 'En la nube', hint: 'Xbox Cloud y PS5', icon: Cloud, tone: '#1f7a3a' },
  { path: '/aplicaciones', label: 'Aplicaciones', hint: 'Todo lo instalado', icon: AppWindow, tone: '#3a5a8c' },
  { path: '/emuladores', label: 'Emuladores', hint: 'Consolas clásicas', icon: GameController, tone: '#b8452f' },
  { path: '/archivos', label: 'Archivos', hint: 'Explorar el PC', icon: FolderSimple, tone: '#a86a12' },
  { path: '/navegador', label: 'Navegador', hint: 'Webs con mando', icon: Globe, tone: '#1d7480' },
  { path: '/recibir', label: 'Recibir', hint: 'FTP en tu red', icon: DownloadSimple, tone: '#4f6b2a' },
  { path: '/claude', label: 'Claude', hint: 'Tu asistente', icon: Globe, tone: '#c15f3c', brand: 'claude' },
  { path: '/ajustes', label: 'Ajustes', hint: 'Sonido, brillo, red…', icon: GearSix, tone: '#5b5148' },
  { path: '/escritorio', label: 'Escritorio', hint: 'Volver a Windows', icon: WindowsLogo, tone: '#0f6cbd' },
]
