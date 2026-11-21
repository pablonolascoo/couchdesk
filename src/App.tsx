import { lazy, Suspense, useEffect, type ComponentType } from 'react'
import { BrowserRouter, Navigate, Route, Routes, useLocation } from 'react-router-dom'
import { IconContext } from '@phosphor-icons/react'
import { useSpatialNav } from './nav/useSpatialNav'
import { useSystemLink } from './nav/useSystemLink'
import { trackInputMode } from './ui/sound'
import { Home } from './screens/Home'
import { useClaudeNavigate } from './data/claude'
import { RemoteCursors } from './components/RemoteCursors'
import { PadModeToast } from './components/PadModeToast'
import { Keyboard } from './components/Keyboard'
import { BootSplash } from './components/BootSplash'
import { ThemeMusic } from './components/ThemeMusic'
import { Backdrop } from './components/Backdrop'
import { call } from './native'
import { useMultitaskLink } from './screens/multitaskStore'

// Arranque rápido: solo el Inicio va dentro; el resto de pantallas se carga aparte
// (y se precarga en cuanto la consola está quieta, para que abrirlas siga siendo instantáneo).
type Mod = Record<string, unknown>
const loaders: (() => Promise<Mod>)[] = []
function screen<M extends Mod>(load: () => Promise<M>) {
  loaders.push(load)
  return <K extends keyof M & string>(name: K) =>
    lazy(() => load().then((m) => ({ default: m[name] as ComponentType })))
}
const library = screen(() => import('./screens/Library'))
const gameDetail = screen(() => import('./screens/GameDetail'))
const apps = screen(() => import('./screens/Apps'))
const cloud = screen(() => import('./screens/Cloud'))
const desktop = screen(() => import('./screens/Desktop'))
const files = screen(() => import('./screens/Files'))
const receive = screen(() => import('./screens/Receive'))
const phoneRemote = screen(() => import('./screens/PhoneRemote'))
const emulators = screen(() => import('./screens/Emulators'))
const claude = screen(() => import('./screens/Claude'))
const claudeOverlay = screen(() => import('./screens/ClaudeOverlay'))
const conversations = screen(() => import('./screens/claude/Conversations'))
const connectors = screen(() => import('./screens/claude/Connectors'))
const extensions = screen(() => import('./screens/claude/Extensions'))
const browser = screen(() => import('./screens/Browser'))
const music = screen(() => import('./screens/Music'))
const controllers = screen(() => import('./screens/Controllers'))
const themes = screen(() => import('./screens/Themes'))
const background = screen(() => import('./screens/BackgroundSettings'))
const screensaver = screen(() => import('./screens/Screensaver'))
const screensaverSettings = screen(() => import('./screens/ScreensaverSettings'))
const ambience = screen(() => import('./screens/Ambience'))
const settings = screen(() => import('./screens/Settings'))
const performance_ = screen(() => import('./screens/Performance'))
const multitask = screen(() => import('./screens/Multitask'))
const Multitask = multitask('Multitask'), MultitaskForceClose = multitask('MultitaskForceClose'), MemorySettings = multitask('MemorySettings')
const DeskZones = screen(() => import('./screens/DeskZones'))('DeskZones')
const DeskZonesLayer = screen(() => import('./screens/DeskZonesLayer'))('DeskZonesLayer')

const Library = library('Library')
const GameDetail = gameDetail('GameDetail')
const Apps = apps('Apps'), AppDetail = apps('AppDetail')
const Cloud = cloud('Cloud')
const Desktop = desktop('Desktop')
const FilesHome = files('FilesHome'), Folder = files('Folder'), FileItem = files('FileItem')
const Receive = receive('Receive'), ReceiveWeb = receive('ReceiveWeb'), ReceiveFtp = receive('ReceiveFtp'),
  ReceiveLocalSend = receive('ReceiveLocalSend'), ReceiveLocalSendDest = receive('ReceiveLocalSendDest')
const PhoneRemote = phoneRemote('PhoneRemote')
const Emulators = emulators('Emulators'), EmulatorDetail = emulators('EmulatorDetail')
const Claude = claude('Claude')
const ClaudeOverlay = claudeOverlay('ClaudeOverlay')
const ClaudeConversations = conversations('ClaudeConversations'), ClaudeConversationDelete = conversations('ClaudeConversationDelete')
const ClaudeConnectors = connectors('ClaudeConnectors'), ClaudeConnectorStore = connectors('ClaudeConnectorStore'), ClaudeConnectorNew = connectors('ClaudeConnectorNew')
const ClaudeExtensions = extensions('ClaudeExtensions'), ClaudeExtensionStore = extensions('ClaudeExtensionStore')
const Browser = browser('Browser')
const Music = music('Music')
const ControllersScreen = controllers('ControllersScreen'), PadProfilePick = controllers('PadProfilePick'), ProfilesScreen = controllers('ProfilesScreen'),
  ProfileDetail = controllers('ProfileDetail'), ProfileName = controllers('ProfileName'), ProfileDelete = controllers('ProfileDelete'), RemapCapture = controllers('RemapCapture')
const ThemesScreen = themes('ThemesScreen'), ThemeEditor = themes('ThemeEditor'), ThemeDelete = themes('ThemeDelete'),
  ThemeShare = themes('ThemeShare'), ThemeImport = themes('ThemeImport'), MenuMusicSettings = themes('MenuMusicSettings')
const BackgroundSettings = background('BackgroundSettings'), BackgroundStyles = background('BackgroundStyles'),
  BackgroundSource = background('BackgroundSource'), BackgroundFiles = background('BackgroundFiles')
const Screensaver = screensaver('Screensaver')
const ScreensaverSettings = screensaverSettings('ScreensaverSettings'), ScreensaverCity = screensaverSettings('ScreensaverCity')
const AmbienceGrid = ambience('AmbienceGrid'), AmbiencePlayer = ambience('AmbiencePlayer')
const SettingsMenu = settings('SettingsMenu'), SoundSettings = settings('SoundSettings'), DisplaySettings = settings('DisplaySettings'),
  PowerSettings = settings('PowerSettings'), NetworkSettings = settings('NetworkSettings'), StartupSettings = settings('StartupSettings'),
  LanguageSettings = settings('LanguageSettings')
const PerformanceMenu = performance_('PerformanceMenu'), PerformanceMotion = performance_('PerformanceMotion'),
  PerformanceWindows = performance_('PerformanceWindows'), PerformanceGame = performance_('PerformanceGame'),
  PerformanceUpdates = performance_('PerformanceUpdates'), PerformanceAdvanced = performance_('PerformanceAdvanced')

/** Primer pintado: se apunta el tiempo (perf.rs) y se precargan las demás pantallas sin estorbar */
function useAppReady() {
  useEffect(() => {
    requestAnimationFrame(() => setTimeout(() => {
      const pageMs = performance.now()
      call('perf_app_ready', { pageMs }).catch(() => { /* versión sin el comando */ })
      if (!/^\/(asistente-flotante|salvapantallas)/.test(location.pathname)) {
        const idle = window.requestIdleCallback ?? ((fn: () => void) => setTimeout(fn, 1500))
        idle(() => loaders.forEach((l) => l().catch(() => {})))
      }
    }))
  }, [])
}

function Shell() {
  useSpatialNav()
  useSystemLink()
  useClaudeNavigate()
  useAppReady()
  useMultitaskLink()
  const location = useLocation()
  useEffect(trackInputMode, [])
  return (
    <>
      <Backdrop />
      <BootSplash />
      <ThemeMusic />
      <RemoteCursors />
      <PadModeToast />
      <Keyboard />
      {/* La clave reinicia la animación de entrada en cada pantalla */}
      <div className="route" key={location.pathname}>
        <Suspense fallback={null}>
        <Routes location={location}>
          <Route path="/" element={<Home />} />
          <Route path="/biblioteca" element={<Library />} />
          <Route path="/juego/:id" element={<GameDetail />} />
          <Route path="/nube" element={<Cloud />} />
          <Route path="/aplicaciones" element={<Apps />} />
          <Route path="/aplicaciones/:id" element={<AppDetail />} />
          <Route path="/emuladores" element={<Emulators />} />
          <Route path="/emuladores/:id" element={<EmulatorDetail />} />
          <Route path="/archivos" element={<FilesHome />} />
          <Route path="/archivos/carpeta" element={<Folder />} />
          <Route path="/archivos/elemento" element={<FileItem />} />
          <Route path="/navegador" element={<Browser />} />
          <Route path="/musica" element={<Music />} />
          <Route path="/recibir" element={<Receive />} />
          <Route path="/recibir/navegador" element={<ReceiveWeb />} />
          <Route path="/recibir/ftp" element={<ReceiveFtp />} />
          <Route path="/recibir/localsend" element={<ReceiveLocalSend />} />
          <Route path="/recibir/localsend/destino" element={<ReceiveLocalSendDest />} />
          <Route path="/mando-movil" element={<PhoneRemote />} />
          <Route path="/claude" element={<Claude />} />
          <Route path="/claude/conversaciones" element={<ClaudeConversations />} />
          <Route path="/claude/conversaciones/:id/borrar" element={<ClaudeConversationDelete />} />
          <Route path="/claude/conectores" element={<ClaudeConnectors />} />
          <Route path="/claude/conectores/tienda" element={<ClaudeConnectorStore />} />
          <Route path="/claude/conectores/nuevo" element={<ClaudeConnectorNew />} />
          <Route path="/claude/extensiones" element={<ClaudeExtensions />} />
          <Route path="/claude/extensiones/tienda" element={<ClaudeExtensionStore />} />
          <Route path="/ajustes" element={<SettingsMenu />} />
          <Route path="/ajustes/sonido" element={<SoundSettings />} />
          <Route path="/ajustes/pantalla" element={<DisplaySettings />} />
          <Route path="/ajustes/energia" element={<PowerSettings />} />
          <Route path="/ajustes/red" element={<NetworkSettings />} />
          <Route path="/ajustes/arranque" element={<StartupSettings />} />
          <Route path="/ajustes/idioma" element={<LanguageSettings />} />
          <Route path="/ajustes/mandos" element={<ControllersScreen />} />
          <Route path="/ajustes/mandos/mando/:pad" element={<PadProfilePick />} />
          <Route path="/ajustes/mandos/perfiles" element={<ProfilesScreen />} />
          <Route path="/ajustes/mandos/perfil/:id" element={<ProfileDetail />} />
          <Route path="/ajustes/mandos/perfil/:id/nombre" element={<ProfileName />} />
          <Route path="/ajustes/mandos/perfil/:id/borrar" element={<ProfileDelete />} />
          <Route path="/ajustes/mandos/perfil/:id/boton/:action" element={<RemapCapture />} />
          <Route path="/ajustes/fondo" element={<BackgroundSettings />} />
          <Route path="/ajustes/fondo/animado" element={<BackgroundStyles />} />
          <Route path="/ajustes/fondo/:kind" element={<BackgroundSource />} />
          <Route path="/ajustes/fondo/:kind/pc" element={<BackgroundFiles />} />
          <Route path="/ajustes/temas" element={<ThemesScreen />} />
          <Route path="/ajustes/temas/nuevo" element={<ThemeEditor />} />
          <Route path="/ajustes/temas/importar" element={<ThemeImport />} />
          <Route path="/ajustes/temas/musica" element={<MenuMusicSettings />} />
          <Route path="/ajustes/temas/:id/editar" element={<ThemeEditor />} />
          <Route path="/ajustes/temas/:id/compartir" element={<ThemeShare />} />
          <Route path="/ajustes/temas/:id/borrar" element={<ThemeDelete />} />
          <Route path="/ajustes/salvapantallas" element={<ScreensaverSettings />} />
          <Route path="/ajustes/salvapantallas/ciudad" element={<ScreensaverCity />} />
          <Route path="/ambiente" element={<AmbienceGrid />} />
          <Route path="/ambiente/:id" element={<AmbiencePlayer />} />
          <Route path="/escritorio" element={<Desktop />} />
          <Route path="/ajustes/escritorio-organizado/*" element={<DeskZones />} />
          <Route path="/ajustes/rendimiento" element={<PerformanceMenu />} />
          <Route path="/ajustes/rendimiento/movimiento" element={<PerformanceMotion />} />
          <Route path="/ajustes/rendimiento/windows" element={<PerformanceWindows />} />
          <Route path="/ajustes/rendimiento/juegos" element={<PerformanceGame />} />
          <Route path="/ajustes/rendimiento/actualizaciones" element={<PerformanceUpdates />} />
          <Route path="/ajustes/rendimiento/avanzado" element={<PerformanceAdvanced />} />
          <Route path="/ajustes/rendimiento/memoria" element={<MemorySettings />} />
          <Route path="/multitarea" element={<Multitask />} />
          <Route path="/multitarea/forzar/:hwnd" element={<MultitaskForceClose />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
        </Suspense>
      </div>
    </>
  )
}

export default function App() {
  return (
    <IconContext.Provider value={{ weight: 'light' }}>
      <BrowserRouter>
        {/* La ventana flotante de Claude ("asistente") solo lleva su tarjeta */}
        {/* La ventana "salvapantallas" solo lleva el salvapantallas */}
        <Suspense fallback={null}>
          {window.location.pathname.startsWith('/asistente-flotante') ? <ClaudeOverlay />
            : window.location.pathname.startsWith('/salvapantallas') ? <Screensaver />
            : window.location.pathname.startsWith('/capa-escritorio') ? <DeskZonesLayer /> : <Shell />}
        </Suspense>
      </BrowserRouter>
    </IconContext.Provider>
  )
}
