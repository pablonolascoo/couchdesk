import { useCallback, useEffect, useMemo, useReducer, useRef, useState, type ReactNode } from 'react'
import { useT, getLanguage } from '../i18n'
import { useLocation } from 'react-router-dom'
import { ArrowFatUp, Backspace, Check, ArrowLeft, ArrowRight } from '@phosphor-icons/react'
import { isTextField, lockNav, unlockNav, navState, type PadKey } from '../nav/padConfig'
import { sound, inputMode, padKind } from '../ui/sound'
import './Keyboard.css'

type Field = HTMLInputElement | HTMLTextAreaElement
type Dir = 'up' | 'down' | 'left' | 'right'
type Special = 'shift' | 'del' | 'page' | 'space' | 'done'
interface Key { id: string; text?: string; special?: Special; wide?: number }

// Teclado en pantalla para el mando: se abre al pulsar A sobre un campo de texto.
// Cruceta mueve · A escribe · X borra · Y espacio · LB/RB mueven el cursor · START = Listo · B cierra.
// Mientras está abierto, la navegación normal queda en pausa (lockNav('keyboard')).

const chars = (row: number | string, s: string): Key[] => [...s].map((c, i) => ({ id: `k${row}-${i}`, text: c }))

// Distribución por idioma: filas de letras, una fila extra de acentos (si hace falta) y los signos de la 2.ª página
interface Layout { top: string; middle: string; bottom: string; accents?: string; symbols: string }
const LAYOUTS: Record<string, Layout> = {
  es: { top: 'qwertyuiop', middle: 'asdfghjklñ', bottom: 'zxcvbnm', symbols: 'áéíóúü¿¡()' },
  en: { top: 'qwertyuiop', middle: 'asdfghjkl', bottom: 'zxcvbnm', symbols: '()[]€$£*/\\' },
  'pt-BR': { top: 'qwertyuiop', middle: 'asdfghjklç', bottom: 'zxcvbnm', accents: 'ãõáéíóúâêô', symbols: '()[]€$*/\\à' },
  fr: { top: 'qwertyuiop', middle: 'asdfghjkl', bottom: 'zxcvbnm', accents: 'éèêàâçùûôï', symbols: '()[]€$«»/\\' },
  de: { top: 'qwertzuiop', middle: 'asdfghjklö', bottom: 'yxcvbnm', accents: 'äüß', symbols: '()[]€$„“/\\' },
  it: { top: 'qwertyuiop', middle: 'asdfghjkl', bottom: 'zxcvbnm', accents: 'àèéìòù', symbols: '()[]€$*/\\' },
}

function keyboardRows(lang: string) {
  const l = LAYOUTS[lang] ?? LAYOUTS.en
  const shiftRow = (s: string): Key[] => [{ id: 'k3-s', special: 'shift', wide: 1.5 }, ...chars(3, s), { id: 'k3-d', special: 'del', wide: 1.5 }]
  const last: Key[] = [{ id: 'k4-p', special: 'page', wide: 1.5 }, { id: 'k4-a', text: '@' }, { id: 'k4-sp', special: 'space', wide: 5 }, { id: 'k4-dot', text: '.' }, { id: 'k4-ok', special: 'done', wide: 1.5 }]
  const letters: Key[][] = [chars(0, '1234567890'), chars(1, l.top), chars(2, l.middle), shiftRow(l.bottom)]
  if (l.accents) letters.push(chars('x', l.accents))
  letters.push(last)
  const symbols: Key[][] = [chars(0, '1234567890'), chars(1, '@./:-_?!,;'), chars(2, l.symbols), shiftRow('#&%+="\''), last]
  return { letters, symbols }
}

function caretOf(el: Field) {
  try { return el.selectionStart ?? el.value.length } catch { return el.value.length }
}

/** Cambia el texto de un campo de forma que React se entere */
function writeValue(el: Field, value: string, caret: number) {
  const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype
  Object.getOwnPropertyDescriptor(proto, 'value')?.set?.call(el, value)
  el.dispatchEvent(new Event('input', { bubbles: true }))
  try { el.setSelectionRange(caret, caret) } catch { /* email/number no tienen cursor */ }
}

/** Tecla más cercana en una dirección (como la navegación de la consola, pero solo dentro del teclado) */
function nearestKey(root: HTMLElement, fromId: string, dir: Dir): string | null {
  const keys = Array.from(root.querySelectorAll<HTMLElement>('[data-key]'))
  const from = keys.find((k) => k.dataset.key === fromId)
  if (!from) return keys[0]?.dataset.key ?? null
  const a = from.getBoundingClientRect()
  const ax = a.left + a.width / 2, ay = a.top + a.height / 2
  let best: HTMLElement | null = null
  let bestScore = Infinity
  for (const el of keys) {
    if (el === from) continue
    const b = el.getBoundingClientRect()
    const dx = b.left + b.width / 2 - ax, dy = b.top + b.height / 2 - ay
    const ok = (dir === 'right' && dx > 4) || (dir === 'left' && dx < -4) || (dir === 'down' && dy > 4) || (dir === 'up' && dy < -4)
    if (!ok) continue
    const horizontal = dir === 'left' || dir === 'right'
    // En horizontal solo cuenta la misma fila
    if (horizontal && Math.abs(dy) > 8) continue
    const score = horizontal ? Math.abs(dx) : Math.abs(dy) + Math.abs(dx) * 2
    if (score < bestScore) { bestScore = score; best = el }
  }
  // Al llegar al borde de una fila, da la vuelta
  if (!best && (dir === 'left' || dir === 'right')) {
    const row = keys.filter((k) => Math.abs(k.getBoundingClientRect().top - a.top) < 8)
    best = dir === 'right' ? row[0] : row[row.length - 1]
  }
  return best?.dataset.key ?? null
}

export function Keyboard() {
  const t = useT()
  const lang = getLanguage()
  const layout = useMemo(() => keyboardRows(lang), [lang])
  const [field, setField] = useState<Field | null>(null)
  const [symbols, setSymbols] = useState(false)
  const [shift, setShift] = useState(false)
  const [focus, setFocus] = useState('k1-0')
  const [, redraw] = useReducer((n: number) => n + 1, 0)
  const caret = useRef(0)
  const root = useRef<HTMLDivElement>(null)
  const location = useLocation()

  const open = useCallback((el: Field) => {
    caret.current = caretOf(el)
    setField(el)
    setSymbols(false)
    setShift(false)
    setFocus('k1-0')
    el.scrollIntoView({ block: 'center', behavior: 'smooth' })
  }, [])
  const close = useCallback(() => setField(null), [])

  // Pausa la navegación de la consola mientras está abierto
  useEffect(() => {
    if (!field) return
    lockNav('keyboard')
    return () => unlockNav('keyboard')
  }, [field])

  // Al cambiar de pantalla, se cierra
  useEffect(close, [location.pathname, close])

  // Cuándo abrirlo
  useEffect(() => {
    const onRequest = (e: Event) => { const el = (e as CustomEvent).detail; if (isTextField(el)) open(el) }
    // Foco puesto por la propia pantalla (no por la navegación) mientras se usa el mando
    const onFocusIn = (e: FocusEvent) => {
      if (!navState.focusing && isTextField(e.target as Element) && inputMode() === 'pad') open(e.target as Field)
    }
    // Ratón o pantalla táctil sobre un campo
    const onPointerUp = (e: PointerEvent) => {
      const el = e.target as Element
      if (isTextField(el)) open(el)
    }
    window.addEventListener('kbopen', onRequest)
    window.addEventListener('focusin', onFocusIn)
    window.addEventListener('pointerup', onPointerUp)
    return () => {
      window.removeEventListener('kbopen', onRequest)
      window.removeEventListener('focusin', onFocusIn)
      window.removeEventListener('pointerup', onPointerUp)
    }
  }, [open])

  // ---- Acciones sobre el texto ----
  const edit = useCallback((fn: (value: string, c: number) => [string, number]) => {
    if (!field) return
    const c = Math.max(0, Math.min(caret.current, field.value.length))
    let [value, next] = fn(field.value, c)
    if (field.maxLength > 0 && value.length > field.maxLength) return
    next = Math.max(0, Math.min(next, value.length))
    if (value !== field.value) writeValue(field, value, next)
    else try { field.setSelectionRange(next, next) } catch { /* sin cursor */ }
    caret.current = next
    redraw()
  }, [field])

  const type = useCallback((s: string) => {
    const text = shift ? s.toLocaleUpperCase(lang) : s
    edit((v, c) => [v.slice(0, c) + text + v.slice(c), c + text.length])
    if (shift) setShift(false) // mayúscula solo para una letra
  }, [edit, shift, lang])
  const erase = useCallback(() => edit((v, c) => (c > 0 ? [v.slice(0, c - 1) + v.slice(c), c - 1] : [v, c])), [edit])
  const moveCaret = useCallback((d: number) => edit((v, c) => [v, c + d]), [edit])
  const done = useCallback(() => {
    const form = field?.form
    close()
    sound.select()
    if (form) form.requestSubmit()
  }, [field, close])

  const press = useCallback((k: Key) => {
    if (k.text) return type(k.text)
    if (k.special === 'shift') return setShift((s) => !s)
    if (k.special === 'del') return erase()
    if (k.special === 'space') return type(' ')
    if (k.special === 'page') return setSymbols((s) => !s)
    if (k.special === 'done') return done()
  }, [type, erase, done])

  const rows = symbols ? layout.symbols : layout.letters

  // Mando: llega ya traducido por useSpatialNav (perfil aplicado)
  useEffect(() => {
    if (!field) return
    const onPad = (e: Event) => {
      const k = (e as CustomEvent<PadKey>).detail
      if (k === 'up' || k === 'down' || k === 'left' || k === 'right') {
        const next = root.current && nearestKey(root.current, focus, k)
        if (next) { sound.move(); setFocus(next) }
      } else if (k === 'accept') {
        const key = rows.flat().find((x) => x.id === focus)
        if (key) { sound.select(); press(key) }
      } else if (k === 'back') { sound.back(); close() }
      else if (k === 'x') erase()
      else if (k === 'y') type(' ')
      else if (k === 'pagePrev') moveCaret(-1)
      else if (k === 'pageNext') moveCaret(1)
      else if (k === 'menu') done()
    }
    window.addEventListener('padkey', onPad)
    return () => window.removeEventListener('padkey', onPad)
  }, [field, focus, rows, press, close, erase, type, moveCaret, done])

  // Teclado físico: Esc cierra; cualquier otra tecla cierra y escribe en el campo como siempre
  useEffect(() => {
    if (!field) return
    const onKey = (e: KeyboardEvent) => {
      if (!e.isTrusted) return
      if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation() }
      close()
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [field, close])

  // Si el campo desaparece (la pantalla se repinta), cerrar
  useEffect(() => {
    if (!field) return
    const id = setInterval(() => { if (!field.isConnected) close() }, 500)
    return () => clearInterval(id)
  }, [field, close])

  if (!field) return null

  const value = field.value
  const c = Math.max(0, Math.min(caret.current, value.length))
  const shown = field.type === 'password' ? '•'.repeat(value.length) : value
  const ps = padKind() === 'ps'
  const label = (k: Key): ReactNode => {
    if (k.text) return shift ? k.text.toLocaleUpperCase(lang) : k.text
    switch (k.special) {
      case 'shift': return <ArrowFatUp size={22} weight={shift ? 'fill' : 'light'} aria-label={t('keyboard.shift')} />
      case 'del': return <Backspace size={22} weight="light" aria-label={t('keyboard.delete')} />
      case 'space': return t('keyboard.space')
      case 'page': return symbols ? 'ABC' : '?123'
      case 'done': return <><Check size={20} weight="light" /> {t('keyboard.done')}</>
    }
  }

  return (
    <div className="osk" role="dialog" aria-label={t('keyboard.label')} onPointerDown={(e) => e.preventDefault() /* el campo no pierde el foco */}>
      <div className="osk-panel" ref={root}>
        <div className="osk-preview">
          <span className="osk-hint-label">{field.placeholder || t('keyboard.placeholder')}</span>
          <span className="osk-text">
            {shown.slice(0, c)}<span className="osk-caret" aria-hidden />{shown.slice(c)}
          </span>
        </div>
        <div className="osk-keys">
          {rows.map((row, r) => (
            <div className="osk-row" key={r}>
              {row.map((k) => (
                <button
                  key={k.id}
                  type="button"
                  data-key={k.id}
                  className={`osk-key${k.special ? ` osk-${k.special}` : ''}${k.special === 'shift' && shift ? ' on' : ''}`}
                  data-kfocused={focus === k.id ? '' : undefined}
                  style={k.wide ? { flexGrow: k.wide } : undefined}
                  onClick={() => { setFocus(k.id); press(k) }}
                >
                  {label(k)}
                </button>
              ))}
            </div>
          ))}
        </div>
        <footer className="osk-hints">
          <span><kbd>{ps ? '✕' : 'A'}</kbd> {t('keyboard.hint.type')}</span>
          <span><kbd>{ps ? '□' : 'X'}</kbd> {t('keyboard.delete')}</span>
          <span><kbd>{ps ? '△' : 'Y'}</kbd> {t('keyboard.space')}</span>
          <span><kbd className="osk-wide">{ps ? 'L1' : 'LB'}</kbd><ArrowLeft size={14} weight="light" /><ArrowRight size={14} weight="light" /><kbd className="osk-wide">{ps ? 'R1' : 'RB'}</kbd> {t('keyboard.hint.caret')}</span>
          <span><kbd className="osk-wide">{ps ? 'OPTIONS' : 'START'}</kbd> {t('keyboard.done')}</span>
          <span><kbd>{ps ? '○' : 'B'}</kbd> {t('keyboard.hint.close')}</span>
        </footer>
      </div>
    </div>
  )
}
