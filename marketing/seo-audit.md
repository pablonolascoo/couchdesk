# Auditoría SEO de CouchDesk

- **Sitio:** https://pablonolascoo.github.io/couchdesk/ (fuente: `docs/index.html`, `robots.txt`, `sitemap.xml`, `llms.txt`)
- **Fecha:** 21-11-2026
- **Método:** lectura del código local, descarga de la página publicada, búsquedas web (WebSearch) en es, en, pt, fr y de.
- **Limitación:** no hay ninguna herramienta SEO conectada (Ahrefs, Semrush, Search Console). Los volúmenes y la dificultad son **estimaciones cualitativas** (Alto / Medio / Bajo), basadas en quién ocupa hoy los resultados.

---

## 1. Resumen ejecutivo

| # | Hallazgo | Impacto |
|---|---|---|
| 1 | El sitio es **una sola URL en español**. Solo puede posicionar para unas pocas búsquedas. | Alto |
| 2 | **Microsoft lanzó el "modo Xbox"** para todos los PC con Windows 11 (abril 2026). Ahora domina "convertir PC en consola". Pero **no existe en Windows 10** y depende del mercado. Es el mayor hueco y el mayor rival. | Alto |
| 3 | Se publican **unas 90 páginas de demo** en `docs/assets/**` (p. ej. `assets/layout/hero-split.html`, título «Saltmarsh: coffee bar hire for events in Edinburgh»). No tienen `noindex`. Contenido ajeno y fino bajo tu dominio. | Alto |
| 4 | `robots.txt` está en `/couchdesk/robots.txt`. **Los buscadores solo leen `/robots.txt` en la raíz** del dominio (hoy da 404). Tu sitemap no se descubre solo. | Medio |
| 5 | La **meta description mide 211 caracteres** (máx. recomendado 155). Google la cortará. | Medio |
| 6 | **Marca incoherente:** la release se llama «Consola 1.0.0 · Salón» y el instalador `Consola_1.0.0_x64-setup.exe`. Debe decir CouchDesk en todo. | Medio |
| 7 | Faltan páginas que la competencia usa para posicionar: **comparativas, guías, descarga, novedades**. | Alto |
| 8 | Repo con 0 estrellas, descripción en español y sin presencia en AlternativeTo, winget, Reddit… **Sin enlaces entrantes** no hay autoridad. | Alto |
| 9 | Lo que está bien: H1 claro, alt de imágenes muy buenos, canonical, OG/Twitter completos, SoftwareApplication JSON-LD, `llms.txt`, peso total razonable (~0,9 MB). | — |

**Posicionamiento recomendado:** «el modo consola gratis y abierto para Windows 10 y 11: todos tus juegos, la nube, emuladores y el salón, con cualquier mando o el móvil». Es decir: lo que el modo Xbox, Steam Big Picture y Playnite hacen por separado, en una app.

---

## 2. Tabla de oportunidades de palabras clave

Dificultad = quién ocupa hoy la primera página. Oportunidad = encaje con CouchDesk × dificultad.

| # | Palabra clave | Idioma | Dificultad estimada | Oportunidad | Intención | Tipo de contenido recomendado |
|---|---|---|---|---|---|---|
| 1 | modo consola windows 10 | es | Baja | Muy alta | Informativa / transaccional | Landing «Modo consola para Windows 10» (el modo Xbox no existe en Win10) |
| 2 | alternativa modo xbox windows 11 | es | Baja-Media | Muy alta | Comercial | Comparativa «CouchDesk vs modo Xbox» |
| 3 | xbox mode alternative | en | Media | Muy alta | Comercial | Comparativa en inglés |
| 4 | convertir pc en consola | es | Media (medios: Xataka, Digital Trends, mundobytes) | Alta | Informativa | Guía «Cómo convertir tu PC en una consola» con CouchDesk como paso |
| 5 | turn pc into console / console mode windows | en | Media-Alta | Alta | Informativa | Guía + home en inglés |
| 6 | steam big picture alternative | en | Media (AlternativeTo, blogs) | Alta | Comercial | «CouchDesk vs Steam Big Picture» |
| 7 | big picture para todos los juegos | es | Baja | Alta | Comercial | Comparativa es + sección en home |
| 8 | playnite alternative / playnite fullscreen alternative | en | Media | Alta | Comercial | «CouchDesk vs Playnite» |
| 9 | alternativa a playnite | es | Baja | Media-Alta | Comercial | Comparativa es |
| 10 | pc como consola salón / pc salón mando | es | Baja | Alta | Informativa | Guía «Montar un PC de salón (HTPC) con Windows» |
| 11 | htpc frontend windows | en | Baja-Media | Media-Alta | Comercial | Guía HTPC + página de función «Arranque en lugar del escritorio» |
| 12 | windows shell replacement gaming / boot into big picture | en | Baja | Alta | Informativa | Guía «Arrancar Windows en modo consola (sin escritorio)» |
| 13 | arrancar windows sin escritorio / arrancar en modo consola | es | Baja | Alta | Informativa | Misma guía en es |
| 14 | windows handheld launcher (ROG Ally, Legion Go, MSI Claw) | en | Media (Winhanced, PocketDeck, Playnite) | Media-Alta | Comercial | Página «CouchDesk en portátiles de mano» |
| 15 | usar mando ps5 en pc / dualsense pc sin ds4windows | es | Alta (genérica) / Media (sin drivers) | Media | Informativa | Guía «Mando de PS5 en PC sin drivers» |
| 16 | ps5 remote play pc con mando / chiaki-ng windows | es/en | Baja-Media (pocas guías Windows de chiaki-ng) | Alta | Informativa | Guía «PS5 Remote Play en PC con chiaki-ng» |
| 17 | xbox cloud gaming pc pantalla completa mando | es/en | Baja | Media-Alta | Informativa | Guía «Xbox Cloud Gaming a pantalla completa en PC» |
| 18 | emulator frontend windows / emuladores windows con mando | en/es | Media (ES-DE, LaunchBox, RetroBat) | Media | Comercial | Página «Emuladores en Windows con mando» + carpetas BIOS/juegos |
| 19 | usar el móvil como mando pc / phone as controller pc | es/en | Media | Media-Alta | Informativa | Página de función «Móvil como mando por QR» |
| 20 | netflix con mando pc / navegador con mando | es/en | Baja | Media | Informativa | Página de función «Navegador con mando y streaming» |
| 21 | game launcher todas las tiendas / unified game library windows | es/en | Media | Media | Comercial | Sección biblioteca + comparativa Playnite |
| 22 | transformar pc em console / modo console windows | pt-BR | Baja | Alta (mercado grande, poca oferta local) | Informativa | Home + guía en pt-BR |
| 23 | transformer pc en console / mode console windows | fr | Baja | Media-Alta | Informativa | Home + guía en fr |
| 24 | pc als konsole nutzen / windows konsolenmodus | de | Baja | Media-Alta | Informativa | Home + guía en de |
| 25 | couchdesk (marca) | todos | Muy baja | Crítica | Navegacional | Home, GitHub, AlternativeTo, winget: nombre coherente en todo |

**Observación:** en es/pt/fr/de casi todos los resultados de «convertir PC en consola» son noticias del modo Xbox. Ninguna herramienta independiente tiene contenido local. Es un hueco claro.

---

## 3. Problemas on-page

| Página | Problema | Gravedad | Solución concreta |
|---|---|---|---|
| `/` | Meta description de 211 caracteres. | Alta | Usar la propuesta de 146 caracteres (ver abajo). |
| `/` | El `<title>` no incluye «modo consola» ni «gratis», que son las palabras que se buscan. | Media | Cambiar por la propuesta (53 caracteres). |
| `/` | No menciona Windows 10 como ventaja frente al modo Xbox. | Alta | Añadir a la entradilla: «También en Windows 10, donde el modo Xbox no llega». |
| `/` | No nombra términos que la gente busca: «Big Picture», «Playnite», «modo Xbox», «HTPC», «ROG Ally», «Legion Go». | Media | Bloque corto «¿En qué se diferencia?» con enlaces a las comparativas. |
| `/` | Funciones sin mencionar: Spotify, salvapantallas con reloj y tiempo, modo ambiente, fondos animados, temas, idiomas, agente Claude con voz, carpetas BIOS/juegos, Disney+, HBO Max, DAZN. | Media | Añadir una rejilla de funciones o enlazar a páginas de función. |
| `/` | Contenido corto (~600 palabras). | Media | Ampliar con FAQ y funciones (sin romper el diseño: mejor en subpáginas). |
| `/` | FAQ visible sin `FAQPage` en JSON-LD. | Baja | Añadirlo. Google ya casi no muestra FAQ enriquecidas, pero ayuda a buscadores de IA. |
| `/` | `SoftwareApplication` incompleto: falta `screenshot`, `featureList`, `softwareRequirements`, `fileSize`, `releaseNotes`, `datePublished`. | Media | Completar. No inventar `aggregateRating`. |
| `/` | Sin `WebSite` ni `Organization`/`Person` con `sameAs` (GitHub). | Baja | Añadir un bloque `@graph`. |
| `/` | No hay `BreadcrumbList` (normal con una sola página). | Baja | Añadir en subpáginas cuando existan. |
| `/` | Sin vídeo. El producto se entiende mucho mejor en movimiento. | Media | Vídeo de 60-90 s en YouTube, incrustado con `VideoObject`. |
| `/` | H2 «Siempre hay una salida» no dice nada al buscador. | Baja | Mantener el estilo, pero añadir subtítulo descriptivo: «Vuelve a la consola desde cualquier juego». |
| `/` | Google Fonts externo (bloquea el render). | Baja | Autoalojar Host Grotesk en woff2 con `font-display: swap`. |
| `/` | Imágenes solo en JPG. | Baja | Añadir AVIF/WebP con `<picture>`. Ahorro estimado del 30-50 %. |
| `/` | Solo `favicon.svg`. | Baja | Añadir `apple-touch-icon` 180 px y PNG 48 px. |
| `/assets/**/*.html` | ~90 demos públicas e indexables con temas ajenos (cafeterías, etc.) y 13 MB de fotos de demo. | Alta | Excluirlas del despliegue (carpeta fuera de `docs/` o paso de build). Si no se puede, `<meta name="robots" content="noindex">` en cada una. |
| `/robots.txt` | No es efectivo en una subruta. | Media | Enviar el sitemap en Search Console y Bing Webmaster. Opcional: crear el repo `pablonolascoo.github.io` con un `/robots.txt` que apunte al sitemap. |
| — | No hay `404.html`. | Baja | Crear una 404 con enlace a inicio y descarga. |
| GitHub release | Nombre «Consola 1.0.0» e instalador `Consola_…exe`. | Media | Renombrar a «CouchDesk 1.0.0 · Salón» y `CouchDesk_1.0.0_x64-setup.exe` (`productName` en `tauri.conf.json`). |
| GitHub repo | Descripción solo en español. GitHub y AlternativeTo se buscan en inglés. | Media | Descripción en inglés (ver plan). |
| Instalador | Sin firmar: SmartScreen asusta y frena descargas y menciones. | Media | Pedir firma gratuita en SignPath Foundation (Playnite la usa). |

### Títulos y meta descriptions propuestos (home)

| Idioma | `<title>` (≤60) | Long. | `meta description` (≤155) | Long. |
|---|---|---|---|---|
| es | CouchDesk: modo consola gratis para tu PC con Windows | 53 | Convierte tu PC con Windows 10 u 11 en una consola: Steam, Epic, GOG y Xbox en una biblioteca, con mando de Xbox o PS5 o el móvil. Gratis y libre. | 146 |
| en | CouchDesk: Free Console Mode for Your Windows PC | 48 | Turn your Windows 10/11 PC into a console: Steam, Epic, GOG and Xbox games in one library, played with an Xbox or PS5 controller or your phone. Free. | 149 |
| pt-BR | CouchDesk: modo console grátis para PC com Windows | 50 | Transforme seu PC com Windows 10 ou 11 em um console: Steam, Epic, GOG e Xbox numa biblioteca, com controle de Xbox, PS5 ou o celular. Grátis. | 142 |
| fr | CouchDesk : mode console gratuit pour PC Windows | 48 | Transformez votre PC Windows 10 ou 11 en console : Steam, Epic, GOG et Xbox réunis, à la manette Xbox ou PS5 ou au smartphone. Gratuit et libre. | 144 |
| de | CouchDesk: kostenloser Konsolenmodus für Windows-PCs | 52 | Mach deinen Windows-10/11-PC zur Konsole: Steam, Epic, GOG und Xbox in einer Bibliothek, mit Xbox- oder PS5-Controller oder Handy. Kostenlos. | 141 |
| it | CouchDesk: modalità console gratis per PC Windows | 49 | Trasforma il tuo PC Windows 10 o 11 in una console: Steam, Epic, GOG e Xbox in una libreria, con controller Xbox o PS5 o lo smartphone. Gratis. | 143 |

### Títulos para las comparativas (ejemplos)

| Página | Título | Long. |
|---|---|---|
| es · vs Playnite | CouchDesk vs Playnite: ¿qué modo consola elegir? | 48 |
| es · vs modo Xbox | Alternativa al modo Xbox de Windows 11 (y para Windows 10) | 58 |
| en · vs Playnite | CouchDesk vs Playnite: Which Console Mode Fits You? | 51 |
| en · vs Xbox mode | Xbox Mode Alternative for Windows 10 and 11 \| CouchDesk | 55 |
| en · vs Big Picture | Steam Big Picture Alternative for All Your Games \| CouchDesk | 60 |

---

## 4. Huecos de contenido

| Prioridad | Página | Por qué | Palabras clave |
|---|---|---|---|
| 1 | **CouchDesk vs modo Xbox** | Rival nuevo que domina las noticias. Ventajas claras: Windows 10, sin restricción de mercado, PS5 Remote Play, emuladores, móvil como mando, navegador con streaming. | alternativa modo xbox, xbox mode alternative, modo consola windows 10 |
| 2 | **CouchDesk vs Steam Big Picture** | Big Picture es el estándar mental. Diferencia: todas las tiendas, apps, nube y arranque sin escritorio. | steam big picture alternative, big picture para todos los juegos |
| 3 | **CouchDesk vs Playnite** | Playnite es el líder libre. Diferencia: CouchDesk es una consola entera (ajustes, Wi-Fi, navegador, nube), no solo una biblioteca. Ser honesto: Playnite tiene más plugins y metadatos. | playnite alternative, playnite fullscreen |
| 4 | **Descargar** (`/descargar/`) | Hoy el botón sale a GitHub. Una página propia posiciona «descargar CouchDesk», muestra versión, tamaño, requisitos, SHA-256 y cómo pasar SmartScreen. | couchdesk download, descargar couchdesk |
| 5 | **Novedades** (`/novedades/`) | Ya existe `CHANGELOG.md`. Publicarlo da contenido fresco y palabras clave por versión. | couchdesk changelog |
| 6 | Guía: **Arrancar Windows en modo consola (sin escritorio)** | Función única frente a casi todos. Poca competencia. | arrancar windows sin escritorio, boot into console mode windows, shell replacement |
| 7 | Guía: **PS5 Remote Play en PC con mando** | Pocas guías de chiaki-ng para Windows. | ps5 remote play pc, chiaki-ng windows |
| 8 | Guía: **Mando de PS5/PS4 en PC sin drivers** | Búsqueda enorme. Ángulo: sin DS4Windows, touchpad como ratón. | usar mando ps5 en pc |
| 9 | Guía: **Emuladores en Windows con mando** | Explica carpetas BIOS/juegos sin dar nada pirata. | emuladores windows con mando, emulator frontend windows |
| 10 | Guía: **Montar un PC de salón (HTPC)** | Captura «pc salón», «htpc windows». | htpc frontend windows, pc como consola salón |
| 11 | Página: **CouchDesk en portátiles de mano** (ROG Ally, Legion Go, MSI Claw) | Público muy activo en Reddit. Rivales: Winhanced, PocketDeck, Armoury Crate. Verificar antes que funciona bien en esos equipos. | rog ally launcher, legion go launcher |
| 12 | Páginas de función: móvil como mando, navegador y streaming, salvapantallas y modo ambiente, Spotify, agente Claude con voz | Cada una responde a una búsqueda concreta y da imágenes para Google Imágenes. | phone as controller pc, netflix con mando pc |
| 13 | **FAQ ampliada** | SmartScreen, antivirus, cómo volver al escritorio, si borra algo, idiomas, mandos compatibles. | — |
| 14 | **Docs / Ayuda** | Primeros pasos, atajos, solución de problemas. Reduce incidencias y suma long tail. | — |

Regla: cada página con su URL, un H1, enlace a Descargar y enlaces cruzados entre comparativas y guías.

---

## 5. Checklist técnico

| Elemento | Estado | Nota |
|---|---|---|
| HTTPS | Pass | GitHub Pages. |
| Página responde y se renderiza sin JS | Pass | El texto está en el HTML. |
| `<title>` presente y ≤60 | Pass | 53 caracteres, pero mejorable en palabras clave. |
| Meta description ≤155 | **Fail** | 211 caracteres. |
| Un solo H1 | Pass | «Modo consola para tu PC con Windows». |
| Jerarquía H2/H3 | Pass | 7 H2, H3 en tarjetas y FAQ. |
| `lang` en `<html>` | Pass | `es`. |
| Canonical | Pass | Absoluto y correcto. |
| hreflang | Warning | No aplica aún (un idioma). Obligatorio al traducir. |
| Open Graph / Twitter Card | Pass | Imagen 1440×900 de 93 KB con alt. |
| JSON-LD SoftwareApplication | Warning | Correcto pero incompleto (sin `screenshot`, `featureList`, `softwareRequirements`, `datePublished`). |
| JSON-LD FAQPage / WebSite / BreadcrumbList / VideoObject | Fail | No existen. |
| Alt en imágenes | Pass | Descriptivos y útiles. |
| `width`/`height` en imágenes (CLS) | Pass | Todas los tienen. |
| `loading="lazy"` y `fetchpriority` en hero | Pass | Bien aplicado. |
| Formatos modernos (AVIF/WebP) | Warning | Solo JPG. |
| Peso de la página | Pass | HTML 23 KB + CSS/JS 394 KB sin comprimir + imágenes ~350-500 KB en desktop. |
| Fuentes | Warning | Google Fonts externo. |
| `robots.txt` efectivo | **Fail** | En subruta: los bots lo ignoran. `/robots.txt` raíz = 404. |
| `sitemap.xml` | Warning | Válido, pero solo descubrible si se envía en Search Console/Bing. |
| Páginas basura indexables | **Fail** | ~90 demos en `/assets/**`, sin `noindex`. |
| 404 personalizada | Fail | No existe. |
| `llms.txt` | Pass | Completo. Añadir enlaces a futuras páginas. |
| Archivo de clave (`f7af…txt`) | Pass | Parece clave de IndexNow o verificación. Usarla para avisar a Bing/Yandex en cada cambio. |
| Search Console / Bing Webmaster | Warning | No verificable desde aquí. Dar de alta ambos. |
| Accesibilidad: skip link, `aria-label` en nav, botón menú | Pass | Correcto. |
| Accesibilidad: contraste del acento #a08672 sobre #111 | Warning | Revisar texto pequeño con ese color (≈5:1 en fondo oscuro, bien; sobre claro, bajo). |
| Marca coherente (release, instalador) | Fail | «Consola» en vez de «CouchDesk». |
| Dominio propio | Warning | `github.io/couchdesk` limita robots, marca y migraciones. |

---

## 6. Comparativa con competidores

| Competidor | Tipo | Precio | Puntos fuertes | Hueco que deja a CouchDesk | Cómo posiciona |
|---|---|---|---|---|---|
| **Modo Xbox (Xbox Full Screen Experience)** | Integrado en Windows 11 | Gratis | Oficial, marca Microsoft, arranque directo, Win+F11 | Solo Win11 24H2+, solo mercados soportados, despliegue gradual, sin emuladores, sin PS5 Remote Play, sin móvil como mando, fallos reportados | Noticias en todos los idiomas, soporte de Microsoft |
| **Steam Big Picture** | Modo de Steam | Gratis | UI de Steam Deck, enorme base | Centrado en Steam; no reemplaza el escritorio; no gestiona Windows | Página oficial y guías de configuración |
| **Playnite (Fullscreen)** | Biblioteca libre (MIT) | Gratis | Todas las tiendas, plugins, temas, emuladores | No es una consola entera (ajustes, Wi-Fi, navegador, nube); configuración larga | Comparativas «vs LaunchBox vs ES-DE», AlternativeTo, PC Gamer, foros |
| **LaunchBox / Big Box** | Frontend de emulación | Big Box de pago (~30-75 $) | El más visual para retro | De pago; enfocado a retro | Tutoriales, comunidad propia |
| **ES-DE / RetroBat / EmuDeck** | Frontends de emulación | Gratis | Multiplataforma, scraper | Solo emulación | Guías de handhelds y Steam Deck |
| **Winhanced** | Dashboard para handhelds | Gratis (beta) | Pensado para ROG Ally/Legion Go; prensa (Windows Central) | Joven; foco handheld | Prensa tecnológica, TechSpot |
| **PocketDeck** | Launcher para handhelds | Gratis | Optimizado para Ally/Legion/Claw | Pequeño, en itch.io | itch.io |
| **Razer Cortex Console Mode** | Modo consola de Razer | Gratis (beta) | Marca Razer | Beta, ecosistema Razer | Foro Razer Insider |
| **Armoury Crate SE / Legion Space / MSI Center M** | Apps del fabricante | Incluidas | Ajustes del equipo | Solo su hardware, criticadas | — |
| **Handheld Companion** | Utilidad de mando/TDP | Gratis | Control de mando y rendimiento | No es un launcher | GitHub, Reddit handheld |
| **CtrlUI, Console Mode (winget), OmniConsole** | Utilidades pequeñas | Gratis | Nicho | Poca visibilidad | GitHub, winget |

**Conclusión:** nadie combina «biblioteca + nube (Xbox y PS5) + emuladores + navegador con streaming + móvil como mando + arranque sin escritorio + Windows 10». Ese es el mensaje de cada comparativa.

---

## 7. Plan de acción

### 7.1 Victorias rápidas (1-2 semanas)

| # | Acción | Esfuerzo |
|---|---|---|
| 1 | Sacar del despliegue las ~90 demos de `docs/assets/**/*.html` y sus fotos (o ponerles `noindex`). | Bajo |
| 2 | Nueva meta description (146) y nuevo título en la home. | Bajo |
| 3 | Dar de alta Search Console y Bing Webmaster; enviar `sitemap.xml`; usar IndexNow. | Bajo |
| 4 | Renombrar release e instalador a **CouchDesk** (`productName`). | Bajo |
| 5 | Repo GitHub: descripción en inglés, imagen social (1280×640), primeras líneas del README en inglés con palabras clave. Propuesta: *«CouchDesk — free, open-source console mode for Windows 10/11. Steam, Epic, GOG and Xbox in one library, Xbox Cloud & PS5 Remote Play, emulators, controller or phone. A Steam Big Picture / Xbox mode alternative.»* | Bajo |
| 6 | Topics (máx. 20): cambiar `react`, `rust`, `dualshock4`, `xcloud` por `xbox-mode`, `console-mode`, `playnite-alternative`, `emulator-frontend`, `handheld` (mantener `tauri`). | Bajo |
| 7 | Completar JSON-LD (`SoftwareApplication` + `WebSite` + `Person` + `FAQPage`). | Bajo |
| 8 | Añadir a la home la línea «También en Windows 10» y un bloque de diferencias. | Bajo |
| 9 | Publicar en winget-pkgs (con `wingetcreate`) y en Scoop Extras. | Bajo-Medio |
| 10 | Fichas en AlternativeTo como alternativa a Playnite, Steam Big Picture, LaunchBox y modo Xbox. | Bajo |
| 11 | Crear `404.html` y `apple-touch-icon`. | Bajo |

### 7.2 Inversiones estratégicas (1-3 meses)

| # | Acción | Por qué |
|---|---|---|
| 1 | **Sitio multilingüe** (es, en, pt-BR, fr, de, it) con la estructura de abajo. | Multiplica el mercado; en pt/fr/de casi no hay rivales independientes. |
| 2 | **3 comparativas** (vs modo Xbox, vs Big Picture, vs Playnite) en es y en primero. | Intención comercial alta. |
| 3 | **Página de descarga** y **novedades** (desde `CHANGELOG.md`). | Captura marca y frescura. |
| 4 | **Guías** (arranque sin escritorio, PS5 Remote Play, mando PS5, emuladores, HTPC, handhelds). | Long tail con poca competencia. |
| 5 | **Vídeo demo** en YouTube (es + subtítulos en) con `VideoObject`. | YouTube es el segundo buscador; ideal para Reddit y prensa. |
| 6 | **Firma de código** con SignPath Foundation. | Quita el aviso de SmartScreen; requisito práctico para Softpedia, prensa y Microsoft Store. |
| 7 | **Dominio propio** (p. ej. `couchdesk.app`) antes de lanzar los idiomas. | Robots propio, marca, y evita migrar dos veces. |
| 8 | Campaña de lanzamiento: Show HN, Reddit, Product Hunt, prensa es (ver lista). | Enlaces y primeras estrellas. |

### 7.3 Estructura multilingüe recomendada

- **Español se queda en la raíz** `/couchdesk/` (conserva lo ya indexado, sin redirecciones).
- **Otros idiomas en subcarpetas:** `/couchdesk/en/`, `/couchdesk/pt-br/`, `/couchdesk/fr/`, `/couchdesk/de/`, `/couchdesk/it/`.
- **x-default → `/couchdesk/en/`** (público global).
- **Nunca redirigir por idioma con JS o IP.** Selector de idioma visible con enlaces reales.
- **Slugs traducidos** en cada idioma.

| Página | es | en | pt-BR | fr | de | it |
|---|---|---|---|---|---|---|
| Inicio | `/couchdesk/` | `/couchdesk/en/` | `/couchdesk/pt-br/` | `/couchdesk/fr/` | `/couchdesk/de/` | `/couchdesk/it/` |
| Descargar | `/descargar/` | `/en/download/` | `/pt-br/baixar/` | `/fr/telecharger/` | `/de/download/` | `/it/scarica/` |
| Novedades | `/novedades/` | `/en/changelog/` | `/pt-br/novidades/` | `/fr/nouveautes/` | `/de/neuigkeiten/` | `/it/novita/` |
| vs modo Xbox | `/comparativa/modo-xbox/` | `/en/vs/xbox-mode/` | `/pt-br/comparacao/modo-xbox/` | `/fr/comparatif/mode-xbox/` | `/de/vergleich/xbox-modus/` | `/it/confronto/modalita-xbox/` |
| vs Big Picture | `/comparativa/steam-big-picture/` | `/en/vs/steam-big-picture/` | … | … | … | … |
| vs Playnite | `/comparativa/playnite/` | `/en/vs/playnite/` | … | … | … | … |
| Guías | `/guias/<slug>/` | `/en/guides/<slug>/` | `/pt-br/guias/<slug>/` | `/fr/guides/<slug>/` | `/de/anleitungen/<slug>/` | `/it/guide/<slug>/` |

(Todas las rutas cuelgan de `https://pablonolascoo.github.io/couchdesk/`.)

**En el `<head>` de cada versión** (lista completa y recíproca, incluida ella misma):

```html
<link rel="canonical" href="https://pablonolascoo.github.io/couchdesk/en/">
<link rel="alternate" hreflang="es" href="https://pablonolascoo.github.io/couchdesk/">
<link rel="alternate" hreflang="en" href="https://pablonolascoo.github.io/couchdesk/en/">
<link rel="alternate" hreflang="pt-BR" href="https://pablonolascoo.github.io/couchdesk/pt-br/">
<link rel="alternate" hreflang="fr" href="https://pablonolascoo.github.io/couchdesk/fr/">
<link rel="alternate" hreflang="de" href="https://pablonolascoo.github.io/couchdesk/de/">
<link rel="alternate" hreflang="it" href="https://pablonolascoo.github.io/couchdesk/it/">
<link rel="alternate" hreflang="x-default" href="https://pablonolascoo.github.io/couchdesk/en/">
```

Además: `<html lang="…">`, `og:locale` + `og:locale:alternate`, `inLanguage` en JSON-LD y textos traducidos por nativo o revisados (no traducción automática sin revisar).

**Sitemap con alternativas** (una entrada `<url>` por idioma, cada una con todas las alternativas):

```xml
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"
        xmlns:xhtml="http://www.w3.org/1999/xhtml">
  <url>
    <loc>https://pablonolascoo.github.io/couchdesk/</loc>
    <lastmod>2026-11-21</lastmod>
    <xhtml:link rel="alternate" hreflang="es" href="https://pablonolascoo.github.io/couchdesk/"/>
    <xhtml:link rel="alternate" hreflang="en" href="https://pablonolascoo.github.io/couchdesk/en/"/>
    <xhtml:link rel="alternate" hreflang="pt-BR" href="https://pablonolascoo.github.io/couchdesk/pt-br/"/>
    <xhtml:link rel="alternate" hreflang="fr" href="https://pablonolascoo.github.io/couchdesk/fr/"/>
    <xhtml:link rel="alternate" hreflang="de" href="https://pablonolascoo.github.io/couchdesk/de/"/>
    <xhtml:link rel="alternate" hreflang="it" href="https://pablonolascoo.github.io/couchdesk/it/"/>
    <xhtml:link rel="alternate" hreflang="x-default" href="https://pablonolascoo.github.io/couchdesk/en/"/>
  </url>
  <!-- repetir el bloque con <loc> = /en/, /pt-br/, /fr/, /de/, /it/ -->
</urlset>
```

Generar páginas, hreflang y sitemap con un script (una plantilla + un JSON de textos por idioma) para que nunca se desincronicen. Un `llms.txt` por idioma o uno en inglés con enlaces a todos.

### 7.4 Dónde publicar (enlaces y menciones)

> Las URL de envío pueden cambiar; comprobarlas antes. Publicar en foros y Reddit lo hace el autor con su cuenta, respetando las normas de cada comunidad (muchas exigen participar antes de autopromocionarse).

| Categoría | Destino | URL | Nota |
|---|---|---|---|
| Gestores de paquetes | winget-pkgs | https://github.com/microsoft/winget-pkgs | Manifiesto con `wingetcreate new <url del .exe>` |
| | Scoop (bucket Extras) | https://github.com/ScoopInstaller/Extras | PR con manifiesto JSON |
| | Chocolatey | https://community.chocolatey.org/packages/upload | Requiere moderación |
| | Microsoft Store | https://partner.microsoft.com/dashboard | Cuenta individual gratuita; muy buena visibilidad (“game launcher”) |
| Directorios | AlternativeTo | https://alternativeto.net | Añadir app y marcarla como alternativa a Playnite, Steam Big Picture, LaunchBox, modo Xbox; proponerla en la lista «Windows Handheld Compendium» |
| | Product Hunt | https://www.producthunt.com/posts/new | Lanzar en martes-jueves con vídeo |
| | Softpedia | https://www.softpedia.com/user/submit.shtml | Mejor con instalador firmado |
| | Uptodown | https://developers.uptodown.com | Muy fuerte en es y pt-BR |
| | MajorGeeks | https://www.majorgeeks.com | Envío por contacto |
| | TechSpot Downloads | https://www.techspot.com/downloads/ | Ya lista Winhanced y Playnite |
| | itch.io (herramientas) | https://itch.io/game/new | Como PocketDeck |
| | SourceForge (espejo de GitHub) | https://sourceforge.net/create/ | Importa releases automáticamente |
| Listas de GitHub | awesome-tauri | https://github.com/tauri-apps/awesome-tauri | PR en «Applications» |
| | Emulation General Wiki – Launchers | https://emulation.gametechwiki.com/index.php/Launcher | Editar la tabla de frontends |
| | Buscar listas `awesome-game-launchers`, `awesome-htpc`, `awesome-windows` | https://github.com/search?q=awesome+windows&type=repositories | PR donde encaje (p. ej. awesome-windows de Awesome-Windows) |
| Comunidades | Hacker News (Show HN) | https://news.ycombinator.com/submit | «Show HN: CouchDesk – open-source console mode for Windows (Tauri)» |
| | r/pcgaming | https://www.reddit.com/r/pcgaming/ | Ángulo: alternativa al modo Xbox que va en Win10 |
| | r/emulation | https://www.reddit.com/r/emulation/ | Ángulo: instalador de emuladores con rollback |
| | r/ROGAlly | https://www.reddit.com/r/ROGAlly/ | Tras validar en handheld |
| | r/LegionGo | https://www.reddit.com/r/LegionGo/ | Ídem |
| | r/htpc | https://www.reddit.com/r/htpc/ | Ángulo: arranque sin escritorio |
| | r/couchgaming | https://www.reddit.com/r/couchgaming/ | Público exacto |
| | r/Playnite | https://www.reddit.com/r/playnite/ | Con cuidado: tono de comparación honesta |
| | r/opensource, r/software, r/Windows11 | https://www.reddit.com/r/opensource/ | Lanzamiento |
| | r/rust, r/tauri (o Discord de Tauri #showcase) | https://discord.com/invite/tauri | Ángulo técnico |
| | r/PS5 / r/xcloud | https://www.reddit.com/r/xcloud/ | Ángulo: nube y Remote Play con mando |
| Comunidades es | Mediavida (Hardware/Juegos), ForoCoches, 3DJuegos foros, r/esGaming | https://www.mediavida.com | Primero en español: es el idioma nativo del proyecto |
| Prensa es | Xataka / Genbeta, MuyComputer, Hipertextual, Vandal, Korben (es), mundobytes, pchardwarepro | https://www.genbeta.com/contacto | Todos han cubierto el modo Xbox: enviar «alternativa gratis que va en Windows 10» |
| Prensa en | Windows Central, Neowin (software news), Liliputing, Digital Trends, Tom's Guide | https://www.neowin.net/news/tips/ | Windows Central cubrió Winhanced: mismo ángulo |
| Confianza | SignPath Foundation (firma gratuita OSS) | https://signpath.org | Quita el aviso de SmartScreen |

### 7.5 Orden sugerido

| Semana | Entregables |
|---|---|
| 1 | Limpiar demos, título/description, Search Console + Bing, renombrar release, repo en inglés, AlternativeTo, winget. |
| 2-3 | Home en inglés + hreflang + sitemap con alternativas; comparativa vs modo Xbox (es/en); página de descarga. |
| 4-5 | Vs Big Picture y vs Playnite (es/en); novedades; vídeo demo; Show HN + Reddit + Product Hunt. |
| 6-10 | pt-BR, fr, de, it; guías; Microsoft Store; firma con SignPath; prensa. |

**Métricas a seguir:** impresiones y clics por idioma (Search Console), posición para «modo consola windows 10», «xbox mode alternative», «steam big picture alternative»; descargas por release (API de GitHub); estrellas; enlaces entrantes (Search Console › Enlaces).
