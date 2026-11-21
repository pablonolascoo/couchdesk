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
    /// Archivos grandes por trozos: id de la subida, posición del trozo, tamaño total y «ya está»
    u: Option<String>,
    o: Option<u64>,
    s: Option<u64>,
    fin: Option<u8>,
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
    let name = |p: PathBuf| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let result = match (q.u.as_deref(), q.s) {
        // Trozo de un archivo grande, o el aviso de que ya han llegado todos
        (Some(id), Some(size)) if !id.is_empty() && id.len() <= 32 && id.chars().all(|c| c.is_ascii_alphanumeric()) => {
            if q.fin.is_some() {
                finish_chunks(&dest.path, &rel, id, size).await.map(name)
            } else {
                save_chunk(&dest.path, &rel, id, q.o.unwrap_or(0), size, body).await.map(|_| String::new())
            }
        }
        (Some(_), _) => return (StatusCode::BAD_REQUEST, tr("transfer.bad_name")).into_response(),
        _ => save(&dest.path, &rel, body).await.map(name),
    };
    match result {
        Ok(text) => (StatusCode::OK, text).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

/// Archivo a medio llegar: oculto junto al destino, con el nombre final y un id
fn part_path(dir: &Path, rel: &Path, id: &str) -> Result<(PathBuf, PathBuf), String> {
    let final_path = dir.join(rel);
    let parent = final_path.parent().ok_or(tr("files.bad_path"))?;
    let part = parent.join(format!(".{}.{id}.parte", rel.file_name().unwrap_or_default().to_string_lossy()));
    Ok((final_path, part))
}

/// Vuelca la red al disco a toda velocidad. La red entrega trozos pequeños; un hilo aparte los
/// junta en escrituras grandes, así recibir y escribir van a la vez y nunca se esperan.
async fn write_stream(part: PathBuf, offset: u64, size: Option<u64>, body: Body) -> Result<(), String> {
    use std::io::{Seek, SeekFrom, Write};
    let (tx, mut rx) = tokio::sync::mpsc::channel::<axum::body::Bytes>(512);
    let writer = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        std::fs::create_dir_all(part.parent().unwrap_or(Path::new(".")))?;
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(false).open(&part)?;
        // Reservar el tamaño final de una vez: el disco no se fragmenta y no crece a saltos
        if let Some(s) = size {
            if f.metadata()?.len() < s {
                f.set_len(s)?;
            }
        }
        f.seek(SeekFrom::Start(offset))?;
        let mut w = std::io::BufWriter::with_capacity(8 << 20, f);
        while let Some(b) = rx.blocking_recv() {
            w.write_all(&b)?;
        }
        w.flush()
    });
    let mut stream = body.into_data_stream();
    let mut cut = false;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => {
                if tx.send(c).await.is_err() {
                    break; // el disco falló: el error sale abajo
                }
            }
            Err(_) => {
                cut = true;
                break;
            }
        }
    }
    drop(tx);
    let written = writer.await.map_err(|e| e.to_string())?;
    if cut {
        return Err(tr("transfer.cut").into());
    }
    written.map_err(|e| trf("transfer.cant_create", &[("err", &e.to_string())]))
}

async fn finish(part: PathBuf, final_path: PathBuf) -> Result<PathBuf, String> {
    tokio::task::spawn_blocking(move || move_into(&part, &final_path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Escribe el archivo mientras llega, en un «.parte» que solo se renombra al terminar entero
pub(crate) async fn save(dir: &Path, rel: &Path, body: Body) -> Result<PathBuf, String> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = format!("{}-{}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let (final_path, part) = part_path(dir, rel, &id)?;
    if let Err(e) = write_stream(part.clone(), 0, None, body).await {
        let _ = tokio::fs::remove_file(&part).await;
        return Err(e);
    }
    finish(part, final_path).await
}

/// Un trozo de un archivo grande. Los archivos grandes llegan en trozos por varias conexiones
/// a la vez (como los gestores de descargas): con wifi, varias conexiones suman mucha más
/// velocidad que una sola. Todos escriben en su sitio del mismo «.parte».
async fn save_chunk(dir: &Path, rel: &Path, id: &str, offset: u64, size: u64, body: Body) -> Result<(), String> {
    if offset >= size.max(1) {
        return Err(tr("transfer.bad_name").into());
    }
    let (_, part) = part_path(dir, rel, id)?;
    write_stream(part, offset, Some(size), body).await
}

/// Todos los trozos han llegado: el archivo aparece con su nombre
async fn finish_chunks(dir: &Path, rel: &Path, id: &str, size: u64) -> Result<PathBuf, String> {
    let (final_path, part) = part_path(dir, rel, id)?;
    let len = tokio::fs::metadata(&part).await.map(|m| m.len()).map_err(|_| tr("transfer.cut").to_string())?;
    if len != size {
        let _ = tokio::fs::remove_file(&part).await;
        return Err(tr("transfer.cut").into());
    }
    finish(part, final_path).await
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

    /// Un archivo grande en trozos por 6 conexiones a la vez, llegando desordenados
    #[test]
    fn subir_por_trozos_en_paralelo() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let hub = crate::remote::Hub::new(None);
        let token = hub.token().to_string();
        rt.spawn(crate::remote::serve(hub, 7792));
        std::thread::sleep(std::time::Duration::from_millis(500));

        const CHUNK: usize = 16 << 20;
        let size = 400 * (1 << 20) + 12345; // el último trozo es incompleto
        let data: Arc<Vec<u8>> = Arc::new((0..size).map(|i| (i % 251) as u8).collect());
        let name = format!("prueba-trozos-{}.bin", std::process::id());
        let base = format!("http://127.0.0.1:7792/api/subir?t={token}&d=recibidos&n={name}&u=abc123&s={size}");
        // Del último al primero, repartidos entre 6 hilos
        let offsets: Vec<usize> = (0..size).step_by(CHUNK).rev().collect();
        let next = Arc::new(std::sync::Mutex::new(offsets));
        let start = std::time::Instant::now();
        let workers: Vec<_> = (0..6)
            .map(|_| {
                let (next, data, base) = (next.clone(), data.clone(), base.clone());
                std::thread::spawn(move || {
                    let http = reqwest::blocking::Client::new();
                    while let Some(o) = { let o = next.lock().unwrap().pop(); o } {
                        let part = data[o..(o + CHUNK).min(data.len())].to_vec();
                        let r = http.put(format!("{base}&o={o}")).body(part).send().unwrap();
                        assert_eq!(r.status(), 200);
                    }
                })
            })
            .collect();
        workers.into_iter().for_each(|w| w.join().unwrap());
        let r = reqwest::blocking::Client::new().put(format!("{base}&fin=1")).send().unwrap();
        let secs = start.elapsed().as_secs_f64();
        assert_eq!(r.status(), 200);
        assert_eq!(r.text().unwrap(), name);
        println!("Velocidad por trozos en local: {:.0} MB/s", size as f64 / secs / 1e6);

        let saved = received_dir().join(&name);
        assert!(std::fs::read(&saved).unwrap() == *data, "el archivo no llegó igual");
        // Un id raro se rechaza
        let r = reqwest::blocking::Client::new().put(base.replace("u=abc123", "u=..%2Fx") + "&o=0").body("x").send().unwrap();
        assert_eq!(r.status(), 400);
        // Si falta algo, «ya está» falla y no deja el archivo a medias
        let r = reqwest::blocking::Client::new().put(format!("{}&o=0", base.replace("abc123", "zzz"))).body(vec![0u8; 10]).send().unwrap();
        assert_eq!(r.status(), 200);
        let r = reqwest::blocking::Client::new().put(format!("{}&fin=1", base.replace("abc123", "zzz").replace(&format!("s={size}"), "s=99"))).send().unwrap();
        assert_ne!(r.status(), 200);
        let _ = std::fs::remove_file(saved);
        for e in std::fs::read_dir(received_dir()).unwrap().flatten() {
            if e.file_name().to_string_lossy().contains(&name) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }

    /// Servidor de prueba para revisar la página a mano: cargo test servidor_de_prueba -- --ignored
    #[test]
    #[ignore]
    fn servidor_de_prueba() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let hub = crate::remote::Hub::new(None);
        println!("http://127.0.0.1:7793/subir?t={}", hub.token());
        rt.block_on(async {
            tokio::select! {
                _ = crate::remote::serve(hub, 7793) => {}
                _ = tokio::time::sleep(std::time::Duration::from_secs(300)) => {}
            }
        });
    }

    #[test]
    fn nombre_de_carpeta_ftp() {
        let d = Dest { id: "pcsx2.bios".into(), group: "PCSX2".into(), label: "BIOS, firmware y claves".into(), path: PathBuf::new() };
        assert_eq!(ftp_folder_name(&d), "PCSX2 - BIOS");
    }
}
