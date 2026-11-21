import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import '@fontsource-variable/geist'
import '@fontsource-variable/geist-mono'
import '@fontsource/instrument-serif'
import './index.css'
import './ui/shared.css'
import App from './App.tsx'
import { initI18n } from './i18n'

// Primero el idioma (del sistema o el elegido en Ajustes), luego la consola
initI18n().finally(() =>
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  ),
)
