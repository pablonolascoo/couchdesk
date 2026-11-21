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
import { Receive } from './screens/Receive'
import { PhoneRemote } from './screens/PhoneRemote'
import { Emulators, EmulatorDetail } from './screens/Emulators'
import { Claude } from './screens/Claude'
import { Browser } from './screens/Browser'
import { RemoteCursors } from './components/RemoteCursors'
import { PadModeToast } from './components/PadModeToast'
import { BootSplash } from './components/BootSplash'
import { SettingsMenu, SoundSettings, DisplaySettings, PowerSettings, NetworkSettings, StartupSettings } from './screens/Settings'

function Shell() {
  useSpatialNav()
  useSystemLink()
  const location = useLocation()
  useEffect(trackInputMode, [])
  return (
    <>
      <BootSplash />
      <RemoteCursors />
      <PadModeToast />
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
          <Route path="/recibir" element={<Receive />} />
          <Route path="/mando-movil" element={<PhoneRemote />} />
          <Route path="/claude" element={<Claude />} />
          <Route path="/ajustes" element={<SettingsMenu />} />
          <Route path="/ajustes/sonido" element={<SoundSettings />} />
          <Route path="/ajustes/pantalla" element={<DisplaySettings />} />
          <Route path="/ajustes/energia" element={<PowerSettings />} />
          <Route path="/ajustes/red" element={<NetworkSettings />} />
          <Route path="/ajustes/arranque" element={<StartupSettings />} />
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
        <Shell />
      </BrowserRouter>
    </IconContext.Provider>
  )
}
