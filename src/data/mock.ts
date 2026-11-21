import type { Icon } from '@phosphor-icons/react'
import {
  SquaresFour, GameController, FolderSimple, Globe, DownloadSimple,
  GearSix, Sparkle, Desktop, AppWindow, Cloud,
} from '@phosphor-icons/react'

export interface Section { path: string; label: string; hint: string; icon: Icon }

export const sections: Section[] = [
  { path: '/biblioteca', label: 'Biblioteca', hint: 'Todos tus juegos', icon: SquaresFour },
  { path: '/nube', label: 'En la nube', hint: 'Xbox Cloud y PS5', icon: Cloud },
  { path: '/aplicaciones', label: 'Aplicaciones', hint: 'Todo lo instalado', icon: AppWindow },
  { path: '/emuladores', label: 'Emuladores', hint: 'Consolas clásicas', icon: GameController },
  { path: '/archivos', label: 'Archivos', hint: 'Explorar el PC', icon: FolderSimple },
  { path: '/navegador', label: 'Navegador', hint: 'Webs con mando', icon: Globe },
  { path: '/recibir', label: 'Recibir', hint: 'FTP en tu red', icon: DownloadSimple },
  { path: '/claude', label: 'Claude', hint: 'Tu asistente', icon: Sparkle },
  { path: '/ajustes', label: 'Ajustes', hint: 'Sonido, brillo, red…', icon: GearSix },
  { path: '/escritorio', label: 'Escritorio', hint: 'Volver a Windows', icon: Desktop },
]
