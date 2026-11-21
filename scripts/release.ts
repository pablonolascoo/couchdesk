#!/usr/bin/env bun
// Publicar una versión nueva en un solo paso (guía _docs-bim: deploy versionado con codename):
//   bun run release            → parche   1.0.0 → 1.0.1
//   bun run release minor      → menor    1.0.1 → 1.1.0
//   bun run release major      → mayor    1.1.0 → 2.0.0
// Sube la versión en todos los archivos, compila el instalador, crea la etiqueta vX.Y.Z,
// lo sube a GitHub y publica la «Release» con el instalador para descargar.
// Si algo falla antes de publicar, lo deja todo como estaba.
import { $ } from 'bun'
import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

type Level = 'patch' | 'minor' | 'major'
const level = (process.argv[2] ?? 'patch') as Level
if (!['patch', 'minor', 'major'].includes(level)) { console.error(`Nivel no válido: ${level}`); process.exit(1) }

function bump(v: string, l: Level) {
  let [a, b, c] = v.split('.').map(Number)
  if (l === 'major') { a++; b = 0; c = 0 } else if (l === 'minor') { b++; c = 0 } else { c++ }
  return `${a}.${b}.${c}`
}

const branch = (await $`git branch --show-current`.text()).trim()
if (branch !== 'main') { console.error(`Hay que publicar desde main (estás en ${branch})`); process.exit(1) }

const meta = JSON.parse(readFileSync('version.json', 'utf8')) as { codename: string; version: string; released: boolean }
const version = meta.released ? bump(meta.version, level) : meta.version
const tag = `v${version}`
const label = `${meta.codename} ${tag}`

// La versión va en cuatro sitios: version.json, package.json, Cargo.toml y tauri.conf.json
const files = ['version.json', 'package.json', 'src-tauri/Cargo.toml', 'src-tauri/tauri.conf.json']
const backup = Object.fromEntries(files.map((f) => [f, readFileSync(f, 'utf8')]))
const restore = () => files.forEach((f) => writeFileSync(f, backup[f]))

writeFileSync('version.json', JSON.stringify({ ...meta, version, released: true }, null, 2) + '\n')
writeFileSync('package.json', backup['package.json'].replace(/"version": "[^"]+"/, `"version": "${version}"`))
writeFileSync('src-tauri/Cargo.toml', backup['src-tauri/Cargo.toml'].replace(/^version = "[^"]+"/m, `version = "${version}"`))
writeFileSync('src-tauri/tauri.conf.json', backup['src-tauri/tauri.conf.json'].replace(/"version": "[^"]+"/, `"version": "${version}"`))

console.log(`\n▶  Preparando ${label}\n`)
const build = await $`npm run app:build`.nothrow()
const installer = `src-tauri/target/release/bundle/nsis/Consola_${version}_x64-setup.exe`
if (build.exitCode !== 0 || !existsSync(installer)) {
  console.error('\n✖  La compilación falló. Todo vuelve a como estaba.')
  restore()
  process.exit(1)
}

// Notas de la versión: su apartado del CHANGELOG
const changelog = existsSync('CHANGELOG.md') ? readFileSync('CHANGELOG.md', 'utf8') : ''
const section = changelog.split(/^## /m).find((s) => s.startsWith(`[${version}]`) || s.startsWith(version))
const notes = (section ? section.split('\n').slice(1).join('\n').trim() : `Versión ${label}.`) +
  `\n\n**Descarga:** \`Consola_${version}_x64-setup.exe\` (Windows 10/11, 64 bits).` +
  `\n\nWindows puede avisar de que el programa no es conocido (aún no está firmado): «Más información» → «Ejecutar de todas formas».`
// Fuera del proyecto: así no se cuela en el commit de la versión
const notesFile = join(tmpdir(), 'consola-release-notes.md')
writeFileSync(notesFile, notes)

await $`git add -A`
await $`git commit -m ${`release: ${label}`}`.nothrow()
await $`git tag -a ${tag} -m ${label}`
const pushed = await $`git push origin main --follow-tags`.nothrow()
if (pushed.exitCode !== 0) { console.error('\n✖  No se pudo subir a GitHub. La etiqueta queda en local.'); process.exit(1) }
await $`gh release create ${tag} ${installer} --title ${`CouchDesk ${version} · ${meta.codename}`} --notes-file ${notesFile} --latest`
// Avisar a los buscadores de que hay versión nueva (si la web ya está publicada)
await $`bun scripts/indexnow.ts`.nothrow()
console.log(`\n✔  ${label} publicada: https://github.com/pablonolascoo/couchdesk/releases/tag/${tag}\n`)
