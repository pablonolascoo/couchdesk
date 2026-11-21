//! A qué zona va cada cosa del escritorio. Lógica pura: lo que hay que leer de Windows
//! (destino de un acceso directo, si es carpeta) llega ya resuelto en `DeskItem`.

use super::config::{Config, Rule};
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeskItem {
    /// Nombre que enseña Windows
    pub name: String,
    /// Nombre completo de Windows (ruta o ::{GUID}); sirve de identificador
    pub parsing: String,
    /// Ruta en disco, si es un archivo o carpeta de verdad
    pub path: Option<String>,
    pub is_dir: bool,
    /// Destino de un acceso directo (.lnk: ruta; .url: dirección)
    pub target: Option<String>,
}

const DOCS: &[&str] = &["pdf", "doc", "docx", "xls", "xlsx", "xlsm", "ppt", "pptx", "txt", "md", "odt", "ods", "odp", "rtf", "csv", "epub", "pages", "numbers", "key", "json", "xml", "log"];
const MEDIA: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "heic", "heif", "avif", "svg", "tif", "tiff", "raw", "psd", "mp4", "mkv", "mov", "avi", "webm", "wmv", "m4v", "flv"];
const ARCHIVES: &[&str] = &["zip", "7z", "rar", "tar", "gz", "tgz", "bz2", "xz", "iso", "img", "cab", "msi", "msix", "msixbundle", "appx", "appxbundle"];
const INSTALLER_WORDS: &[&str] = &["setup", "install", "instalador", "installer", "instalar", "redist", "updater", "_x64", "_x86", "-x64", "-x86", "win64", "win32"];
const GAME_URIS: &[&str] = &["steam://", "com.epicgames.launcher://", "uplay://", "battlenet://", "goggalaxy://", "origin://", "origin2://", "eadm://", "itch://", "riotclient://", "heroic://"];

fn ext_of(s: &str) -> String {
    let file = s.rsplit(['\\', '/']).next().unwrap_or(s);
    match file.rsplit_once('.') {
        Some((stem, e)) if !stem.is_empty() => e.to_lowercase(),
        _ => String::new(),
    }
}

fn rule_matches(r: &Rule, item: &DeskItem) -> bool {
    let v = r.value.trim().to_lowercase();
    if v.is_empty() {
        return false;
    }
    match r.kind.as_str() {
        "ext" => {
            let src = item.path.as_deref().unwrap_or(&item.parsing);
            !item.is_dir && ext_of(src) == v.trim_start_matches('.')
        }
        "name" => item.name.to_lowercase().contains(&v),
        _ => false,
    }
}

/// Categoría de serie (sin reglas del usuario)
pub fn category(item: &DeskItem) -> &'static str {
    if item.is_dir {
        return "carpetas";
    }
    // Papelera, Este equipo, Red…: no son archivos
    let Some(path) = item.path.as_deref() else { return "otros" };
    let ext = ext_of(path);
    match ext.as_str() {
        "lnk" | "url" | "appref-ms" => {
            let target = item.target.as_deref().unwrap_or("").to_lowercase();
            if GAME_URIS.iter().any(|u| target.starts_with(u)) || crate::apps::looks_like_game(&item.name, &target) {
                "juegos"
            } else if !target.is_empty() && std::path::Path::new(&target).is_dir() {
                "carpetas"
            } else {
                let t_ext = ext_of(&target);
                if DOCS.contains(&t_ext.as_str()) {
                    "documentos"
                } else if MEDIA.contains(&t_ext.as_str()) {
                    "multimedia"
                } else {
                    "apps"
                }
            }
        }
        "exe" | "bat" | "cmd" => {
            let n = item.name.to_lowercase();
            if INSTALLER_WORDS.iter().any(|w| n.contains(w)) {
                "comprimidos"
            } else if crate::apps::looks_like_game(&item.name, path) {
                "juegos"
            } else {
                "apps"
            }
        }
        e if DOCS.contains(&e) => "documentos",
        e if MEDIA.contains(&e) => "multimedia",
        e if ARCHIVES.contains(&e) => "comprimidos",
        _ => "otros",
    }
}

/// Zona final: primero las reglas del usuario (la primera que encaja), luego la de serie;
/// si esa zona está oculta, "otros".
pub fn zone_for(item: &DeskItem, cfg: &Config) -> String {
    if let Some(r) = cfg.rules.iter().find(|r| rule_matches(r, item)) {
        return cfg.target_zone(&r.zone).to_string();
    }
    cfg.target_zone(category(item)).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str) -> DeskItem {
        let path = format!("C:\\Users\\Ana\\Desktop\\{name}");
        DeskItem { name: name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name).into(), parsing: path.clone(), path: Some(path), ..Default::default() }
    }

    fn link(name: &str, target: &str) -> DeskItem {
        DeskItem { target: Some(target.into()), ..file(name) }
    }

    #[test]
    fn categorias_de_serie() {
        assert_eq!(category(&link("Hades.url", "steam://rungameid/1145360")), "juegos");
        assert_eq!(category(&link("Fortnite.url", "com.epicgames.launcher://apps/Fortnite?action=launch")), "juegos");
        assert_eq!(category(&link("Witcher.lnk", "D:\\GOG Games\\The Witcher 3\\witcher3.exe")), "juegos");
        assert_eq!(category(&link("Steam.lnk", "C:\\Program Files (x86)\\Steam\\steam.exe")), "apps", "la tienda no es un juego");
        assert_eq!(category(&link("Chrome.lnk", "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe")), "apps");
        assert_eq!(category(&link("Banco.url", "https://banco.es")), "apps");
        assert_eq!(category(&link("Informe.lnk", "C:\\Docs\\informe.PDF")), "documentos");
        assert_eq!(category(&file("factura.pdf")), "documentos");
        assert_eq!(category(&file("notas.MD")), "documentos");
        assert_eq!(category(&file("tabla.xlsx")), "documentos");
        assert_eq!(category(&file("foto.JPG")), "multimedia");
        assert_eq!(category(&file("clip.mp4")), "multimedia");
        assert_eq!(category(&file("cosas.zip")), "comprimidos");
        assert_eq!(category(&file("node-v22.msi")), "comprimidos");
        assert_eq!(category(&file("ChromeSetup.exe")), "comprimidos", "instalador");
        assert_eq!(category(&file("herramienta.exe")), "apps");
        assert_eq!(category(&file("cancion.mp3")), "otros");
        assert_eq!(category(&file("sin_extension")), "otros");
        assert_eq!(category(&file(".gitignore")), "otros");
        let dir = DeskItem { is_dir: true, ..file("Proyectos") };
        assert_eq!(category(&dir), "carpetas");
        let bin = DeskItem { name: "Papelera".into(), parsing: "::{645FF040-5081-101B-9F08-00AA002F954E}".into(), ..Default::default() };
        assert_eq!(category(&bin), "otros");
    }

    #[test]
    fn reglas_del_usuario_mandan() {
        let mut cfg = Config::default();
        cfg.rules.push(Rule { kind: "ext".into(), value: ".MP3".into(), zone: "multimedia".into() });
        cfg.rules.push(Rule { kind: "name".into(), value: "tfg".into(), zone: "documentos".into() });
        assert_eq!(zone_for(&file("cancion.mp3"), &cfg), "multimedia");
        assert_eq!(zone_for(&file("TFG final.zip"), &cfg), "documentos");
        // Una carpeta no tiene extensión
        let dir = DeskItem { is_dir: true, ..file("musica.mp3") };
        assert_eq!(zone_for(&dir, &cfg), "carpetas");
        // Zona oculta → otros
        cfg.zones.iter_mut().find(|z| z.id == "documentos").unwrap().hidden = true;
        assert_eq!(zone_for(&file("TFG final.zip"), &cfg), "otros");
        assert_eq!(zone_for(&file("carta.pdf"), &cfg), "otros");
    }
}
