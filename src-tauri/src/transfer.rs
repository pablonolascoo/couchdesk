//! Enviar archivos a la consola desde otro dispositivo de casa.
//!
//! - Navegador: el móvil u otro PC abre /subir (por QR), elige carpeta y arrastra archivos.
//!   Cada archivo va en un PUT que se escribe a disco a la vez que llega (sin cargarlo en memoria),
//!   y la página envía varios en paralelo: el límite es la propia red.
//! - FTP: en Recibidos se crean carpetas «PCSX2 - BIOS»… Lo que llega a ellas se coloca solo
//!   en la carpeta real del emulador.
//!
//! Nunca se acepta una ruta libre: solo destinos conocidos y nombres de archivo saneados.

use crate::emulators;
use crate::i18n::{tr, trf};
use crate::receive::received_dir;
use axum::body::Body;
use axum::extract::{ConnectInfo, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

const PAGE: &str = include_str!("../remote/subir.html");

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Dest {
    pub id: String,
    /// Grupo en la página: «Recibidos» o el nombre del emulador
    pub group: String,
    pub label: String,
    #[serde(skip)]
    pub path: PathBuf,
}

/// Recibidos + las carpetas de cada emulador instalado
pub fn destinations() -> Vec<Dest> {
    let mut out = vec![Dest {
        id: "recibidos".into(),
        group: tr("files.received").into(),
        label: tr("files.received").into(),
        path: received_dir(),
    }];
    out.extend(emulators::destinations().into_iter().map(|d| Dest {
        id: d.id,
        group: d.emu.into(),
        label: d.label.into(),
        path: d.path,
    }));
    out
}

/// Nombre de la carpeta FTP de un destino: «PCSX2 - BIOS, firmware y claves»
pub fn ftp_folder_name(d: &Dest) -> String {
    if d.id == "recibidos" {
        return String::new();
    }
    // Nombres fijos (no siguen el idioma): son carpetas reales; si cambiaran, se crearían otras nuevas
    let short = match d.id.rsplit('.').next() {
        Some("juegos") => "Juegos",
        Some("bios") => "BIOS",
        Some("datos") => "Partidas",
        _ => d.label.as_str(),
    };
    clean_name(&format!("{} - {}", d.group, short)).unwrap_or_default()
}

/// Un trozo de nombre seguro para Windows (sin barras, ni «..», ni caracteres prohibidos)
fn clean_name(s: &str) -> Option<String> {
    let t: String = s.chars().filter(|c| !"<>:\"/\\|?*".contains(*c) && !c.is_control()).collect();
    let t = t.trim().trim_end_matches('.').trim().to_string();
    (!t.is_empty() && t != "." && t != "..").then_some(t)
}

/// Ruta relativa segura («Juego/disco1.bin» al subir una carpeta). Nunca sale del destino.
pub fn safe_relative(rel: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in rel.split(['/', '\\']) {
        if part.is_empty() {
            continue;
        }
        out.push(clean_name(part)?);
    }
    (out.components().count() > 0 && out.components().all(|c| matches!(c, Component::Normal(_)))).then_some(out)
}

/// Mueve un archivo sin sobrescribir nada (añade « (2)» si ya existe). Entre discos, copia y borra.
pub fn move_into(from: &Path, to: &Path) -> std::io::Result<PathBuf> {
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let target = free_name(to);
    if std::fs::rename(from, &target).is_err() {
        std::fs::copy(from, &target)?;
        std::fs::remove_file(from)?;
    }
    Ok(target)
}

fn free_name(p: &Path) -> PathBuf {
    if !p.exists() {
        return p.to_path_buf();
    }
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let dir = p.parent().unwrap_or(Path::new(""));
    (2..).map(|n| dir.join(format!("{stem} ({n}){ext}"))).find(|c| !c.exists()).unwrap()
}

// ---------------- FTP: carpetas que se colocan solas ----------------

/// Crea en la carpeta del FTP una subcarpeta por destino
pub fn prepare_ftp_root(root: &Path) -> Vec<(String, PathBuf)> {
    destinations()
        .iter()
        .map(|d| (ftp_folder_name(d), d.path.clone()))
        .filter(|(name, _)| !name.is_empty())
        .inspect(|(name, _)| {
            let _ = std::fs::create_dir_all(root.join(name));
        })
        .collect()
}

/// Archivo recién subido por FTP («/PCSX2 - BIOS/scph.bin»): si va a una carpeta de emulador, se mueve allí
pub fn route_ftp_upload(root: &Path, map: &[(String, PathBuf)], ftp_path: &str) -> Option<PathBuf> {
    let rel = safe_relative(ftp_path)?;
    let mut parts = rel.components();
    let first = parts.next()?.as_os_str().to_string_lossy().into_owned();
    let rest: PathBuf = parts.collect();
    if rest.as_os_str().is_empty() {
        return None; // archivo suelto en la raíz: se queda en Recibidos
    }
    let (_, real) = map.iter().find(|(name, _)| name.eq_ignore_ascii_case(&first))?;
    move_into(&root.join(&rel), &real.join(rest)).ok()
}

// ---------------- Navegador ----------------

#[derive(Deserialize)]
pub struct UploadQuery {
    t: Option<String>,
    /// Destino (id)
    d: String,
    /// Nombre (o ruta relativa si se sube una carpeta)
    n: String,
}

#[derive(Deserialize)]
pub struct TokenQuery {
    t: Option<String>,
}

pub async fn page(State(hub): State<Arc<crate::remote::Hub>>, ConnectInfo(addr): ConnectInfo<SocketAddr>, Query(q): Query<TokenQuery>) -> Response {
    if !hub.allowed(addr, q.t.as_deref()) {
        return (StatusCode::FORBIDDEN, tr("remote.scan_qr")).into_response();
    }
    ([(header::CACHE_CONTROL, "no-store")], Html(PAGE.replace("__TOKEN__", hub.token()))).into_response()
}

pub async fn list(State(hub): State<Arc<crate::remote::Hub>>, ConnectInfo(addr): ConnectInfo<SocketAddr>, Query(q): Query<TokenQuery>) -> Response {
    if !hub.allowed(addr, q.t.as_deref()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let list = tokio::task::spawn_blocking(destinations).await.unwrap_or_default();
    axum::Json(list).into_response()
}

pub async fn upload(
    State(hub): State<Arc<crate::remote::Hub>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(q): Query<UploadQuery>,
    body: Body,
) -> Response {
    if !hub.allowed(addr, q.t.as_deref()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let dests = tokio::task::spawn_blocking(destinations).await.unwrap_or_default();
    let Some(dest) = dests.into_iter().find(|d| d.id == q.d) else {
        return (StatusCode::NOT_FOUND, tr("transfer.unknown_folder")).into_response();
    };
    let Some(rel) = safe_relative(&q.n) else {
        return (StatusCode::BAD_REQUEST, tr("transfer.bad_name")).into_response();
    };
    match save(&dest.path, &rel, body).await {
        Ok(saved) => (StatusCode::OK, saved.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

/// Escribe el archivo mientras llega, en un «.parte» que solo se renombra al terminar entero
pub(crate) async fn save(dir: &Path, rel: &Path, body: Body) -> Result<PathBuf, String> {
    let final_path = dir.join(rel);
    let parent = final_path.parent().ok_or(tr("files.bad_path"))?.to_path_buf();
    tokio::fs::create_dir_all(&parent).await.map_err(|e| trf("transfer.cant_create", &[("err", &e.to_string())]))?;
    let part = parent.join(format!(".{}.{}.parte", rel.file_name().unwrap_or_default().to_string_lossy(), std::process::id()));
    let file = tokio::fs::File::create(&part).await.map_err(|e| e.to_string())?;
    let mut w = tokio::io::BufWriter::with_capacity(4 << 20, file);
    let mut stream = body.into_data_stream();
    let result: Result<(), String> = async {
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| tr("transfer.cut").to_string())?;
            w.write_all(&chunk).await.map_err(|e| e.to_string())?;
        }
        w.flush().await.map_err(|e| e.to_string())
    }
    .await;
    drop(w);
    if let Err(e) = result {
        let _ = tokio::fs::remove_file(&part).await;
        return Err(e);
    }
    tokio::task::spawn_blocking(move || move_into(&part, &final_path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombres_seguros() {
        assert_eq!(safe_relative("scph5501.bin"), Some(PathBuf::from("scph5501.bin")));
        assert_eq!(safe_relative("Juego/disco 1.bin"), Some(PathBuf::from("Juego").join("disco 1.bin")));
        assert_eq!(safe_relative("../../Windows/x.dll"), None);
        assert_eq!(safe_relative("C:\\Windows\\x.dll"), Some(PathBuf::from("C").join("Windows").join("x.dll")));
        assert_eq!(safe_relative(""), None);
        assert_eq!(safe_relative("a/./b"), None);
    }

    #[test]
    fn ftp_coloca_cada_archivo_en_su_carpeta() {
        let base = std::env::temp_dir().join(format!("consola-transfer-{}", std::process::id()));
        let (root, bios) = (base.join("ftp"), base.join("emu").join("bios"));
        let map = vec![("PCSX2 - BIOS".to_string(), bios.clone())];
        std::fs::create_dir_all(root.join("PCSX2 - BIOS")).unwrap();
        std::fs::write(root.join("PCSX2 - BIOS").join("ps2.bin"), b"bios").unwrap();
        std::fs::write(root.join("suelto.txt"), b"x").unwrap();

        let moved = route_ftp_upload(&root, &map, "/PCSX2 - BIOS/ps2.bin").unwrap();
        assert_eq!(moved, bios.join("ps2.bin"));
        assert_eq!(std::fs::read(bios.join("ps2.bin")).unwrap(), b"bios");
        assert!(!root.join("PCSX2 - BIOS").join("ps2.bin").exists());
        // Sin sobrescribir
        std::fs::write(root.join("PCSX2 - BIOS").join("ps2.bin"), b"otra").unwrap();
        assert_eq!(route_ftp_upload(&root, &map, "/PCSX2 - BIOS/ps2.bin").unwrap(), bios.join("ps2 (2).bin"));
        // Lo suelto en la raíz se queda en Recibidos
        assert_eq!(route_ftp_upload(&root, &map, "/suelto.txt"), None);
        assert!(root.join("suelto.txt").exists());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn nombre_de_carpeta_ftp() {
        let d = Dest { id: "pcsx2.bios".into(), group: "PCSX2".into(), label: "BIOS, firmware y claves".into(), path: PathBuf::new() };
        assert_eq!(ftp_folder_name(&d), "PCSX2 - BIOS");
    }
}
