import { useEffect, useRef } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { sound, setPadKind } from '../ui/sound'
import {
  getConfig, subscribe, profileFor, effectiveButtons, threshold, repeatTiming,
  isNavLocked, navState, isTextField, requestKeyboard, type PadKey, type PadButton,
} from './padConfig'

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
  navState.focusing = true
  try { el.focus({ preventScroll: true }) } finally { navState.focusing = false }
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
    // Con mando, A sobre un campo de texto abre el teclado en pantalla
    const accept = (fromPad = false) => {
      const c = current()
      if (!c) return
      sound.select()
      if (fromPad && isTextField(c)) { c.focus(); requestKeyboard(c) } else c.click()
    }
    const back = () => { if (pathRef.current !== '/') { sound.back(); navigate(-1) } }

    const keys: Record<string, () => void> = {
      ArrowUp: () => move('up'), ArrowDown: () => move('down'),
      ArrowLeft: () => move('left'), ArrowRight: () => move('right'),
      Enter: () => accept(), Escape: back, Backspace: back,
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
      if (isNavLocked()) return // teclado en pantalla o reasignando un botón: ellos mandan
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

    // Mando (API Gamepad estándar): cualquier mando conectado vale, cada uno con su perfil.
    // Por defecto A=0, B=1, LB=4, RB=5, START=9, central=16, cruceta 12-15, stick izquierdo ejes 0/1.
    let cfg = getConfig()
    const unsubscribe = subscribe((c) => { cfg = c })
    const held = new Map<string, number>() // "índice:acción" → cuándo empezó (o tocaba repetir)
    const prevRaw = new Map<number, boolean[]>()
    const emit = (name: string, detail?: unknown) => window.dispatchEvent(new CustomEvent(name, { detail }))
    const fire = (k: PadKey) => {
      emit('padkey', k) // siempre: el teclado en pantalla la usa aunque la navegación esté en pausa
      if (isNavLocked()) return
      if (k === 'accept') accept(true)
      else if (k === 'back') back()
      else if (k === 'home') keys.Home()
      else if (k === 'menu') emit('padmenu')
      else if (k === 'pagePrev') emit('padpage', -1)
      else if (k === 'pageNext') emit('padpage', 1)
      else if (k !== 'x' && k !== 'y') move(k)
    }
    let raf = 0
    const tick = (t: number) => {
      const pads = navigator.getGamepads().filter((p): p is Gamepad => !!p)
      // Si hay un juego o el navegador delante, la consola no reacciona al mando
      if (pads.length && document.hasFocus()) {
        if (!document.documentElement.dataset.pad) setPadKind(pads[0].id)
        for (const pad of pads) {
          const p = pad.index
          const b = (i: number) => pad.buttons[i]?.pressed ?? false
          // Botones crudos (para reasignar): al pulsar y al soltar
          const prev = prevRaw.get(p) ?? []
          const now = pad.buttons.map((x) => x.pressed)
          now.forEach((on, button) => {
            if (on === !!prev[button]) return
            emit('padbutton', { pad: p, id: pad.id, button, pressed: on } satisfies PadButton)
            if (on) setPadKind(pad.id) // la leyenda enseña los botones del último mando usado
          })
          prevRaw.set(p, now)

          // START (9) + BACK (8) = xTend cambia de modo: mando → ratón → teclado → mando
          // (en PlayStation, OPTIONS + SHARE: lo cambia el proceso de fondo de la consola)
          const combo = b(8) && b(9)
          if (combo && !held.has(`${p}:combo`)) {
            held.set(`${p}:combo`, t)
            if (!held.has(`${p}:menu`)) held.set(`${p}:menu`, t) // START del combo no abre el menú
            padMode = (padMode + 1) % 3
            emit('padmode', padMode)
          } else if (!combo) held.delete(`${p}:combo`)
          // En modo ratón o teclado el mando lo usa xTend, no la consola
          if (padMode !== 0) continue

          const prof = profileFor(cfg, pad.id)
          const btn = effectiveButtons(prof)
          const th = threshold(prof.deadzone)
          const rep = repeatTiming(prof.repeat)
          // Zurdo: se navega con el stick derecho (ejes 2/3)
          const ax = (prof.leftHanded ? pad.axes[2] : pad.axes[0]) ?? 0
          const ayRaw = (prof.leftHanded ? pad.axes[3] : pad.axes[1]) ?? 0
          const ay = prof.invertY ? -ayRaw : ayRaw
          const input: [PadKey, boolean][] = [
            ['up', b(12) || ay < -th], ['down', b(13) || ay > th],
            ['left', b(14) || ax < -th], ['right', b(15) || ax > th],
            ['accept', b(btn.accept)], ['back', b(btn.back)], ['home', b(btn.home)],
            ['menu', b(btn.menu) && !(btn.menu === 9 && b(8))], // START sin BACK
            ['pagePrev', b(btn.pagePrev)], ['pageNext', b(btn.pageNext)],
            ['x', b(2)], ['y', b(3)],
          ]
          for (const [k, on] of input) {
            const key = `${p}:${k}`
            const since = held.get(key)
            if (!on) { held.delete(key); continue }
            // Direcciones siempre; borrar y mover el cursor solo con el teclado en pantalla abierto
            const repeatable = k === 'up' || k === 'down' || k === 'left' || k === 'right' ||
              ((k === 'x' || k === 'pagePrev' || k === 'pageNext') && isNavLocked())
            if (since === undefined) {
              held.set(key, t)
              window.dispatchEvent(new Event('padinput'))
              fire(k)
            } else if (repeatable && t - since > rep.delay) {
              held.set(key, t - rep.delay + rep.rate)
              fire(k)
            }
          }
        }
      }
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => { window.removeEventListener('keydown', onKey); window.removeEventListener('keyup', onKeyUp); cancelAnimationFrame(raf); unsubscribe() }
  }, [navigate])
}
