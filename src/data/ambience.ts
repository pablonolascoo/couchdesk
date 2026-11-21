// Vídeos del modo ambiente y del salvapantallas.
// Solo fuentes reales y comprobadas (HEAD 200 video/mp4 y oEmbed 200 el 21-11-2026).
// Se ven a cámara lenta tipo "salvapantallas aéreo": por eso los de Pexels son de 50/60 fps
// y se reproducen a 0,5 (siguen viéndose fluidos, a 25–30 imágenes por segundo).
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
  /**
   * Velocidad de reproducción (1 = normal). Cámara lenta suave: los de 50/60 fps van a 0,5
   * (quedan 25–30 imágenes por segundo, sin saltos); uno de 24–30 fps nunca baja de 0,8.
   */
  rate: number
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

/**
 * Vídeo de Pexels. `file` es el nombre del archivo en su carpeta, sin ".mp4"
 * (p. ej. "12503404_1920_1080_60fps" o "12465390-hd_1920_1080_60fps").
 * `poster` es el nombre de su imagen en images.pexels.com/videos/<id>/ (o nada).
 */
const pexels = (id: number, file: string, credit: string, rate: number, poster?: string): Clip => ({
  source: 'pexels',
  url: `https://videos.pexels.com/video-files/${id}/${file}.mp4`,
  poster: poster ? `https://images.pexels.com/videos/${id}/${poster}?auto=compress&cs=tinysrgb&w=640` : undefined,
  credit,
  page: `https://www.pexels.com/video/${id}/`,
  rate,
})

const youtube = (id: string, credit: string, rate = 1): Clip => ({
  source: 'youtube',
  url: `https://www.youtube.com/watch?v=${id}`,
  poster: `https://i.ytimg.com/vi/${id}/hqdefault.jpg`,
  credit,
  page: `https://www.youtube.com/watch?v=${id}`,
  rate,
})

// ---------------- Modo ambiente ----------------

/** Nombre y categoría se traducen al leerlos (ambience.item.<id>, ambience.cat.<categoría>) */
const amb = (id: string, cat: string, clip: Clip, fallback?: Clip): Ambience => ({
  id, ...clip, ...(fallback ? { fallback } : {}),
  get title() { return t(`ambience.item.${id}`) },
  get category() { return t(`ambience.cat.${cat}`) },
})

// Todos los de Pexels están grabados a 50/60 fps (o más) y van a cámara lenta (0,5).
// YouTube solo donde Pexels no tiene nada parecido (chimenea de horas, cafetería): a velocidad normal.
export const ambience: Ambience[] = [
  amb('chimenea', 'fuego', youtube('cdKop6aixVE', 'Fireplace 10 Hours'),
    pexels(13081949, '13081949-hd_1920_1080_60fps', 'Irina Obolonko', 0.5, 'blaze-bonfire-burn-burning-13081949.jpeg')),
  amb('lluvia-ventana', 'lluvia',
    pexels(28110696, '12301301_1920_1080_120fps', 'Mehedi Hasan', 0.5, 'peaceful-peaceful-moment-rainy-night-28110696.jpeg'),
    pexels(13619802, '13619802-hd_1920_1080_60fps', 'Joey D.', 0.5, 'after-rain-after-the-rain-glass-heavy-rain-13619802.jpeg')),
  amb('cafe-lluvia', 'lluvia', youtube('l2mW0DxCBY4', 'Coffee Shop Vibes'),
    pexels(36142889, '15330476_1920_1080_50fps', 'Salim Da', 0.5, 'pexels-photo-36142889.jpeg')),
  amb('acuario', 'agua',
    pexels(26593809, '11968058_1920_1080_60fps', 'Jianqing Xu', 0.5, 'a-lot-aquariums-blue-color-26593809.jpeg'),
    pexels(11042667, '11042667-hd_1920_1080_60fps', 'Diana', 0.5, 'pexels-photo-11042667.jpeg')),
  amb('cascada', 'agua', pexels(33609301, '14285682_1920_1080_60fps', 'Philip Dominic Carpentero', 0.5, 'aerial-drone-falls-forest-waterfall-33609301.jpeg')),
  amb('olas', 'mar', pexels(12465390, '12465390-hd_1920_1080_60fps', 'Vi Ulloa', 0.5, 'ocean-sea-waves-12465390.jpeg')),
  amb('bosque', 'naturaleza', pexels(27546116, '12164436_1920_1080_60fps', 'Ünal Karabiber', 0.5, 'pexels-photo-27546116.jpeg')),
  amb('nieve', 'invierno', pexels(30123241, '12919602_1920_1080_60fps', 'Ahto Palmre', 0.5, 'winter-forest-3-30123241.jpeg')),
  amb('auroras', 'cielo', pexels(28889010, '12505178_1920_1080_60fps', 'Frank Cone', 0.5, 'aurora-time-lapse-28889010.jpeg')),
  amb('ciudad-noche', 'ciudad', pexels(11677590, '11677590-hd_1920_1080_60fps', 'Daniel Narinian', 0.5, 'pexels-photo-11677590.jpeg')),
  amb('nubes', 'cielo', pexels(41157432, '17625507_1920_1080_60fps', 'Yunus Terk', 0.5, 'pexels-photo-41157432.jpeg')),
  amb('lagos', 'montana', pexels(38361089, '16290220_1920_1080_60fps', 'Yunus Terk', 0.5, 'pexels-photo-38361089.jpeg')),
]

// ---------------- Salvapantallas ----------------

export type CollectionId = 'costas' | 'montanas' | 'bosques' | 'ciudades' | 'nubes' | 'agua'

const collection = (id: CollectionId) => ({ id, get label() { return t(`screensaver.collection.${id}`) } })

export const collections: { id: CollectionId; readonly label: string }[] =
  (['costas', 'montanas', 'bosques', 'ciudades', 'nubes', 'agua'] as const).map(collection)

/** Paisaje aéreo de Pexels grabado a 50/60 fps: siempre a cámara lenta (0,5), suave */
const land = (collection: CollectionId, id: number, file: string, credit: string): Landscape =>
  ({ collection, ...pexels(id, file, credit, 0.5) })

export const landscapes: Landscape[] = [
  land('costas', 28882533, '12503404_1920_1080_60fps', 'Andre Dantas'),
  land('costas', 33301276, '14184775_1920_1080_60fps', 'FL Studio Image-Line'),
  land('costas', 28652279, '12443214_1920_1080_60fps', 'Luca Musella'),
  land('costas', 33734610, '14323332_1920_1080_60fps', 'Malcoln Oliveira'),
  land('montanas', 37218218, '15767088_1920_1080_60fps', 'Жанна Алимкулова'),
  land('montanas', 31115054, '13294747_1920_1080_60fps', 'Florian Delée'),
  land('montanas', 30859401, '13196587_1920_1080_60fps', 'Florian Delée'),
  land('montanas', 29136312, '12586418_1920_1080_60fps', 'Sergey Guk'),
  land('bosques', 40168172, '17158383_1920_1080_60fps', 'Ali Akdemir'),
  land('bosques', 29792712, '12800451_1920_1080_60fps', 'Julien Goettelmann'),
  land('bosques', 28712877, '12458593_1920_1080_60fps', 'Sascha Weber'),
  land('bosques', 35548240, '15060531_1920_1080_50fps', 'Sascha Weber'),
  land('ciudades', 11016337, '11016337-hd_1920_1080_60fps', 'Altaf Shah'),
  land('ciudades', 40524262, '17316552_1920_1080_60fps', 'Altaf Shah'),
  land('ciudades', 33290791, '14179907_1920_1080_60fps', 'The Six'),
  land('ciudades', 12097524, '12097524-hd_1920_1080_60fps', 'Braeson Holland'),
  land('nubes', 32194311, '13729922_1920_1080_60fps', 'Riedelmax'),
  land('nubes', 41157432, '17625507_1920_1080_60fps', 'Yunus Terk'),
  land('nubes', 32313070, '13781234_1920_1080_60fps', 'Julien Goettelmann'),
  land('nubes', 11053545, '11053545-hd_1920_1080_60fps', 'Sergei Skrynnik'),
  land('agua', 33609301, '14285682_1920_1080_60fps', 'Philip Dominic Carpentero'),
  land('agua', 37692559, '15981863_1920_1080_60fps', 'Nirjhar Basak'),
  land('agua', 38361089, '16290220_1920_1080_60fps', 'Yunus Terk'),
  land('agua', 35893963, '15224090_1920_1080_60fps', 'Julien Goettelmann'),
]

/** Créditos en una línea: "Vídeos de Pexels: A, B… · YouTube: canal, canal…" */
export function ambienceCredits(list: Ambience[] = ambience): string {
  const uniq = (xs: string[]) => [...new Set(xs)]
  const px = uniq(list.flatMap((a) => [a, a.fallback].filter((c): c is Clip => !!c && c.source === 'pexels').map((c) => c.credit)))
  const yt = uniq(list.filter((a) => a.source === 'youtube').map((a) => a.credit))
  return t('ambience.credits', { pexels: px.join(', '), youtube: yt.join(', ') })
}
