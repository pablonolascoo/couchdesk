// Comprueba el contraste de todos los temas integrados.
// Uso: node scripts/check-themes.ts   (Node 23.6+ entiende TypeScript sin compilar)
import { PALETTES, BUILTIN_IDS, checkPalette } from '../src/data/themePalettes.ts'

let failed = 0
for (const id of BUILTIN_IDS) {
  const results = checkPalette(PALETTES[id], id === 'alto-contraste')
  const worst = results.reduce((a, b) => (b.ratio / b.min < a.ratio / a.min ? b : a))
  const bad = results.filter((r) => !r.ok)
  failed += bad.length
  console.log(`${bad.length ? 'FALLA' : 'OK   '} ${id.padEnd(15)} peor: ${worst.pair} ${worst.ratio}:1 (mín ${worst.min})`)
  for (const r of bad) console.log(`        ${r.pair}: ${r.ratio}:1 < ${r.min}`)
}
if (process.argv.includes('--all')) {
  for (const id of BUILTIN_IDS) console.table(checkPalette(PALETTES[id], id === 'alto-contraste'))
}
process.exit(failed ? 1 : 0)
