import { invoke } from '@tauri-apps/api/core'

// true cuando corre como app de escritorio (Tauri), false en el navegador
export const isDesktopApp = '__TAURI_INTERNALS__' in window

/**
 * Llama a una función nativa de la app. En el navegador devuelve `mock`
 * para poder ver y probar las pantallas sin la app de escritorio.
 */
export async function call<T>(cmd: string, args: Record<string, unknown> = {}, mock?: T): Promise<T> {
  if (isDesktopApp) return invoke<T>(cmd, args)
  await new Promise((r) => setTimeout(r, 80))
  return mock as T
}

/** `normal`: Windows deja de arrancar en la consola (vuelve el escritorio de siempre) */
export async function exitToDesktop(normal = false) {
  if (isDesktopApp) await invoke('exit_to_desktop', { normal })
}
