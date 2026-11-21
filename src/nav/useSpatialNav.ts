import { useEffect, useRef } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { sound, setPadKind } from '../ui/sound'

type Dir = 'up' | 'down' | 'left' | 'right'

// Modo de xTend: 0 mando (la consola responde), 1 ratón, 2 teclado
let padMode = 0

// Foco recordado por pantalla, para que "atrás" vuelva donde estabas
const lastFocus = new Map<string, string>()

const items = () => Array.from(document.querySelectorAll<HTMLElement>('[data-nav]'))

function setFocus(el: HTMLElement | null, path: string, remember = true) {
  if (!el) return
  // Atributo (no clase) para que React no lo borre al volver a pintar
  items().forEach((n) => n.removeAttribute('data-focused'))
  el.setAttribute('data-focused', '')
  el.focus({ preventScroll: true })
  el.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'smooth' })
  if (remember) lastFocus.set(path, el.dataset.nav!)
  el.dispatchEvent(new CustomEvent('navfocus', { bubbles: true }))
}

function nearest(from: HTMLElement, dir: Dir): HTMLElement | null {
  const a = from.getBoundingClientRect()
  const ax = a.left + a.width / 2
  const ay = a.top + a.height / 2
  let best: HTMLElement | null = null
  let bestScore = Infinity
  for (const el of items()) {
    if (el === from) continue
    const b = el.getBoundingClientRect()
    const bx = b.left + b.width / 2
    const by = b.top + b.height / 2
    const dx = bx - ax
    const dy = by - ay
    const ok =
      (dir === 'right' && dx > 4) || (dir === 'left' && dx < -4) ||
      (dir === 'down' && dy > 4) || (dir === 'up' && dy < -4)
    if (!ok) continue
    const main = dir === 'left' || dir === 'right' ? Math.abs(dx) : Math.abs(dy)
    const side = dir === 'left' || dir === 'right' ? Math.abs(dy) : Math.abs(dx)
    const score = main + side * 2
    if (score < bestScore) { bestScore = score; best = el }
  }
  return best
}

// Navegación por teclado y mando de Xbox (API Gamepad estándar)
export function useSpatialNav() {
  const navigate = useNavigate()
  const location = useLocation()
  const pathname = location.pathname + location.search // cada carpeta cuenta como pantalla propia
  const pathRef = useRef(pathname)
  pathRef.current = pathname

  // Al entrar en una pantalla: recuperar el foco guardado o el primero
  // Si el contenido llega más tarde (p. ej. la lista de juegos), espera hasta 3 s a que aparezca
  useEffect(() => {
    const preferred = () => {
      const saved = lastFocus.get(pathname)
      return (saved && document.querySelector<HTMLElement>(`[data-nav="${CSS.escape(saved)}"]`)) ||
        document.querySelector<HTMLElement>('[data-nav-default]')
    }
    let observer: MutationObserver | null = null
    let giveUp = 0
    const id = setTimeout(() => {
      const el = preferred()
      if (el) return setFocus(el, pathname)
      setFocus(items()[0] ?? null, pathname, false)
      observer = new MutationObserver(() => {
        const late = preferred()
        if (late) { observer?.disconnect(); setFocus(late, pathname) }
      })
      observer.observe(document.body, { childList: true, subtree: true })
      giveUp = window.setTimeout(() => observer?.disconnect(), 3000)
    })
    return () => { clearTimeout(id); clearTimeout(giveUp); observer?.disconnect() }
  }, [pathname])

  useEffect(() => {
    const current = () =>
      document.querySelector<HTMLElement>('[data-nav][data-focused]') ??
      document.querySelector<HTMLElement>('[data-nav-default]') ?? items()[0]
    const move = (dir: Dir) => {
      const c = current()
      if (!c) return
      // Dirección fija opcional: data-nav-up / -down / -left / -right="id"
      const forced = c.dataset[`nav${dir[0].toUpperCase()}${dir.slice(1)}`]
      const next = (forced && document.querySelector<HTMLElement>(`[data-nav="${CSS.escape(forced)}"]`)) || nearest(c, dir)
      if (next) { sound.move(); setFocus(next, pathRef.current) }
    }
    const accept = () => { const c = current(); if (c) { sound.select(); c.click() } }
    const back = () => { if (pathRef.current !== '/') { sound.back(); navigate(-1) } }

    const keys: Record<string, () => void> = {
      ArrowUp: () => move('up'), ArrowDown: () => move('down'),
      ArrowLeft: () => move('left'), ArrowRight: () => move('right'),
      Enter: accept, Escape: back, Backspace: back,
      Home: () => { if (pathRef.current !== '/') { sound.back(); navigate('/') } },
    }
    // Tab / Mayús+Tab: siguiente o anterior en orden de lectura
    const step = (delta: number) => {
      const list = items().filter((el) => !(el as HTMLButtonElement).disabled)
      if (!list.length) return
      const i = list.indexOf(current() as HTMLElement)
      sound.move()
      setFocus(list[(i + delta + list.length) % list.length], pathRef.current)
    }

    const onKey = (e: KeyboardEvent) => {
      if (e.altKey || e.ctrlKey || e.metaKey) return // atajos del sistema intactos
      // Escribiendo en un campo de texto: solo arriba/abajo/Esc/Tab sirven para salir de él
      const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement
      if (typing && !['ArrowUp', 'ArrowDown', 'Escape', 'Tab'].includes(e.key)) return
      if (e.key === 'Tab') { e.preventDefault(); return step(e.shiftKey ? -1 : 1) }
      if (e.key === ' ') { e.preventDefault(); return accept() }
      const fn = keys[e.key]
      if (fn) { e.preventDefault(); fn() }
    }
    // El botón nativo también "pulsa" al soltar Espacio: se evita para no hacerlo dos veces
    const onKeyUp = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement
      if (e.key === ' ' && !typing) e.preventDefault()
    }
    window.addEventListener('keydown', onKey)
    window.addEventListener('keyup', onKeyUp)

    // Mando: A=0, B=1, cruceta 12-15, stick izquierdo ejes 0/1
    const held = new Map<string, number>()
    let raf = 0
    const tick = (t: number) => {
      const pad = navigator.getGamepads().find((p) => p)
      // Si hay un juego o el navegador delante, la consola no reacciona al mando
      if (pad && document.hasFocus()) {
        setPadKind(pad.id)
        const b = (i: number) => pad.buttons[i]?.pressed ?? false
        // START (9) + BACK (8) = xTend cambia de modo: mando → ratón → teclado → mando
        // (en PlayStation, OPTIONS + SHARE: lo cambia el proceso de fondo de la consola)
        const combo = b(8) && b(9)
        if (combo && !held.has('combo')) {
          held.set('combo', t)
          padMode = (padMode + 1) % 3
          window.dispatchEvent(new CustomEvent('padmode', { detail: padMode }))
        } else if (!combo) held.delete('combo')
        // En modo ratón o teclado el mando lo usa xTend, no la consola
        if (padMode !== 0) { raf = requestAnimationFrame(tick); return }
        const [lx = 0, ly = 0] = pad.axes
        const input: Record<string, boolean> = {
          up: b(12) || ly < -0.6, down: b(13) || ly > 0.6,
          left: b(14) || lx < -0.6, right: b(15) || lx > 0.6,
          a: b(0), b: b(1), home: b(16), // 16 = botón Xbox / PS central
        }
        for (const [k, on] of Object.entries(input)) {
          const since = held.get(k)
          if (!on) { held.delete(k); continue }
          const repeatable = k !== 'a' && k !== 'b' && k !== 'home'
          if (since === undefined) {
            held.set(k, t)
            window.dispatchEvent(new Event('padinput'))
            if (k === 'a') accept()
            else if (k === 'b') back()
            else if (k === 'home') keys.Home()
            else move(k as Dir)
          } else if (repeatable && t - since > 350) {
            held.set(k, t - 350 + 120)
            move(k as Dir)
          }
        }
      }
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => { window.removeEventListener('keydown', onKey); window.removeEventListener('keyup', onKeyUp); cancelAnimationFrame(raf) }
  }, [navigate])
}
