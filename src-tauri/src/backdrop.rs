//! Fondos de pantalla de la consola: busca imágenes, vídeos y modelos 3D en las carpetas
//! personales y copia el elegido a %LOCALAPPDATA%\Consola\fondos (la app solo puede leer de ahí).

use crate::i18n::{tr, trf};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Serialize, Clone, Debug)]
pub struct MediaFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    /// Segundos desde 1970
    pub modified: u64,
    /// Carpeta de origen, para enseñarla bajo el nombre ("Imágenes", "Descargas"…)
    pub folder: String,
}

const IMAGE: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "avif", "bmp"];
const VIDEO: &[&str] = &["mp4", "m4v", "webm", "ogv"];
const MODEL: &[&str] = &["glb", "gltf"];

/// Tipo de fondo según la extensión: "image" | "video" | "model"
pub fn kind_of(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let e = ext.as_str();
    if IMAGE.contains(&e) {
        Some("image")
    } else if VIDEO.contains(&e) {
        Some("video")
    } else if MODEL.contains(&e) {
        Some("model")
    } else {
        None
    }
}

pub fn dir() -> PathBuf {
    crate::paths::data_dir().join("fondos")
}

fn home() -> PathBuf {
    std::env::var("USERPROFILE").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("C:\\"))
}

fn is_hidden(meta: &fs::Metadata, name: &str) -> bool {
    use std::os::windows::fs::MetadataExt;
    let attrs = meta.file_attributes();
    attrs & 0x2 != 0 || attrs & 0x4 != 0 || name.starts_with('.') || name.starts_with('$')
}

/// Recorre una carpeta (y subcarpetas hasta `depth`) guardando los archivos del tipo pedido
fn scan(dir: &Path, kind: &str, folder: &str, depth: u32, out: &mut Vec<MediaFile>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        if out.len() >= 2000 {
            return;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        let Ok(meta) = e.metadata() else { continue };
        if is_hidden(&meta, &name) {
            continue;
        }
        let path = e.path();
        if meta.is_dir() {
            if depth > 0 {
                scan(&path, kind, folder, depth - 1, out);
            }
        } else if kind_of(&path) == Some(kind) {
            out.push(MediaFile {
                name,
                path: path.to_string_lossy().into_owned(),
                size: meta.len(),
                modified: meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs()),
                folder: folder.to_string(),
            });
        }
    }
}

/// Imágenes, vídeos o modelos 3D del PC, los más nuevos primero
#[tauri::command(async)]
pub fn backdrop_list_media(kind: String) -> Result<Vec<MediaFile>, String> {
    if !["image", "video", "model"].contains(&kind.as_str()) {
        return Err(trf("backdrop.unknown_kind", &[("kind", &kind)]));
    }
    let h = home();
    let mut out = Vec::new();
    // Ya importados antes: primero, para volver a elegirlos rápido
    scan(&dir(), &kind, tr("backdrop.saved"), 1, &mut out);
    for (label, sub) in [("files.pictures", "Pictures"), ("files.videos", "Videos"), ("files.downloads", "Downloads"), ("files.desktop", "Desktop")] {
        scan(&h.join(sub), &kind, tr(label), 2, &mut out);
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out.truncate(300);
    Ok(out)
}

/// Rutas relativas a otros archivos dentro de un .gltf (buffers .bin y texturas)
pub fn gltf_uris(json: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return Vec::new() };
    let mut out = Vec::new();
    for key in ["buffers", "images"] {
        for item in v.get(key).and_then(|x| x.as_array()).into_iter().flatten() {
            if let Some(uri) = item.get("uri").and_then(|u| u.as_str()) {
                let lower = uri.to_ascii_lowercase();
                let external = lower.starts_with("data:") || lower.contains("://");
                // Nunca fuera de la carpeta del modelo
                if !external && !uri.contains("..") && !uri.starts_with('/') && !uri.starts_with('\\') && !uri.contains(':') {
                    out.push(uri.to_string());
                }
            }
        }
    }
    out
}

/// Decodifica %20 y similares de una ruta del .gltf
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(n) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(n);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Copia el archivo elegido a la carpeta de fondos y devuelve su nueva ruta
#[tauri::command(async)]
pub fn backdrop_import(path: String) -> Result<String, String> {
    let src = PathBuf::from(&path);
    if !src.is_file() {
        return Err(tr("backdrop.missing").into());
    }
    let Some(kind) = kind_of(&src) else {
        return Err(tr("backdrop.bad_type").into());
    };
    let target_dir = dir();
    // Ya está en la carpeta de fondos: no hace falta copiarlo
    if src.starts_with(&target_dir) {
        return Ok(path);
    }
    let stem = src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "fondo".into());
    let ext = src.extension().map(|s| s.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let id = &crate::apps::stable_id(&path)[..8];
    let err = |e: std::io::Error| trf("backdrop.copy_failed", &[("err", &e.to_string())]);

    if kind == "model" && ext == "gltf" {
        // Un .gltf suele ir con otros archivos al lado (.bin, texturas): se copian juntos en su carpeta
        let folder = target_dir.join(format!("{stem}-{id}"));
        fs::create_dir_all(&folder).map_err(err)?;
        let json = fs::read_to_string(&src).map_err(|e| trf("backdrop.model_unreadable", &[("err", &e.to_string())]))?;
        let base = src.parent().unwrap_or(Path::new("."));
        for uri in gltf_uris(&json) {
            let rel = percent_decode(&uri);
            let from = base.join(&rel);
            let to = folder.join(&rel);
            if let Some(p) = to.parent() {
                fs::create_dir_all(p).map_err(err)?;
            }
            fs::copy(&from, &to).map_err(|_| trf("backdrop.model_part_missing", &[("file", &rel)]))?;
        }
        let to = folder.join(src.file_name().unwrap_or_default());
        fs::copy(&src, &to).map_err(err)?;
        return Ok(to.to_string_lossy().into_owned());
    }

    fs::create_dir_all(&target_dir).map_err(err)?;
    let to = target_dir.join(format!("{stem}-{id}.{ext}"));
    if !to.is_file() || to.metadata().map(|m| m.len()).ok() != src.metadata().map(|m| m.len()).ok() {
        fs::copy(&src, &to).map_err(err)?;
    }
    Ok(to.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tipo_por_extension() {
        assert_eq!(kind_of(Path::new(r"C:\a\Foto.JPG")), Some("image"));
        assert_eq!(kind_of(Path::new("playa.webp")), Some("image"));
        assert_eq!(kind_of(Path::new("clip.MP4")), Some("video"));
        assert_eq!(kind_of(Path::new("clip.webm")), Some("video"));
        assert_eq!(kind_of(Path::new("casa.glb")), Some("model"));
        assert_eq!(kind_of(Path::new("casa.gltf")), Some("model"));
        assert_eq!(kind_of(Path::new("notas.txt")), None);
        assert_eq!(kind_of(Path::new("sin_extension")), None);
        assert_eq!(kind_of(Path::new("peli.mkv")), None, "WebView2 no reproduce MKV");
    }

    #[test]
    fn lista_solo_el_tipo_pedido() {
        let tmp = std::env::temp_dir().join(format!("consola-fondos-{}", std::process::id()));
        let sub = tmp.join("sub");
        fs::create_dir_all(&sub).unwrap();
        for f in ["a.png", "b.mp4", "c.glb", "d.txt"] {
            fs::write(tmp.join(f), b"x").unwrap();
        }
        fs::write(sub.join("e.jpeg"), b"x").unwrap();
        let mut out = Vec::new();
        scan(&tmp, "image", "Prueba", 1, &mut out);
        let mut names: Vec<_> = out.iter().map(|m| m.name.as_str()).collect();
        names.sort();
        assert_eq!(names, ["a.png", "e.jpeg"]);
        let mut shallow = Vec::new();
        scan(&tmp, "image", "Prueba", 0, &mut shallow);
        assert_eq!(shallow.len(), 1, "sin bajar a subcarpetas");
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn uris_del_gltf_seguras() {
        let json = r#"{"buffers":[{"uri":"scene.bin"},{"uri":"data:application/octet-stream;base64,AAA"}],
          "images":[{"uri":"tex/Base%20Color.png"},{"uri":"../fuera.png"},{"uri":"https://x.com/a.png"},{"bufferView":1}]}"#;
        assert_eq!(gltf_uris(json), vec!["scene.bin".to_string(), "tex/Base%20Color.png".to_string()]);
        assert_eq!(percent_decode("tex/Base%20Color.png"), "tex/Base Color.png");
        assert!(gltf_uris("no es json").is_empty());
    }
}
