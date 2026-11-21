#!/usr/bin/env node
// Capturas de la app para la web y el README.
// Usa Chrome (o Edge) sin ventana contra el servidor de desarrollo, que ya debe estar abierto (npm run dev).
//
//   node scripts/screenshots.mjs              -> es y en
//   node scripts/screenshots.mjs es en fr     -> los idiomas que digas (pt-br, fr, de, it…)
//   URL=http://localhost:5173 node scripts/screenshots.mjs
//   DEBUG=1 node scripts/screenshots.mjs      -> dice cada ruta antes de abrirla
//
// Guarda en docs/img/<idioma>/: <nombre>.jpg y <nombre>.webp a 1600x900, y <nombre>-800.jpg/.webp a 800x450.
// Una ruta que no existe (la app vuelve a "/") o sale en blanco se salta.
// Después: npm run site (la web usa docs/img/<idioma>/ y, si falta, docs/img/en/).
import { spawn } from 'node:child_process'
import { existsSync, mkdirSync, writeFileSync, rmSync, mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const BASE = (process.env.URL || 'http://localhost:5173').replace(/\/$/, '')
const LANGS = process.argv.slice(2).length ? process.argv.slice(2) : ['es', 'en']
const W = 1600, H = 900

// nombre del archivo -> ruta de la app
const ROUTES = [
  ['inicio', '/'],
  ['biblioteca', '/biblioteca'],
  ['nube', '/nube'],
  ['navegador', '/navegador'],
  ['emuladores', '/emuladores'],
  ['musica', '/musica'],
  ['ambiente', '/ambiente'],
  ['ajustes', '/ajustes'],
  ['mandos', '/ajustes/mandos'],
  ['temas', '/ajustes/temas'],
  ['claude', '/claude'],
  ['salvapantallas', '/salvapantallas?prueba=1'],
]
// el código de idioma que guarda la app (couchdesk.lang)
const APP_LANG = { es: 'es', en: 'en', 'pt-br': 'pt-BR', 'pt-BR': 'pt-BR', fr: 'fr', de: 'de', it: 'it' }

function findBrowser() {
  const list = [
    process.env.CHROME_PATH,
    'C:/Program Files/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
    `${process.env.LOCALAPPDATA}/Google/Chrome/Application/chrome.exe`,
    'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
    'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
  ].filter(Boolean)
  return list.find((p) => existsSync(p))
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const withTimeout = (p, ms, what) => Promise.race([p, new Promise((_, j) => setTimeout(() => j(new Error(`${what}: más de ${ms / 1000} s`)), ms))])

async function cdpConnect(port) {
  let info
  for (let i = 0; i < 50 && !info; i++) {
    try { info = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json() } catch { await sleep(200) }
  }
  if (!info) throw new Error('El navegador no respondió')
  const ws = new WebSocket(info.webSocketDebuggerUrl)
  await new Promise((r, j) => { ws.onopen = r; ws.onerror = j })
  let id = 0
  const pending = new Map()
  const listeners = new Set()
  ws.onmessage = (ev) => {
    const msg = JSON.parse(ev.data)
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id)
      pending.delete(msg.id)
      msg.error ? reject(new Error(msg.error.message)) : resolve(msg.result)
    } else for (const l of listeners) l(msg)
  }
  const send = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
    const mid = ++id
    pending.set(mid, { resolve, reject })
    ws.send(JSON.stringify({ id: mid, method, params, sessionId }))
  })
  return { ws, send, listeners }
}

async function shootRoute(s, sessionId, listeners, lang, outDir, name, route) {
  const loaded = new Promise((r) => {
    const fn = (m) => { if (m.sessionId === sessionId && m.method === 'Page.loadEventFired') { listeners.delete(fn); r() } }
    listeners.add(fn)
    setTimeout(() => { listeners.delete(fn); r() }, 15000)
  })
  if (process.env.DEBUG) console.log('  ->', lang, route)
  await s('Page.navigate', { url: BASE + route })
  await loaded
  // esperar a que la pantalla pinte texto (la primera carga de Vite tarda)
  const want = route.split('?')[0]
  let v = { path: '', text: 0 }
  for (let t = 0; t < 40; t++) {
    try {
      const r = await s('Runtime.evaluate', {
        returnByValue: true,
        expression: `({ path: location.pathname, text: (document.body?.innerText || '').trim().length })`,
      })
      if (r?.result?.value) v = r.result.value
    } catch { /* la página aún está cambiando */ }
    if ((v.path && v.path !== want) || v.text >= 20) break
    await sleep(500)
  }
  if (v.path !== want) return `no existe (la app fue a ${v.path || '?'}), se salta`
  if (v.text < 20) return 'en blanco, se salta'
  await sleep(3000) // portadas, fondos y animaciones de entrada
  // el ratón, fuera de la captura
  await s('Input.dispatchMouseEvent', { type: 'mouseMoved', x: W - 1, y: H - 1 })
  await sleep(300)
  for (const [suffix, scale] of [['', 1], ['-800', 0.5]]) {
    const clip = { x: 0, y: 0, width: W, height: H, scale }
    const jpg = await s('Page.captureScreenshot', { format: 'jpeg', quality: 82, clip })
    const webp = await s('Page.captureScreenshot', { format: 'webp', quality: 80, clip })
    writeFileSync(join(outDir, `${name}${suffix}.jpg`), Buffer.from(jpg.data, 'base64'))
    writeFileSync(join(outDir, `${name}${suffix}.webp`), Buffer.from(webp.data, 'base64'))
  }
  return null
}

async function main() {
  try { await fetch(BASE) } catch { console.error(`No responde ${BASE}. Abre antes el servidor de desarrollo (npm run dev).`); process.exit(1) }
  const exe = findBrowser()
  if (!exe) { console.error('No encuentro Chrome ni Edge. Indica la ruta con CHROME_PATH.'); process.exit(1) }

  const profile = mkdtempSync(join(tmpdir(), 'couchdesk-shots-'))
  const port = 9300 + Math.floor(Math.random() * 500)
  const proc = spawn(exe, [
    '--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`,
    '--no-first-run', '--no-default-browser-check', '--hide-scrollbars', '--mute-audio',
    '--autoplay-policy=no-user-gesture-required', `--window-size=${W},${H}`, 'about:blank',
  ], { stdio: 'ignore' })

  const { ws, send, listeners } = await cdpConnect(port)
  const done = []
  try {
    for (const lang of LANGS) {
      const code = APP_LANG[lang] || lang
      const outDir = join(ROOT, 'docs', 'img', lang.toLowerCase())
      mkdirSync(outDir, { recursive: true })
      for (const [name, route] of ROUTES) {
        // una pestaña nueva por ruta: si una se cuelga (vídeo, WebGL), no arrastra a las demás
        const { targetId } = await send('Target.createTarget', { url: 'about:blank' })
        const { sessionId } = await send('Target.attachToTarget', { targetId, flatten: true })
        const s = (m, p) => send(m, p, sessionId)
        try {
          await s('Page.enable')
          await s('Runtime.enable')
          await s('Emulation.setDeviceMetricsOverride', { width: W, height: H, deviceScaleFactor: 1, mobile: false })
          // el idioma se fija antes de que cargue la app
          await s('Page.addScriptToEvaluateOnNewDocument', {
            source: `try{localStorage.setItem('couchdesk.lang',${JSON.stringify(code)})}catch(e){}`,
          })
          const skip = await withTimeout(shootRoute(s, sessionId, listeners, lang, outDir, name, route), 90000, route)
          if (skip) console.log(`  ${lang} ${route}: ${skip}`)
          else { done.push(`${lang}/${name}`); console.log(`  ${lang} ${route} -> docs/img/${lang.toLowerCase()}/${name}.jpg/.webp`) }
        } catch (e) {
          console.log(`  ${lang} ${route}: falló (${e.message}), se salta`)
        } finally {
          await send('Target.closeTarget', { targetId }).catch(() => {})
        }
      }
    }
  } finally {
    try { await withTimeout(send('Browser.close'), 5000, 'cerrar') } catch { /* ya cerrado */ }
    ws.close()
    await sleep(500)
    try { proc.kill() } catch { /* nada */ }
    try { rmSync(profile, { recursive: true, force: true }) } catch { /* Windows puede tardar en soltarla */ }
  }
  console.log(`Listo: ${done.length} capturas.`)
}

main().catch((e) => { console.error(e); process.exit(1) })
