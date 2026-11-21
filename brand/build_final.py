"""Kit final de la marca CouchDesk (dirección A · Standby).
python brand/build_final.py  → brand/final/*.svg
Colores:
  Tinta   #1F1A17  (carbón cálido)
  Lino    #F3ECE1  (fondo claro)
  Brasa   #E8643A  (la luz de encendido)
"""
from pathlib import Path
import build_concepts as bc

INK, LINEN, EMBER = "#1F1A17", "#F3ECE1", "#E8643A"
OUT = Path(__file__).resolve().parent / "final"
OUT.mkdir(exist_ok=True)

ring, dot = bc.concept_a()


def sym(ring_fill, dot_fill, size=256, tile=None, pad=0.0, radius=0.23):
    inner = f'<path d="{ring}" fill="{ring_fill}"/><path d="{dot}" fill="{dot_fill}"/>'
    if tile:
        s = 1 - 2 * pad
        inner = (f'<rect width="256" height="256" rx="{256 * radius:.1f}" fill="{tile}"/>'
                 f'<g transform="translate({256 * pad:.1f} {256 * pad:.1f}) scale({s:.4f})">{inner}</g>')
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="{size}" height="{size}">{inner}</svg>\n'


def write(name, text):
    (OUT / name).write_text(text, encoding="utf-8")


write("symbol.svg", sym(INK, EMBER))
write("symbol-reversed.svg", sym(LINEN, EMBER))
write("symbol-black.svg", sym("#000", "#000"))
write("symbol-white.svg", sym("#fff", "#fff"))
# Icono de la app: baldosa de tinta, C de lino y la luz encendida
write("app-icon.svg", sym(LINEN, EMBER, size=1024, tile=INK, pad=0.2))
write("app-icon-light.svg", sym(INK, EMBER, size=1024, tile=LINEN, pad=0.2))

# Logotipo horizontal en color y en negativo
words, ww = bc.wordmark()
s, gap = 0.92, 56


def lock(ink, ember, name):
    g = (f'<g transform="translate(0 {256 * (1 - s) / 2:.1f}) scale({s})"><path d="{ring}" fill="{ink}"/><path d="{dot}" fill="{ember}"/></g>'
         f'<g transform="translate({256 * s + gap:.1f} 0)" fill="{ink}">' + "".join(f'<path d="{d}"/>' for d in words) + "</g>")
    w = 256 * s + gap + ww
    write(name, f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w:.1f} 256" width="{w:.1f}" height="256">{g}</svg>\n')


lock(INK, EMBER, "lockup.svg")
lock(LINEN, EMBER, "lockup-reversed.svg")
lock("#000", "#000", "lockup-black.svg")
write("wordmark.svg", f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {ww:.1f} 256" width="{ww:.1f}" height="256" fill="{INK}">'
      + "".join(f'<path d="{d}"/>' for d in words) + "</svg>\n")
print("ok", sorted(p.name for p in OUT.iterdir()))
