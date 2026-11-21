"""Genera los conceptos de marca de CouchDesk (SVG en negro, sin texto vivo).
Uso: python brand/build_concepts.py  → brand/concepts/*.svg
"""
import math
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "concepts"
OUT.mkdir(exist_ok=True)
FONT = ROOT.parent / "node_modules/@fontsource/instrument-serif/files/instrument-serif-latin-400-normal.woff"


def pt(cx, cy, r, deg):
    a = math.radians(deg)
    return cx + r * math.cos(a), cy - r * math.sin(a)


def f(v):
    return f"{v:.2f}".rstrip("0").rstrip(".")


def ring_with_gap(cx, cy, r_out, r_in, start, end):
    """Anillo abierto de `start` a `end` grados (sentido antihorario), con extremos redondos."""
    cap = (r_out - r_in) / 2
    o1, o2 = pt(cx, cy, r_out, start), pt(cx, cy, r_out, end)
    i1, i2 = pt(cx, cy, r_in, start), pt(cx, cy, r_in, end)
    large = 1 if (end - start) % 360 > 180 else 0
    return (
        f"M{f(o1[0])} {f(o1[1])}"
        f"A{f(r_out)} {f(r_out)} 0 {large} 0 {f(o2[0])} {f(o2[1])}"
        f"A{f(cap)} {f(cap)} 0 0 0 {f(i2[0])} {f(i2[1])}"
        f"A{f(r_in)} {f(r_in)} 0 {large} 1 {f(i1[0])} {f(i1[1])}"
        f"A{f(cap)} {f(cap)} 0 0 0 {f(o1[0])} {f(o1[1])}Z"
    )


def circle(cx, cy, r):
    return f"M{f(cx - r)} {f(cy)}a{f(r)} {f(r)} 0 1 0 {f(2 * r)} 0a{f(r)} {f(r)} 0 1 0 {f(-2 * r)} 0Z"


def svg(paths, w=256, h=256, evenodd=False):
    rule = ' fill-rule="evenodd"' if evenodd else ""
    body = "".join(f'<path d="{d}"{rule}/>' for d in paths)
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">{body}</svg>\n'


# ---------- A · Standby: una C abierta con la luz de encendido en el hueco ----------
def concept_a():
    cx, cy, ro, ri = 124, 128, 104, 66
    gap = 38  # grados a cada lado del eje horizontal
    ring = ring_with_gap(cx, cy, ro, ri, gap, 360 - gap)
    dot = circle(cx + (ro + ri) / 2 + 2, cy, (ro - ri) / 2 + 1)
    return [ring, dot]


# ---------- B · Chaise: una sola línea que es C y también un sillón reclinado ----------
def concept_b():
    # Trazo de grosor fijo construido como contorno (sin stroke en el archivo final)
    w = 34
    cx, cy, r = 118, 118, 78
    ro, ri = r + w / 2, r - w / 2
    start = 48  # extremo superior derecho
    o1 = pt(cx, cy, ro, start)
    i1 = pt(cx, cy, ri, start)
    seat_y_o, seat_y_i = cy + ro, cy + ri  # borde inferior / superior del asiento
    x_end = 214
    arm_top = cy + 34
    capr = w / 2
    d = (
        f"M{f(i1[0])} {f(i1[1])}"
        f"A{f(capr)} {f(capr)} 0 0 0 {f(o1[0])} {f(o1[1])}"  # tapa redonda arriba
        f"A{f(ro)} {f(ro)} 0 1 0 {f(cx)} {f(seat_y_o)}"      # exterior de la C hasta abajo
        f"H{f(x_end - capr)}"
        f"A{f(capr)} {f(capr)} 0 0 0 {f(x_end)} {f(seat_y_o - capr)}"  # esquina del reposabrazos
        f"V{f(arm_top)}"
        f"A{f(capr)} {f(capr)} 0 0 0 {f(x_end - w)} {f(arm_top)}"      # tapa del reposabrazos
        f"V{f(seat_y_i)}"
        f"H{f(cx)}"
        f"A{f(ri)} {f(ri)} 0 1 1 {f(i1[0])} {f(i1[1])}Z"     # interior de la C de vuelta arriba
    )
    return [d]


# ---------- C · Pantalla: una D (pantalla/escritorio) con la luz encendida dentro ----------
def concept_c():
    x0, y0, y1, w = 44, 36, 220, 36
    h = y1 - y0
    r = h / 2
    xs = 108  # donde empieza la curva de la D
    outer = (f"M{x0 + 18} {y0}H{xs}A{f(r)} {f(r)} 0 0 1 {xs} {y1}H{x0 + 18}"
             f"A18 18 0 0 1 {x0} {y1 - 18}V{y0 + 18}A18 18 0 0 1 {x0 + 18} {y0}Z")
    ri = r - w
    inner = (f"M{x0 + w} {y0 + w}H{xs}A{f(ri)} {f(ri)} 0 0 1 {xs} {y1 - w}H{x0 + w}Z")
    dot = circle(xs + 6, (y0 + y1) / 2, 18)
    return [outer + inner, dot]


# ---------- Logotipo «CouchDesk» en trazos (Instrument Serif, licencia OFL) ----------
def wordmark(text="CouchDesk", height=256, tracking=-8):
    font = TTFont(FONT)
    gs = font.getGlyphSet()
    cmap = font.getBestCmap()
    upm = font["head"].unitsPerEm
    cap = font["OS/2"].sCapHeight or 700
    scale = (height * 0.62) / cap
    base = height * 0.81
    x = 0
    paths = []
    for ch in text:
        name = cmap[ord(ch)]
        pen = SVGPathPen(gs)
        tp = TransformPen(pen, (scale, 0, 0, -scale, x, base))
        gs[name].draw(tp)
        paths.append(pen.getCommands())
        x += gs[name].width * scale + tracking
    return paths, x - tracking


def lockup(symbol_paths, name):
    words, ww = wordmark()
    gap = 56
    # Símbolo a la altura de la caja alta, centrado ópticamente con el texto
    s = 0.92
    sym = f'<g transform="translate(0 {f(256 * (1 - s) / 2)}) scale({s})">' + "".join(
        f'<path d="{d}" fill-rule="evenodd"/>' for d in symbol_paths) + "</g>"
    txt = f'<g transform="translate({f(256 * s + gap)} 0)">' + "".join(f'<path d="{d}"/>' for d in words) + "</g>"
    w = 256 * s + gap + ww
    (OUT / f"{name}-lockup.svg").write_text(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {f(w)} 256" width="{f(w)}" height="256">{sym}{txt}</svg>\n', encoding="utf-8")


for key, fn in {"a-standby": concept_a, "b-chaise": concept_b, "c-cojin": concept_c}.items():
    p = fn()
    (OUT / f"{key}.svg").write_text(svg(p, evenodd=True), encoding="utf-8")
    lockup(p, key)
print("ok", sorted(x.name for x in OUT.iterdir()))
