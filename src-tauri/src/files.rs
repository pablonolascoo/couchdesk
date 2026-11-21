//! Explorador de archivos: listar, abrir, copiar, mover, nueva carpeta y papelera.

use crate::i18n::{tr, trf};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    name: String,
    path: String,
    is_dir: bool,
    size: Option<u64>,
    modified: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    label: String,
    path: String,
    /// "carpeta" | "usb" | "disco"
    kind: &'static str,
}

/// ¿Es una unidad extraíble (pincho USB, tarjeta SD)?
fn is_removable(root: &str) -> bool {
    use windows::core::HSTRING;
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;
    const DRIVE_REMOVABLE: u32 = 2;
    unsafe { GetDriveTypeW(&HSTRING::from(root)) == DRIVE_REMOVABLE }
}

fn home() -> PathBuf {
    std::env::var("USERPROFILE").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("C:\\"))
}

/// Accesos rápidos: pinchos USB primero, luego carpetas personales y discos
#[tauri::command(async)]
pub fn list_places() -> Vec<Place> {
    let h = home();
    let mut usb = Vec::new();
    let mut disks = Vec::new();
    for c in b'A'..=b'Z' {
        let root = format!("{}:\\", c as char);
        if Path::new(&root).is_dir() {
            if is_removable(&root) {
                usb.push(Place { label: format!("USB ({}:)", c as char), path: root, kind: "usb" });
            } else {
                let label = trf("files.disk", &[("letter", &(c as char).to_string())]);
                disks.push(Place { label, path: root, kind: "disco" });
            }
        }
    }
    let folders = [
        ("files.received", "Recibidos"),
        ("files.downloads", "Downloads"),
        ("files.games", "Juegos"),
        ("files.desktop", "Desktop"),
        ("files.documents", "Documents"),
        ("files.pictures", "Pictures"),
        ("files.videos", "Videos"),
        ("files.music", "Music"),
    ]
    .into_iter()
    .map(|(l, d)| (l, h.join(d)))
    .filter(|(_, p)| p.is_dir())
    .map(|(l, p)| Place { label: tr(l).to_string(), path: p.to_string_lossy().into_owned(), kind: "carpeta" });
    usb.into_iter().chain(folders).chain(disks).collect()
}

#[tauri::command(async)]
pub fn list_dir(path: String) -> Result<Vec<Entry>, String> {
    let mut out: Vec<Entry> = fs::read_dir(&path)
        .map_err(|e| trf("files.cant_open", &[("err", &e.to_string())]))?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let meta = e.metadata().ok()?;
            // Ocultar archivos de sistema y ocultos de Windows
            use std::os::windows::fs::MetadataExt;
            let attrs = meta.file_attributes();
            if attrs & 0x2 != 0 || attrs & 0x4 != 0 || name.starts_with('$') {
                return None;
            }
            Some(Entry {
                path: e.path().to_string_lossy().into_owned(),
                is_dir: meta.is_dir(),
                size: meta.is_file().then(|| meta.len()),
                modified: meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()),
                name,
            })
        })
        .collect();
    // Carpetas primero, luego por nombre
    out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(out)
}

/// Abre un archivo con su programa predeterminado (o la carpeta en el Explorador)
#[tauri::command(async)]
pub fn open_path(path: String) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err(tr("files.not_found").into());
    }
    Command::new("explorer").arg(&path).spawn().map(|_| ()).map_err(|e| e.to_string())
}

fn unique_target(dir: &Path, name: &str) -> PathBuf {
    let mut target = dir.join(name);
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    let mut n = 2;
    while target.exists() {
        target = dir.join(format!("{stem} ({n}){ext}"));
        n += 1;
    }
    target
}

fn copy_recursive(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        fs::create_dir_all(to)?;
        for e in fs::read_dir(from)? {
            let e = e?;
            copy_recursive(&e.path(), &to.join(e.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(from, to).map(|_| ())
    }
}

/// Copia o mueve elementos a una carpeta. Nunca sobrescribe: añade " (2)", " (3)"...
#[tauri::command(async)]
pub fn paste_items(items: Vec<String>, dest: String, cut: bool) -> Result<u32, String> {
    let dest = PathBuf::from(dest);
    if !dest.is_dir() {
        return Err(tr("files.dest_missing").into());
    }
    let mut done = 0;
    for item in items {
        let from = PathBuf::from(&item);
        let name = from.file_name().ok_or(tr("files.bad_path"))?.to_string_lossy().into_owned();
        if dest.starts_with(&from) {
            return Err(tr("files.into_itself").into());
        }
        let to = unique_target(&dest, &name);
        if cut {
            if fs::rename(&from, &to).is_err() {
                // Entre discos distintos: copiar y luego enviar el original a la papelera
                copy_recursive(&from, &to).map_err(|e| e.to_string())?;
                trash::delete(&from).map_err(|e| e.to_string())?;
            }
        } else {
            copy_recursive(&from, &to).map_err(|e| e.to_string())?;
        }
        done += 1;
    }
    Ok(done)
}

#[tauri::command(async)]
pub fn new_folder(dir: String) -> Result<String, String> {
    let target = unique_target(Path::new(&dir), tr("files.new_folder"));
    fs::create_dir(&target).map_err(|e| e.to_string())?;
    Ok(target.to_string_lossy().into_owned())
}

/// Envía a la papelera de reciclaje (se puede recuperar)
#[tauri::command(async)]
pub fn trash_items(items: Vec<String>) -> Result<(), String> {
    trash::delete_all(&items).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copiar_mover_y_papelera() {
        let base = std::env::temp_dir().join(format!("consola-files-{}", std::process::id()));
        let (a, b) = (base.join("a"), base.join("b"));
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("foto.jpg"), b"x").unwrap();
        fs::write(b.join("foto.jpg"), b"y").unwrap();

        let s = |p: &Path| p.to_string_lossy().into_owned();
        // Copiar sin sobrescribir
        assert_eq!(paste_items(vec![s(&a.join("foto.jpg"))], s(&b), false).unwrap(), 1);
        assert!(b.join("foto (2).jpg").exists());
        assert_eq!(fs::read(b.join("foto.jpg")).unwrap(), b"y");
        // Mover
        paste_items(vec![s(&a.join("foto.jpg"))], s(&b), true).unwrap();
        assert!(!a.join("foto.jpg").exists());
        assert!(b.join("foto (3).jpg").exists());
        // Nueva carpeta y listado (carpetas primero)
        new_folder(s(&b)).unwrap();
        let list = list_dir(s(&b)).unwrap();
        assert!(list[0].is_dir && list[0].name == "Nueva carpeta");
        // Una carpeta no puede ir dentro de sí misma
        assert!(paste_items(vec![s(&b)], s(&b.join("Nueva carpeta")), false).is_err());
        // Papelera
        trash_items(vec![s(&b.join("foto (2).jpg"))]).unwrap();
        assert!(!b.join("foto (2).jpg").exists());

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn lugares_incluyen_disco_c() {
        assert!(list_places().iter().any(|p| p.path == "C:\\"));
    }
}
