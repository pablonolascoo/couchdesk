import { useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { isDesktopApp, call } from '../native'

const editable = (el: EventTarget | null) =>
  el instanceof HTMLTextAreaElement ||
  (el instanceof HTMLInputElement && !/^(button|checkbox|radio|range|submit|reset|file|color|image|hidden)$/i.test(el.type))

// Une la interfaz con el resto del sistema:
// · "go-home" (Esc mantenido, botón Xbox, Inicio del móvil…) → pantalla de Inicio
// · Al escribir en un campo, los móviles conectados abren su teclado; al salir, lo cierran
export function useSystemLink() {
  const navigate = useNavigate()
  useEffect(() => {
    if (!isDesktopApp) return
    let off: (() => void) | undefined
    import('@tauri-apps/api/event').then(({ listen }) =>
      Promise.all([
        listen('go-home', () => navigate('/')),
        // Alguien envía con LocalSend: se abre su pantalla para aceptar o rechazar
        listen('localsend-request', () => navigate('/recibir/localsend')),
      ]).then((u) => { off = () => u.forEach((f) => f()) }),
    )
    const focusIn = (e: FocusEvent) => { if (editable(e.target)) call('remote_mode', { tab: 'mouse', kbd: true }).catch(() => {}) }
    const focusOut = (e: FocusEvent) => { if (editable(e.target)) call('remote_mode', { tab: 'gp', kbd: false }).catch(() => {}) }
    window.addEventListener('focusin', focusIn)
    window.addEventListener('focusout', focusOut)
    return () => { off?.(); window.removeEventListener('focusin', focusIn); window.removeEventListener('focusout', focusOut) }
  }, [navigate])
}
