import { useEffect } from 'react'
import { BrowserRouter, Navigate, Route, Routes, useLocation } from 'react-router-dom'
import { IconContext } from '@phosphor-icons/react'
import { useSpatialNav } from './nav/useSpatialNav'
import { useSystemLink } from './nav/useSystemLink'
import { trackInputMode } from './ui/sound'
import { Home } from './screens/Home'
import { Library } from './screens/Library'
import { GameDetail } from './screens/GameDetail'
import { Apps, AppDetail } from './screens/Apps'
import { Cloud } from './screens/Cloud'
import { Desktop } from './screens/Desktop'
import { FilesHome, Folder, FileItem } from './screens/Files'
import { Receive, ReceiveWeb, ReceiveFtp, ReceiveLocalSend, ReceiveLocalSendDest } from './screens/Receive'
import { PhoneRemote } from './screens/PhoneRemote'
import { Emulators, EmulatorDetail } from './screens/Emulators'
import { Claude } from './screens/Claude'
import { ClaudeOverlay } from './screens/ClaudeOverlay'
import { ClaudeConversations, ClaudeConversationDelete } from './screens/claude/Conversations'
import { ClaudeConnectors, ClaudeConnectorStore, ClaudeConnectorNew } from './screens/claude/Connectors'
import { ClaudeExtensions, ClaudeExtensionStore } from './screens/claude/Extensions'
import { useClaudeNavigate } from './data/claude'
import { Browser } from './screens/Browser'
import { Music } from './screens/Music'
import { RemoteCursors } from './components/RemoteCursors'
import { PadModeToast } from './components/PadModeToast'
import { Keyboard } from './components/Keyboard'
import { ControllersScreen, PadProfilePick, ProfilesScreen, ProfileDetail, ProfileName, ProfileDelete, RemapCapture } from './screens/Controllers'
import { BootSplash } from './components/BootSplash'
import { ThemeMusic } from './components/ThemeMusic'
import { ThemesScreen, ThemeEditor, ThemeDelete, ThemeShare, ThemeImport, MenuMusicSettings } from './screens/Themes'
import { Backdrop } from './components/Backdrop'
import { BackgroundSettings, BackgroundStyles, BackgroundSource, BackgroundFiles } from './screens/BackgroundSettings'
import { Screensaver } from './screens/Screensaver'
import { ScreensaverSettings, ScreensaverCity } from './screens/ScreensaverSettings'
import { AmbienceGrid, AmbiencePlayer } from './screens/Ambience'
import { SettingsMenu, SoundSettings, DisplaySettings, PowerSettings, NetworkSettings, StartupSettings, LanguageSettings } from './screens/Settings'

function Shell() {
  useSpatialNav()
  useSystemLink()
  useClaudeNavigate()
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
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
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
        {window.location.pathname.startsWith('/asistente-flotante') ? <ClaudeOverlay />
          : window.location.pathname.startsWith('/salvapantallas') ? <Screensaver /> : <Shell />}
      </BrowserRouter>
    </IconContext.Provider>
  )
}
