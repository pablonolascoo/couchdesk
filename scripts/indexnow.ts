#!/usr/bin/env bun
// Avisa a los buscadores (Bing, Yandex, Seznam, Naver… vía IndexNow) de que la web ha cambiado.
// Google no usa IndexNow: se entera por sitemap.xml (Search Console).
// La clave es nuestra (hex de 32) y está publicada en docs/<clave>.txt, como pide IndexNow.
//   bun scripts/indexnow.ts            → manda todas las URLs del sitemap
import { readdirSync, readFileSync } from 'node:fs'

const SITE = 'https://pablonolascoo.github.io/couchdesk'
const keyFile = readdirSync('docs').find((f) => /^[a-f0-9]{32}\.txt$/.test(f))
if (!keyFile) { console.error('Falta docs/<clave>.txt'); process.exit(1) }
const key = keyFile.replace('.txt', '')

// Antes de avisar: la clave tiene que estar ya publicada (GitHub Pages tarda un minuto en actualizar)
const live = await fetch(`${SITE}/${keyFile}`).then((r) => (r.ok ? r.text() : '')).catch(() => '')
if (live.trim() !== key) { console.error(`La clave aún no está publicada en ${SITE}/${keyFile}. Prueba en un minuto.`); process.exit(1) }

const sitemap = readFileSync('docs/sitemap.xml', 'utf8')
const urlList = [...sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)].map((m) => m[1].trim())
const res = await fetch('https://api.indexnow.org/indexnow', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json; charset=utf-8' },
  body: JSON.stringify({ host: new URL(SITE).host, key, keyLocation: `${SITE}/${keyFile}`, urlList }),
})
// 200 = recibido · 202 = recibido, pendiente de validar la clave
console.log(`IndexNow: ${res.status} ${res.statusText} · ${urlList.length} URL(s): ${urlList.join(', ')}`)
if (res.status >= 400) process.exit(1)
