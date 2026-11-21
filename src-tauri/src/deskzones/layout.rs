//! Cuentas de colocación: dónde va cada zona y cada icono. Todo en píxeles de pantalla.
//! Las zonas se ponen de izquierda a derecha y de arriba abajo, cada una del tamaño justo
//! para sus iconos, sin pisarse y sin bajar nunca de la zona de trabajo (la barra de tareas queda libre).

use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn right(&self) -> i32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }
    pub fn overlaps(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
    pub fn contains(&self, o: &Rect) -> bool {
        o.x >= self.x && o.y >= self.y && o.right() <= self.right() && o.bottom() <= self.bottom()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// Hueco de un icono (lo que Windows deja entre iconos)
    pub cell_w: i32,
    pub cell_h: i32,
    /// Separación entre zonas y con el borde
    pub gutter: i32,
    /// Margen interior de cada zona
    pub pad: i32,
    /// Alto del título de la zona
    pub header: i32,
}

impl Metrics {
    /// Medidas de la marca a partir del hueco del icono y la escala de la pantalla (1.0 = 96 ppp)
    pub fn for_cell(cell_w: i32, cell_h: i32, scale: f32) -> Self {
        let s = |v: f32| (v * scale).round() as i32;
        Self { cell_w: cell_w.max(1), cell_h: cell_h.max(1), gutter: s(18.0), pad: s(10.0), header: s(38.0) }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZoneBox {
    pub id: String,
    pub rect: Rect,
    pub cols: usize,
    pub count: usize,
}

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
pub struct Layout {
    pub zones: Vec<ZoneBox>,
    /// false = no caben las zonas: los iconos van en una cuadrícula sencilla, sin marcos
    pub fits: bool,
}

const MIN_COLS: usize = 2;

fn try_shelves(zones: &[(String, usize)], m: &Metrics, area: Rect, max_cols: usize) -> Option<Vec<ZoneBox>> {
    let (left, right, bottom) = (area.x + m.gutter, area.right() - m.gutter, area.bottom() - m.gutter);
    let (mut x, mut y, mut shelf_h) = (left, area.y + m.gutter, 0);
    let mut out = Vec::new();
    // Columnas y alto de una zona con `count` iconos si empieza en la altura `top`
    let size = |count: usize, top: i32| -> (usize, i32, i32) {
        let mut cols = count.clamp(MIN_COLS, max_cols.max(MIN_COLS));
        let rows_fit = ((bottom - top - m.header - 2 * m.pad) / m.cell_h).max(0) as usize;
        if rows_fit >= 1 && count.div_ceil(cols) > rows_fit {
            cols = count.div_ceil(rows_fit).max(MIN_COLS); // poco alto: más ancha
        }
        let rows = count.div_ceil(cols) as i32;
        (cols, 2 * m.pad + cols as i32 * m.cell_w, m.header + 2 * m.pad + rows * m.cell_h)
    };
    for (id, count) in zones.iter().filter(|(_, c)| *c > 0) {
        let (mut cols, mut w, mut h) = size(*count, y);
        if x > left && (x + w > right || y + h > bottom) {
            // Siguiente fila de zonas
            x = left;
            y += shelf_h + m.gutter;
            shelf_h = 0;
            (cols, w, h) = size(*count, y);
        }
        if x + w > right || y + h > bottom {
            return None;
        }
        out.push(ZoneBox { id: id.clone(), rect: Rect { x, y, w, h }, cols, count: *count });
        x += w + m.gutter;
        shelf_h = shelf_h.max(h);
    }
    Some(out)
}

/// Coloca las zonas (en su orden; las vacías no salen) dentro de la zona de trabajo `area`.
pub fn layout(zones: &[(String, usize)], m: &Metrics, area: Rect) -> Layout {
    // Primero zonas recogidas (4 columnas como mucho); si no caben, más anchas
    for max_cols in 4..=64 {
        if let Some(z) = try_shelves(zones, m, area, max_cols) {
            return Layout { zones: z, fits: true };
        }
    }
    Layout { zones: Vec::new(), fits: false }
}

/// Esquina de arriba a la izquierda del icono `i` dentro de su zona
pub fn item_pos(z: &ZoneBox, i: usize, m: &Metrics) -> (i32, i32) {
    let (col, row) = (i % z.cols, i / z.cols);
    (z.rect.x + m.pad + col as i32 * m.cell_w, z.rect.y + m.header + m.pad + row as i32 * m.cell_h)
}

/// Plan B si no caben las zonas: cuadrícula por columnas, como Windows, sin salir de `area`
pub fn plain_grid(total: usize, m: &Metrics, area: Rect) -> Vec<(i32, i32)> {
    let rows = ((area.h - m.gutter) / m.cell_h).max(1) as usize;
    let cols = ((area.w - m.gutter) / m.cell_w).max(1) as usize;
    (0..total)
        .map(|i| {
            let i = i.min(rows * cols - 1); // más iconos que huecos: se amontonan en el último
            (area.x + m.gutter / 2 + (i / rows) as i32 * m.cell_w, area.y + m.gutter / 2 + (i % rows) as i32 * m.cell_h)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1920x1080 con barra de tareas de 48 px abajo, iconos grandes a 100 %
    const AREA: Rect = Rect { x: 0, y: 0, w: 1920, h: 1032 };

    fn m() -> Metrics {
        Metrics::for_cell(110, 118, 1.0)
    }

    fn zones(counts: &[usize]) -> Vec<(String, usize)> {
        counts.iter().enumerate().map(|(i, c)| (format!("z{i}"), *c)).collect()
    }

    /// Ningún icono se pisa, cada uno dentro de su zona y todo por encima de la barra de tareas
    fn check(l: &Layout, m: &Metrics, area: Rect) {
        assert!(l.fits);
        for (i, a) in l.zones.iter().enumerate() {
            assert!(area.contains(&a.rect), "{} fuera de la zona de trabajo: {:?}", a.id, a.rect);
            for b in &l.zones[i + 1..] {
                assert!(!a.rect.overlaps(&b.rect), "{} pisa {}", a.id, b.id);
            }
            let cells: Vec<Rect> = (0..a.count)
                .map(|k| {
                    let (x, y) = item_pos(a, k, m);
                    Rect { x, y, w: m.cell_w, h: m.cell_h }
                })
                .collect();
            for (k, c) in cells.iter().enumerate() {
                assert!(a.rect.contains(c), "icono {k} se sale de {}", a.id);
                assert!(c.bottom() <= area.bottom(), "icono bajo la barra de tareas");
                assert!(cells[k + 1..].iter().all(|d| !c.overlaps(d)), "iconos pisados en {}", a.id);
            }
        }
    }

    #[test]
    fn escritorio_normal() {
        let m = m();
        let l = layout(&zones(&[6, 9, 3, 12, 0, 2, 5]), &m, AREA);
        check(&l, &m, AREA);
        assert_eq!(l.zones.len(), 6, "la zona vacía no sale");
        // Primera zona arriba a la izquierda, con el margen
        assert_eq!((l.zones[0].rect.x, l.zones[0].rect.y), (m.gutter, m.gutter));
        // De izquierda a derecha en la misma fila
        assert!(l.zones[1].rect.x > l.zones[0].rect.right());
        assert_eq!(l.zones[1].rect.y, l.zones[0].rect.y);
        // Tamaño justo: 6 iconos en 4 columnas = 2 filas
        assert_eq!(l.zones[0].cols, 4);
        assert_eq!(l.zones[0].rect.h, m.header + 2 * m.pad + 2 * m.cell_h);
        // Una zona de 1 icono mide al menos 2 huecos (cabe el título)
        let one = layout(&zones(&[1]), &m, AREA);
        assert_eq!(one.zones[0].rect.w, 2 * m.pad + 2 * m.cell_w);
    }

    #[test]
    fn muchos_iconos_ensanchan_las_zonas() {
        let m = m();
        let l = layout(&zones(&[40, 30, 25]), &m, AREA);
        check(&l, &m, AREA);
        assert!(l.zones.iter().any(|z| z.cols > 4));
    }

    #[test]
    fn barra_arriba_y_pantalla_pequena() {
        // Barra de tareas arriba: la zona de trabajo empieza más abajo
        let area = Rect { x: 0, y: 40, w: 1366, h: 728 };
        let m = Metrics::for_cell(76, 86, 1.0);
        let l = layout(&zones(&[10, 10, 10, 10, 10, 10, 10]), &m, area);
        check(&l, &m, area);
        assert!(l.zones.iter().all(|z| z.rect.y >= 40 + m.gutter));
    }

    #[test]
    fn a_150_por_ciento() {
        let area = Rect { x: 0, y: 0, w: 2880, h: 1548 };
        let m = Metrics::for_cell(165, 177, 1.5);
        let l = layout(&zones(&[3, 8, 1, 14, 6, 2, 4]), &m, area);
        check(&l, &m, area);
    }

    #[test]
    fn si_no_cabe_cuadricula_sencilla() {
        let m = m();
        let l = layout(&zones(&[200, 200]), &m, AREA);
        assert!(!l.fits && l.zones.is_empty());
        let pts = plain_grid(60, &m, AREA);
        assert_eq!(pts.len(), 60);
        for (i, p) in pts.iter().enumerate() {
            assert!(p.1 + m.cell_h <= AREA.bottom(), "icono {i} bajo la barra");
            assert!(pts[i + 1..].iter().all(|q| q != p), "iconos pisados");
        }
    }
}
