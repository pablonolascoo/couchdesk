// Revisa las traducciones: compara cada idioma con el español (la base).
// Uso: npm run i18n:check            → todos los idiomas y zonas
//      node scripts/i18n-check.mjs --ns home,games   → solo esas zonas (archivos)
// Avisa de: claves que faltan, claves de más, {variables} distintas, plurales "uno|varios" distintos,
// claves repetidas en dos archivos y JSON roto. Sale con código 1 si hay algo.
import { readdirSync, readFileSync, existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..', 'src', 'i18n', 'locales')
const nsArg = process.argv.indexOf('--ns')
const onlyNs = nsArg > -1 ? new Set(process.argv[nsArg + 1].split(',')) : null
const BASE = 'es'

let problems = 0
const report = (lang, msg) => { problems++; console.log(`  [${lang}] ${msg}`) }

/** { clave: { text, file } } de un idioma */
function loadLang(lang) {
  const dir = join(root, lang)
  const out = {}
  for (const file of readdirSync(dir).filter((f) => f.endsWith('.json')).sort()) {
    const ns = file.replace(/\.json$/, '')
    if (onlyNs && !onlyNs.has(ns)) continue
    let data
    try { data = JSON.parse(readFileSync(join(dir, file), 'utf8').replace(/^﻿/, '')) } catch (e) {
      report(lang, `${file}: JSON roto (${e.message})`)
      continue
    }
    for (const [key, text] of Object.entries(data)) {
      if (typeof text !== 'string') { report(lang, `${file}: "${key}" no es texto`); continue }
      if (out[key]) report(lang, `"${key}" está en ${out[key].file} y en ${file}`)
      out[key] = { text, file }
    }
  }
  return out
}

const vars = (s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(',')
const plural = (s) => s.includes('|')

if (!existsSync(join(root, BASE))) { console.error('No encuentro la carpeta base es/'); process.exit(1) }
const base = loadLang(BASE)
const langs = readdirSync(root, { withFileTypes: true }).filter((d) => d.isDirectory() && d.name !== BASE).map((d) => d.name)

console.log(`Base (${BASE}): ${Object.keys(base).length} claves${onlyNs ? ` en ${[...onlyNs].join(', ')}` : ''}`)
for (const lang of langs) {
  const before = problems
  const dict = loadLang(lang)
  for (const [key, { text, file }] of Object.entries(base)) {
    const tr = dict[key]
    if (!tr) { report(lang, `falta "${key}" (${file})`); continue }
    if (tr.file !== file) report(lang, `"${key}" debería estar en ${file}, no en ${tr.file}`)
    if (vars(text) !== vars(tr.text)) report(lang, `"${key}": variables {${vars(text)}} ≠ {${vars(tr.text)}}`)
    if (plural(text) !== plural(tr.text)) report(lang, `"${key}": el plural "uno|varios" no coincide`)
    if (!tr.text.trim() && text.trim()) report(lang, `"${key}" está vacía`)
  }
  for (const key of Object.keys(dict)) if (!base[key]) report(lang, `sobra "${key}" (${dict[key].file})`)
  console.log(`${lang}: ${Object.keys(dict).length} claves · ${problems - before === 0 ? 'bien' : `${problems - before} avisos`}`)
}

if (problems) { console.log(`\n${problems} avisos.`); process.exit(1) }
console.log('\nTodo en orden.')
