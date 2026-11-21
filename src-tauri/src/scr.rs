//! CouchDesk como salvapantallas de Windows y otros ajustes "de Windows".
//!
//! Windows arranca los salvapantallas así: `programa.scr /s` (enseñar), `/c` (configurar),
//! `/p <hwnd>` (miniatura en el panel de Windows). Copiamos `consola.exe` como
//! `%LOCALAPPDATA%\Consola\CouchDesk.scr` y lo apuntamos en `HKCU\Control Panel\Desktop`:
//! `SCRNSAVE.EXE`, `ScreenSaveActive=1` y `ScreenSaveTimeOut` (segundos). Los valores de antes se
//! guardan en `salvapantallas-windows.json` y se devuelven al desactivarlo.
//! Si CouchDesk ya está abierta, el `/s` le llega por "una sola instancia" (lib.rs) y enseña su
//! salvapantallas; si no, arranca en modo mínimo: solo el salvapantallas y se cierra al despertar.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

// ---------------- Cómo se arrancó ----------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode {
    Normal,
    /// `/s`: enseñar el salvapantallas
    Saver,
    /// `/c`: abrir sus ajustes
    Config,
    /// `/p <hwnd>`: miniatura del panel de Windows (no la hacemos: salir)
    Preview,
}

/// Lee los argumentos de Windows (`/s`, `/S`, `-s`, `/c:1234`, `/p 1234`). El primero es el programa.
pub fn parse_args(args: &[String]) -> LaunchMode {
    let Some(first) = args.iter().skip(1).find(|a| !a.trim().is_empty()) else { return LaunchMode::Normal };
    let a = first.trim().to_lowercase();
    if !(a.starts_with('/') || a.starts_with('-')) {
        return LaunchMode::Normal;
    }
    match a.chars().nth(1) {
        Some('s') if a.len() == 2 => LaunchMode::Saver,
        Some('c') if a.len() == 2 || a[2..].starts_with(':') => LaunchMode::Config,
        Some('p') if a.len() == 2 || a[2..].starts_with(':') => LaunchMode::Preview,
        _ => LaunchMode::Normal,
    }
}

static SAVER_ONLY: AtomicBool = AtomicBool::new(false);
static LAUNCH_ROUTE: Mutex<Option<String>> = Mutex::new(None);

pub fn set_saver_only(on: bool) {
    SAVER_ONLY.store(on, Ordering::Relaxed);
}
/// Arrancada solo para el salvapantallas de Windows: al despertar, se cierra
pub fn saver_only() -> bool {
    SAVER_ONLY.load(Ordering::Relaxed)
}

pub const SAVER_ROUTE: &str = "/ajustes/salvapantallas";

pub fn set_launch_route(route: &str) {
    *LAUNCH_ROUTE.lock().unwrap() = Some(route.to_string());
}

/// La pantalla con la que se pidió abrir la consola (una sola vez)
#[tauri::command(async)]
pub fn launch_route_take() -> Option<String> {
    LAUNCH_ROUTE.lock().unwrap().take()
}

// ---------------- Salvapantallas de Windows ----------------

fn desktop_key() -> String {
    // Las pruebas usan otra clave para no tocar el Windows real
    std::env::var("CONSOLA_DESKTOP_KEY").unwrap_or_else(|_| "Control Panel\\Desktop".into())
}

fn testing() -> bool {
    std::env::var("CONSOLA_DESKTOP_KEY").is_ok()
}

fn data_dir() -> PathBuf {
    std::env::var("CONSOLA_SCR_DIR").map(PathBuf::from).unwrap_or_else(|_| {
        crate::paths::data_dir()
    })
}

pub fn scr_path() -> PathBuf {
    data_dir().join("CouchDesk.scr")
}

fn backup_path() -> PathBuf {
    data_dir().join("salvapantallas-windows.json")
}

/// Cómo estaba el salvapantallas de Windows antes de usar CouchDesk
#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
pub struct Backup {
    scrnsave: Option<String>,
    active: Option<String>,
    timeout: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowsSaver {
    enabled: bool,
    path: String,
}

fn read_values() -> Backup {
    let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(desktop_key(), KEY_READ) else { return Backup::default() };
    Backup {
        scrnsave: k.get_value("SCRNSAVE.EXE").ok(),
        active: k.get_value("ScreenSaveActive").ok(),
        timeout: k.get_value("ScreenSaveTimeOut").ok(),
    }
}

fn write_values(b: &Backup) -> Result<(), String> {
    let (k, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(desktop_key()).map_err(|e| e.to_string())?;
    for (name, value) in [("SCRNSAVE.EXE", &b.scrnsave), ("ScreenSaveActive", &b.active), ("ScreenSaveTimeOut", &b.timeout)] {
        match value {
            Some(v) => k.set_value(name, v).map_err(|e| e.to_string())?,
            None => match k.delete_value(name) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            },
        }
    }
    Ok(())
}

/// ¿El valor de Windows apunta a nuestro .scr?
pub fn is_ours(scrnsave: Option<&str>, ours: &Path) -> bool {
    scrnsave.is_some_and(|s| same_path(s, &ours.display().to_string()))
}

/// ¿Apunta al .scr de la antigua «Consola»?
pub fn is_legacy_scr(scrnsave: Option<&str>) -> bool {
    scrnsave.is_some_and(|s| s.trim().trim_matches('"').replace('/', "\\").to_lowercase().ends_with("\\consola\\couchdesk.scr"))
}

fn same_path(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.trim().trim_matches('"').replace('/', "\\").to_lowercase();
    norm(a) == norm(b)
}

/// Valores para Windows con CouchDesk: segundos de espera (mínimo un minuto)
pub fn saver_values(scr: &Path, after_min: u32) -> Backup {
    Backup {
        scrnsave: Some(scr.display().to_string()),
        active: Some("1".into()),
        timeout: Some((after_min.max(1) * 60).to_string()),
    }
}

/// Avisa a Windows del cambio sin cerrar sesión
fn notify_windows(active: bool, timeout_s: u32) {
    if testing() {
        return;
    }
    use windows::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        let flags = SPIF_UPDATEINIFILE | SPIF_SENDCHANGE;
        let _ = SystemParametersInfoW(SPI_SETSCREENSAVEACTIVE, u32::from(active), None, flags);
        if active {
            let _ = SystemParametersInfoW(SPI_SETSCREENSAVETIMEOUT, timeout_s, None, flags);
        }
    }
}

fn copy_exe(to: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    if same_path(&exe.display().to_string(), &to.display().to_string()) {
        return Ok(()); // ya somos el .scr
    }
    std::fs::copy(&exe, to).map(|_| ()).map_err(|e| format!("No se pudo copiar el salvapantallas: {e}"))
}

pub fn set_windows_saver(enabled: bool, after_min: u32) -> Result<WindowsSaver, String> {
    let scr = scr_path();
    let now = read_values();
    if enabled {
        // Guardar lo de antes solo la primera vez (si no es ya nuestro)
        if !is_ours(now.scrnsave.as_deref(), &scr) {
            let text = serde_json::to_string_pretty(&now).map_err(|e| e.to_string())?;
            std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
            std::fs::write(backup_path(), text).map_err(|e| e.to_string())?;
        }
        copy_exe(&scr)?;
        write_values(&saver_values(&scr, after_min))?;
        notify_windows(true, after_min.max(1) * 60);
    } else if is_ours(now.scrnsave.as_deref(), &scr) {
        let before: Backup = std::fs::read_to_string(backup_path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        write_values(&before)?;
        let active = before.active.as_deref() == Some("1") && before.scrnsave.is_some();
        notify_windows(active, before.timeout.as_deref().and_then(|t| t.parse().ok()).unwrap_or(600));
        let _ = std::fs::remove_file(backup_path());
    }
    Ok(windows_saver_status())
}

#[tauri::command(async)]
pub fn windows_saver_status() -> WindowsSaver {
    let scr = scr_path();
    let v = read_values();
    WindowsSaver { enabled: is_ours(v.scrnsave.as_deref(), &scr) && v.active.as_deref() != Some("0"), path: scr.display().to_string() }
}

#[tauri::command(async)]
pub fn windows_saver_set(enabled: bool, after_min: u32) -> Result<WindowsSaver, String> {
    set_windows_saver(enabled, after_min)
}

/// Tras una actualización, el .scr se queda con la versión vieja: se renueva al arrancar
pub fn refresh_copy() {
    let scr = scr_path();
    let values = read_values();
    let Ok(exe) = std::env::current_exe() else { return };
    let name = exe.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !(name == "couchdesk.exe" || name == "consola.exe") || exe.display().to_string().to_lowercase().contains("\\target\\") {
        return; // solo la consola instalada
    }
    // Hasta la 1.1.0 el .scr vivía en %LOCALAPPDATA%\Consola: se pasa a la carpeta nueva
    if is_legacy_scr(values.scrnsave.as_deref()) {
        if copy_exe(&scr).is_ok() {
            let _ = write_values(&Backup { scrnsave: Some(scr.display().to_string()), ..values });
        }
        return;
    }
    if !is_ours(values.scrnsave.as_deref(), &scr) {
        return;
    }
    let meta = |p: &Path| std::fs::metadata(p).ok().map(|m| (m.len(), m.modified().ok()));
    let (a, b) = (meta(&exe), meta(&scr));
    let newer = match (&a, &b) {
        (Some((la, ma)), Some((lb, mb))) => la != lb || ma > mb,
        (Some(_), None) => true,
        _ => false,
    };
    if newer {
        let _ = copy_exe(&scr);
    }
}

// ---------------- Ajustes de Windows ----------------

/// Abre una página de Configuración de Windows (solo las de esta lista)
#[tauri::command(async)]
pub fn open_windows_settings(page: String) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const ALLOWED: [&str; 3] = ["signinoptions", "lockscreen", "personalization"];
    if !ALLOWED.contains(&page.as_str()) {
        return Err("Página no permitida".into());
    }
    std::process::Command::new("cmd")
        .raw_arg(format!("/C start \"\" ms-settings:{page}"))
        .creation_flags(0x0800_0000)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

const OPTIMIZE: &str = include_str!("../../scripts/optimizar-consola.cmd");
const UNDO: &str = include_str!("../../scripts/deshacer-optimizacion.cmd");

/// Lanza el optimizador como administrador: Windows pide permiso (el usuario decide)
#[tauri::command(async)]
pub fn run_optimizer(undo: bool) -> Result<(), String> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{HSTRING, w};
    let dir = data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let (name, text) = if undo { ("deshacer-optimizacion.cmd", UNDO) } else { ("optimizar-consola.cmd", OPTIMIZE) };
    let path = dir.join(name);
    // cmd.exe lee los .cmd en la página de códigos del sistema; los textos son ASCII
    std::fs::write(&path, text.replace("\r\n", "\n").replace('\n', "\r\n")).map_err(|e| e.to_string())?;
    let file = HSTRING::from(path.as_os_str());
    let r = unsafe { ShellExecuteW(None, w!("runas"), &file, None, None, SW_SHOWNORMAL) };
    if r.0 as isize <= 32 {
        return Err("Windows no dejó abrir el optimizador (¿se canceló el aviso?).".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(xs: &[&str]) -> Vec<String> {
        std::iter::once("consola.exe").chain(xs.iter().copied()).map(String::from).collect()
    }

    #[test]
    fn lee_los_argumentos_de_windows() {
        assert_eq!(parse_args(&args(&[])), LaunchMode::Normal);
        assert_eq!(parse_args(&args(&["/s"])), LaunchMode::Saver);
        assert_eq!(parse_args(&args(&["/S"])), LaunchMode::Saver);
        assert_eq!(parse_args(&args(&["-s"])), LaunchMode::Saver);
        assert_eq!(parse_args(&args(&["/c"])), LaunchMode::Config);
        assert_eq!(parse_args(&args(&["/c:1234"])), LaunchMode::Config);
        assert_eq!(parse_args(&args(&["/p", "1234"])), LaunchMode::Preview);
        assert_eq!(parse_args(&args(&["/P:5678"])), LaunchMode::Preview);
        // Otros argumentos de la consola no se confunden
        assert_eq!(parse_args(&args(&["--mcp"])), LaunchMode::Normal);
        assert_eq!(parse_args(&args(&["--restaurar-escritorio"])), LaunchMode::Normal);
        assert_eq!(parse_args(&args(&["/start"])), LaunchMode::Normal);
    }

    #[test]
    fn valores_para_windows() {
        let scr = Path::new("C:\\Users\\Ana\\AppData\\Local\\Consola\\CouchDesk.scr");
        let v = saver_values(scr, 5);
        assert_eq!(v.timeout.as_deref(), Some("300"));
        assert_eq!(v.active.as_deref(), Some("1"));
        assert_eq!(saver_values(scr, 0).timeout.as_deref(), Some("60"), "mínimo un minuto");
        assert!(is_ours(Some("c:\\users\\ana\\appdata\\local\\consola\\couchdesk.scr"), scr));
        assert!(is_ours(Some("\"C:\\Users\\Ana\\AppData\\Local\\Consola\\CouchDesk.scr\""), scr));
        assert!(!is_ours(Some("C:\\Windows\\system32\\Mystify.scr"), scr));
        assert!(!is_ours(None, scr));
        assert!(is_legacy_scr(Some("C:\\Users\\Ana\\AppData\\Local\\Consola\\CouchDesk.scr")));
        assert!(!is_legacy_scr(Some("C:\\Users\\Ana\\AppData\\Local\\CouchDesk\\CouchDesk.scr")));
    }

    #[test]
    fn activar_y_devolver_lo_de_antes() {
        let dir = std::env::temp_dir().join(format!("couchdesk-scr-{}", std::process::id()));
        unsafe {
            std::env::set_var("CONSOLA_DESKTOP_KEY", "Software\\ConsolaPruebas\\Desktop");
            std::env::set_var("CONSOLA_SCR_DIR", &dir);
        }
        // Antes: un salvapantallas de Windows con 10 minutos
        let before = Backup { scrnsave: Some("C:\\Windows\\system32\\Ribbons.scr".into()), active: Some("1".into()), timeout: Some("600".into()) };
        write_values(&before).unwrap();

        let on = set_windows_saver(true, 5).unwrap();
        assert!(on.enabled);
        assert_eq!(read_values().timeout.as_deref(), Some("300"));
        assert!(scr_path().exists());
        // Cambiar el tiempo no pisa la copia de lo de antes
        set_windows_saver(true, 15).unwrap();
        assert_eq!(read_values().timeout.as_deref(), Some("900"));

        let off = set_windows_saver(false, 5).unwrap();
        assert!(!off.enabled);
        assert_eq!(read_values(), before);

        let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all("Software\\ConsolaPruebas");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
