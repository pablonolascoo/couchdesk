// Versión visible de la consola: "Salón v1.0.0". En desarrollo, la siguiente con "-dev".
import meta from '../version.json'

function bumpPatch(v: string) {
  const [a, b, c] = v.split('.').map(Number)
  return `${a}.${b}.${c + 1}`
}

const isProd = import.meta.env.PROD
const number = isProd ? meta.version : meta.released ? bumpPatch(meta.version) : meta.version

export const APP_CODENAME = meta.codename
export const APP_VERSION_NUMBER = number
export const APP_VERSION = `${meta.codename} v${number}${isProd ? '' : '-dev'}`
