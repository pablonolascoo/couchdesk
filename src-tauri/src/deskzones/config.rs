//! Ajustes del escritorio organizado (%LOCALAPPDATA%\CouchDesk\escritorio.json) y copia del
//! escritorio de antes (escritorio-original.json) para poder dejarlo como estaba.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Zonas de serie, en su orden por defecto. "otros" siempre existe (recoge lo de las ocultas).
pub const CATEGORIES: [&str; 7] = ["juegos", "apps", "carpetas", "documentos", "multimedia", "comprimidos", "otros"];
pub const FALLBACK: &str = "otros";
/// Tamaños de icono que se ofrecen (medianos, grandes, muy grandes)
pub const ICON_SIZES: [u32; 3] = [48, 72, 96];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Zone {
    /// Una de CATEGORIES
    pub id: String,
    /// Nombre elegido por el usuario ("" = el de serie, traducido)
    pub name: String,
    pub hidden: bool,
}

impl Default for Zone {
    fn default() -> Self {
        Self { id: FALLBACK.into(), name: String::new(), hidden: false }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Rule {
    /// "ext" (la extensión es…) | "name" (el nombre contiene…)
    pub kind: String,
    pub value: String,
    /// Zona de destino
    pub zone: String,
}

impl Default for Rule {
    fn default() -> Self {
        Self { kind: "ext".into(), value: String::new(), zone: FALLBACK.into() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// Encendido de serie: la primera vez que aparece el escritorio se guarda cómo estaba y se ordena
    pub enabled: bool,
    /// Dibujar los marcos detrás de los iconos
    pub frames: bool,
    /// Tamaño de los iconos en píxeles: 48 medianos, 72 grandes, 96 muy grandes
    pub icon_size: u32,
    /// En orden de colocación
    pub zones: Vec<Zone>,
    pub rules: Vec<Rule>,
    /// Se pidió volver al escritorio de antes sin el escritorio de Windows a la vista: se hace al volver
    pub restore_pending: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            frames: true,
            icon_size: 72,
            zones: CATEGORIES.iter().map(|id| Zone { id: (*id).into(), ..Default::default() }).collect(),
            rules: Vec::new(),
            restore_pending: false,
        }
    }
}

impl Config {
    /// Arregla archivos viejos o tocados a mano: todas las zonas de serie, una vez, y "otros" visible
    pub fn normalized(mut self) -> Self {
        let mut seen = Vec::new();
        self.zones.retain(|z| CATEGORIES.contains(&z.id.as_str()) && !seen.contains(&z.id) && {
            seen.push(z.id.clone());
            true
        });
        for id in CATEGORIES {
            if !self.zones.iter().any(|z| z.id == id) {
                self.zones.push(Zone { id: id.into(), ..Default::default() });
            }
        }
        for z in &mut self.zones {
            z.name = z.name.trim().chars().take(40).collect();
            if z.id == FALLBACK {
                z.hidden = false;
            }
        }
        if !ICON_SIZES.contains(&self.icon_size) {
            self.icon_size = 72;
        }
        self.rules.retain(|r| (r.kind == "ext" || r.kind == "name") && !r.value.trim().is_empty() && CATEGORIES.contains(&r.zone.as_str()));
        self
    }

    /// Zona donde acaba algo de la categoría `cat` (si está oculta, en "otros")
    pub fn target_zone<'a>(&'a self, cat: &'a str) -> &'a str {
        match self.zones.iter().find(|z| z.id == cat) {
            Some(z) if !z.hidden => cat,
            _ => FALLBACK,
        }
    }
}

// ---------- Copia del escritorio de antes ----------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedIcon {
    /// Nombre completo de Windows (ruta del archivo o ::{GUID} de Papelera, Este equipo…)
    pub parsing: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Original {
    /// Opciones de la vista del escritorio (organizar solos, alinear a la cuadrícula…)
    pub flags: u32,
    /// Modo de vista (FOLDERVIEWMODE) y tamaño de icono de antes
    pub view_mode: i32,
    pub icon_size: i32,
    pub icons: Vec<SavedIcon>,
    /// Segundos desde 1970
    pub saved_at: u64,
}

// ---------- Disco ----------

pub fn config_path() -> PathBuf {
    crate::paths::data_dir().join("escritorio.json")
}

pub fn original_path() -> PathBuf {
    crate::paths::data_dir().join("escritorio-original.json")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &PathBuf) -> Option<T> {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
}

fn write_json<T: Serialize>(path: &PathBuf, value: &T) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    // Primero a un temporal: un corte de luz a medias no deja el archivo roto
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub fn load_from(path: &PathBuf) -> Config {
    read_json::<Config>(path).unwrap_or_default().normalized()
}

pub fn save_to(path: &PathBuf, c: &Config) -> Result<(), String> {
    write_json(path, c)
}

pub fn load() -> Config {
    load_from(&config_path())
}

pub fn save(c: &Config) -> Result<(), String> {
    save_to(&config_path(), c)
}

pub fn load_original() -> Option<Original> {
    read_json(&original_path())
}

pub fn save_original(o: &Original) -> Result<(), String> {
    write_json(&original_path(), o)
}

pub fn drop_original() {
    let _ = std::fs::remove_file(original_path());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta_en_disco() {
        let dir = std::env::temp_dir().join(format!("couchdesk-zonas-{}", std::process::id()));
        let path = dir.join("escritorio.json");
        let mut c = Config { enabled: true, ..Default::default() };
        c.zones[0].name = "Mis juegos".into();
        c.zones.swap(1, 2);
        c.zones[3].hidden = true;
        c.rules.push(Rule { kind: "ext".into(), value: "blend".into(), zone: "documentos".into() });
        save_to(&path, &c).unwrap();
        assert_eq!(load_from(&path), c);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn archivo_raro_se_arregla() {
        let c: Config = serde_json::from_str(r#"{"enabled":true,"zones":[{"id":"otros","hidden":true},{"id":"juegos"},{"id":"juegos"},{"id":"inventada"}],"rules":[{"kind":"x","value":"a","zone":"juegos"},{"kind":"name","value":" ","zone":"juegos"},{"kind":"name","value":"tfg","zone":"documentos"}]}"#).unwrap();
        let c = c.normalized();
        assert_eq!(c.zones.len(), CATEGORIES.len());
        assert_eq!(c.zones[0].id, "otros");
        assert!(!c.zones[0].hidden, "otros nunca se oculta");
        assert_eq!(c.zones[1].id, "juegos");
        assert_eq!(c.rules.len(), 1);
        assert!(c.frames, "marcos de serie");
    }

    #[test]
    fn encendido_de_serie_y_zona_oculta_va_a_otros() {
        let mut c = Config::default();
        assert!(c.enabled);
        assert_eq!(c.icon_size, 72);
        let raro: Config = serde_json::from_str(r#"{"iconSize":13}"#).unwrap();
        assert_eq!(raro.normalized().icon_size, 72);
        assert_eq!(c.target_zone("juegos"), "juegos");
        c.zones[0].hidden = true;
        assert_eq!(c.target_zone("juegos"), "otros");
    }
}
