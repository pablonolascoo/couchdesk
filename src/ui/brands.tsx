// Logos oficiales de marca (simple-icons, licencia CC0) en SVG: se ven nítidos a cualquier tamaño.
// Xbox no tiene logo libre: se representa con su color y un mando, nunca con un logo imitado.
import {
  siSteam, siEpicgames, siGogdotcom, siPlaystation, siYoutube, siTwitch,
  siGoogle, siWikipedia, siNvidia, siClaude, siNetflix,
  siPlaystation2, siPlaystation3, siPlaystation4, siPlaystation5,
} from 'simple-icons'

type SimpleIcon = { path: string; hex: string; title: string }

export const brands = {
  steam: siSteam,
  epic: siEpicgames,
  gog: siGogdotcom,
  playstation: siPlaystation,
  youtube: siYoutube,
  twitch: siTwitch,
  google: siGoogle,
  wikipedia: siWikipedia,
  nvidia: siNvidia,
  claude: siClaude,
  netflix: siNetflix,
  ps2: siPlaystation2,
  ps3: siPlaystation3,
  ps4: siPlaystation4,
  ps5: siPlaystation5,
} satisfies Record<string, SimpleIcon>

export type Brand = keyof typeof brands

/** Logo de marca. `color` por defecto: el suyo; "currentColor" para teñirlo */
export function BrandLogo({ name, size = 24, color, className }: { name: Brand; size?: number; color?: string; className?: string }) {
  const icon = brands[name]
  return (
    <svg className={className} width={size} height={size} viewBox="0 0 24 24" role="img" aria-label={icon.title} fill={color ?? `#${icon.hex}`}>
      <path d={icon.path} />
    </svg>
  )
}
