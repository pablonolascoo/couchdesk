# CouchDesk · Marca

## La idea
**Standby.** Una C abierta con la luz de encendido en el hueco. Es el piloto de la tele del salón: está ahí, tranquila, lista. También se lee **«C:»**, el disco de Windows: CouchDesk es Windows convertido en salón.

Personalidad: tranquila, cálida, elegante, sencilla. Nada de estética «gamer agresiva» (neón, ángulos, rayos).

## Archivos
| Archivo | Uso |
|---|---|
| `final/symbol.svg` | Símbolo a color sobre fondo claro |
| `final/symbol-reversed.svg` | Símbolo sobre fondo oscuro |
| `final/symbol-black.svg` / `symbol-white.svg` | Una tinta |
| `final/lockup.svg` / `lockup-reversed.svg` / `lockup-black.svg` | Símbolo + nombre en horizontal |
| `final/wordmark.svg` | Solo el nombre |
| `final/app-icon.svg` | Icono de la app (baldosa carbón) |
| `web-icons/*` | favicon.ico, favicon.svg, apple-touch-icon, iconos 192/512, manifest |

Regenerar: `python brand/build_concepts.py` y `python brand/build_final.py`.

## Color
| Nombre | HEX | Uso |
|---|---|---|
| **Tinta** | `#1F1A17` | Texto, símbolo, fondos oscuros |
| **Lino** | `#F3ECE1` | Fondo claro principal |
| **Brasa** | `#E8643A` | La luz: acento, foco, acciones principales. Poco y siempre con intención |
| Arena | `#E6DCCB` | Superficies y bordes suaves |
| Taupe | `#8C7F72` | Texto secundario |
| Noche | `#14110F` | Fondo del modo oscuro |

Reglas: la Brasa no se usa para texto largo. Sobre Lino, el texto es Tinta (contraste 15:1). Sobre Brasa, el texto es Tinta, no blanco.

## Tipografía
- **Títulos:** Instrument Serif (licencia OFL). Grande, con poco interlineado.
- **Interfaz y texto:** Geist Variable. Mono: Geist Mono.
- El logotipo está convertido en trazos: no hace falta la fuente para usarlo.

## Uso del logo
- **Espacio libre:** alrededor del símbolo, como mínimo el diámetro de la luz (el punto).
- **Tamaño mínimo:** 16 px el símbolo; 96 px de ancho el logotipo horizontal.
- **Fondos:** Lino, blanco o Tinta. Sobre fotos, solo en una zona tranquila y con la versión en negativo.
- **No:** girarlo, cambiar el color de la luz por otro que no sea Brasa (salvo una tinta), añadir sombras, degradados o contornos, ni separar la luz de la C.

## Movimiento
Lento y suave, como respirar. La luz puede «latir» (opacidad 1 → 0,55 → 1 en 2,4 s) al arrancar o en espera. Nada de rebotes bruscos.
