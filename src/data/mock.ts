import type { Icon } from '@phosphor-icons/react'
import {
  SquaresFour, GameController, FolderSimple, Globe, DownloadSimple,
  GearSix, AppWindow, Cloud, WindowsLogo, MusicNotes,
} from '@phosphor-icons/react'
import type { Brand } from '../ui/brands'
import { t } from '../i18n'

/** Cada sección tiene su propio color (`tone`) y su dibujo; nunca violeta */
export interface Section { path: string; label: string; hint: string; icon: Icon; tone: string; brand?: Brand }

/** Secciones de Inicio, con los textos del idioma actual */
export const sections = (): Section[] => [
  { path: '/biblioteca', label: t('home.section.library'), hint: t('home.section.library.hint'), icon: SquaresFour, tone: '#b7791f' },
  { path: '/nube', label: t('home.section.cloud'), hint: t('home.section.cloud.hint'), icon: Cloud, tone: '#1f7a3a' },
  { path: '/aplicaciones', label: t('home.section.apps'), hint: t('home.section.apps.hint'), icon: AppWindow, tone: '#3a5a8c' },
  { path: '/emuladores', label: t('home.section.emulators'), hint: t('home.section.emulators.hint'), icon: GameController, tone: '#b8452f' },
  { path: '/archivos', label: t('home.section.files'), hint: t('home.section.files.hint'), icon: FolderSimple, tone: '#a86a12' },
  { path: '/navegador', label: t('home.section.browser'), hint: t('home.section.browser.hint'), icon: Globe, tone: '#1d7480' },
  { path: '/musica', label: t('home.section.music'), hint: t('home.section.music.hint'), icon: MusicNotes, tone: '#a3324f' },
  { path: '/recibir', label: t('home.section.receive'), hint: t('home.section.receive.hint'), icon: DownloadSimple, tone: '#4f6b2a' },
  { path: '/claude', label: 'Claude', hint: t('home.section.claude.hint'), icon: Globe, tone: '#c15f3c', brand: 'claude' },
  { path: '/ajustes', label: t('home.section.settings'), hint: t('home.section.settings.hint'), icon: GearSix, tone: '#5b5148' },
  { path: '/escritorio', label: t('home.section.desktop'), hint: t('home.section.desktop.hint'), icon: WindowsLogo, tone: '#0f6cbd' },
]
