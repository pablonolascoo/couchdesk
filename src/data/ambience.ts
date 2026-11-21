// Vídeos del modo ambiente y del salvapantallas.
// Solo fuentes reales y comprobadas (HEAD 200 video/mp4 y oEmbed 200 el 21-11-2026):
// · Pexels: licencia libre de Pexels (uso gratis, sin atribución obligatoria; la damos igual)
// · YouTube: vídeos largos de canales de ambiente; si el dueño bloquea la inserción, se usa el de Pexels

import { t } from '../i18n'

export type Source = 'pexels' | 'youtube'

export interface Clip {
  source: Source
  /** MP4 directo (Pexels, HD) o dirección de YouTube */
  url: string
  /** Imagen de portada (puede faltar) */
  poster?: string
  /** Autor o canal */
  credit: string
  /** Página original */
  page: string
}

export interface Ambience extends Clip {
  id: string
  title: string
  category: string
  /** Si YouTube no deja verlo aquí, este de Pexels */
  fallback?: Clip
}

export interface Landscape extends Clip {
  collection: CollectionId
}

const pexels = (id: number, file: string, credit: string, poster = true): Clip => ({
  source: 'pexels',
  url: `https://videos.pexels.com/video-files/${id}/${id}-${file}.mp4`,
  poster: poster ? `https://images.pexels.com/videos/${id}/pictures/preview-0.jpg?auto=compress&w=640` : undefined,
  credit,
  page: `https://www.pexels.com/video/${id}/`,
})

const youtube = (id: string, credit: string): Clip => ({
  source: 'youtube',
  url: `https://www.youtube.com/watch?v=${id}`,
  poster: `https://i.ytimg.com/vi/${id}/hqdefault.jpg`,
  credit,
  page: `https://www.youtube.com/watch?v=${id}`,
})

// ---------------- Modo ambiente ----------------

/** Nombre y categoría se traducen al leerlos (ambience.item.<id>, ambience.cat.<categoría>) */
const amb = (id: string, cat: string, clip: Clip, fallback?: Clip): Ambience => ({
  id, ...clip, ...(fallback ? { fallback } : {}),
  get title() { return t(`ambience.item.${id}`) },
  get category() { return t(`ambience.cat.${cat}`) },
})

export const ambience: Ambience[] = [
  amb('chimenea', 'fuego', youtube('cdKop6aixVE', 'Fireplace 10 Hours'), pexels(6507534, 'hd_1920_1080_25fps', 'Mikhail Nilov')),
  amb('lluvia-ventana', 'lluvia', youtube('YNuP7Fu7DXk', "That'll Play"), pexels(5197762, 'hd_1920_1080_25fps', 'Aleks BM')),
  amb('cafe-lluvia', 'lluvia', youtube('l2mW0DxCBY4', 'Coffee Shop Vibes'), pexels(5227362, 'hd_1920_1080_24fps', 'Francesco Ungaro')),
  amb('acuario', 'agua', youtube('zQ8caaUxIvY', 'Above Relaxation'), pexels(13192265, 'hd_1920_1080_25fps', 'Magda Ehlers', false)),
  amb('cascada', 'agua', pexels(5657930, 'hd_1920_1080_30fps', 'Wayne Speedy')),
  amb('olas', 'mar', pexels(1739010, 'hd_1920_1080_30fps', 'Nino Souza')),
  amb('bosque', 'naturaleza', pexels(3768671, 'hd_1920_1080_25fps', 'Taryn Elliott')),
  amb('nieve', 'invierno', pexels(6056609, 'hd_1920_1080_30fps', 'Francesco Ungaro')),
  amb('auroras', 'cielo', pexels(5700949, 'hd_1920_1080_25fps', 'Nico Becker')),
  amb('ciudad-noche', 'ciudad', pexels(5670194, 'hd_1920_1080_30fps', 'Michael Swigunski')),
  amb('nubes', 'cielo', pexels(6385335, 'hd_1920_1080_30fps', 'Poolside Creative')),
  amb('lagos', 'montana', pexels(3654337, 'hd_1920_1080_30fps', 'SwissHumanity Stories')),
]

// ---------------- Salvapantallas ----------------

export type CollectionId = 'costas' | 'montanas' | 'bosques' | 'ciudades' | 'nubes' | 'agua'

const collection = (id: CollectionId) => ({ id, get label() { return t(`screensaver.collection.${id}`) } })

export const collections: { id: CollectionId; readonly label: string }[] =
  (['costas', 'montanas', 'bosques', 'ciudades', 'nubes', 'agua'] as const).map(collection)

const land = (collection: CollectionId, id: number, file: string, credit: string): Landscape =>
  ({ collection, ...pexels(id, file, credit, false) })

export const landscapes: Landscape[] = [
  land('costas', 10070724, 'hd_1920_1080_24fps', 'Claudiu Ciobanu'),
  land('costas', 7913483, 'hd_1920_1080_30fps', 'Noel McShane'),
  land('costas', 6981329, 'hd_1920_1080_25fps', 'Mikhail Nilov'),
  land('costas', 1739010, 'hd_1920_1080_30fps', 'Nino Souza'),
  land('montanas', 3654337, 'hd_1920_1080_30fps', 'SwissHumanity Stories'),
  land('montanas', 3947129, 'hd_1920_1080_30fps', 'SwissHumanity Stories'),
  land('montanas', 6630032, 'hd_1920_1080_25fps', 'Adrien JACTA'),
  land('montanas', 2110771, 'hd_1920_1080_30fps', 'Wietse van den Hout'),
  land('bosques', 3768496, 'hd_1920_1080_25fps', 'Taryn Elliott'),
  land('bosques', 4046286, 'hd_1920_1080_25fps', 'Taryn Elliott'),
  land('bosques', 5607842, 'hd_1920_1080_24fps', 'Zlatin Georgiev'),
  land('bosques', 8191389, 'hd_2048_1080_30fps', 'Roman Odintsov'),
  land('ciudades', 5670194, 'hd_1920_1080_30fps', 'Michael Swigunski'),
  land('ciudades', 11016337, 'hd_1920_1080_60fps', 'Altaf Shah'),
  land('ciudades', 15215011, 'hd_1920_1080_30fps', 'Jiří Malý'),
  land('nubes', 6385335, 'hd_1920_1080_30fps', 'Poolside Creative'),
  land('nubes', 856171, 'hd_1920_1080_30fps', 'Pixabay'),
  land('nubes', 4092850, 'hd_1920_1080_30fps', 'Rajamohan S'),
  land('agua', 10171430, 'hd_1920_1080_25fps', 'Adrien JACTA'),
  land('agua', 5657930, 'hd_1920_1080_30fps', 'Wayne Speedy'),
  land('agua', 10989547, 'hd_1920_1080_30fps', 'Hussain Naushad'),
]

/** Créditos en una línea: "Vídeos de Pexels: A, B… · YouTube: canal, canal…" */
export function ambienceCredits(list: Ambience[] = ambience): string {
  const uniq = (xs: string[]) => [...new Set(xs)]
  const px = uniq(list.flatMap((a) => [a, a.fallback].filter((c): c is Clip => !!c && c.source === 'pexels').map((c) => c.credit)))
  const yt = uniq(list.filter((a) => a.source === 'youtube').map((a) => a.credit))
  return t('ambience.credits', { pexels: px.join(', '), youtube: yt.join(', ') })
}
