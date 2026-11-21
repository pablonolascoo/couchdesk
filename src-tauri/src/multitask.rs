//! Multitarea (como el centro de control de PS5 o el selector de apps de iOS).
//! El botón Xbox o PS (pulsación corta) o Ctrl + Alt + Inicio, desde cualquier sitio, traen la
//! consola delante con una fila de tarjetas: Inicio y cada ventana abierta (juegos, programas, el
//! navegador y la nube de la consola), con su icono, su memoria y una miniatura.
//! A cambia a ella · X la cierra · Y libera memoria · B (o el botón otra vez) vuelve a lo de antes.
//!
//! El proceso del mando de PlayStation (otro proceso) avisa por un puerto local: "multitask" o "home".

use serde::Serialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::BOOL;

/// Puerto local donde escucha la consola (solo 127.0.0.1)
pub const PORT: u16 = 47802;

/// La pantalla de Multitarea está a la vista
static SHOWING: AtomicBool = AtomicBool::new(false);
/// Ventana que estaba delante al abrirla (0 = la propia consola)
static PREV: AtomicIsize = AtomicIsize::new(0);
static APP: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

const WS_EX_TOOLWINDOW: u32 = 0x80;
const WS_EX_APPWINDOW: u32 = 0x40000;
const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
/// Ventanas del propio Windows que nunca son "apps abiertas"
const SHELL_CLASSES: &[&str] = &["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd", "Windows.UI.Core.CoreWindow", "XamlExplorerHostIslandWindow"];
/// Nunca se fuerzan a cerrar
const PROTECTED: &[&str] = &["explorer.exe", "dwm.exe", "csrss.exe", "winlogon.exe", "lsass.exe", "services.exe", "svchost.exe"];

// ---------------------------------------------------------------- Qué ventanas salen (se prueba)

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    App,
    /// Navegador de la consola
    Navegador,
    /// Xbox Cloud Gaming de la consola
    Nube,
}

/// Lo que dice Windows de una ventana
#[derive(Clone, Debug, Default)]
pub struct RawWindow {
    pub title: String,
    pub class: String,
    pub visible: bool,
    pub cloaked: bool,
    pub has_owner: bool,
    pub ex_style: u32,
    /// Es de la consola: su etiqueta ("main", "navegador", "nube", "asistente"…) o "" si no se sabe
    pub own_label: Option<String>,
}

/// Mismo criterio que Alt + Tab: visible, no oculta por Windows, con título, de primer nivel y no
/// de herramientas. De la consola solo salen su navegador y su nube.
pub fn classify(w: &RawWindow) -> Option<Kind> {
    if !w.visible || w.cloaked || w.title.trim().is_empty() {
        return None;
    }
    let app_window = w.ex_style & WS_EX_APPWINDOW != 0;
    if !app_window && (w.ex_style & (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) != 0 || w.has_owner) {
        return None;
    }
    if SHELL_CLASSES.contains(&w.class.as_str()) {
        return None;
    }
    match w.own_label.as_deref() {
        None => Some(Kind::App),
        Some("navegador") => Some(Kind::Navegador),
        Some("nube") => Some(Kind::Nube),
        Some(_) => None,
    }
}

/// Nombre bonito si el programa no tiene descripción: "notepad++.exe" → "Notepad++"
pub fn name_from_exe(path: &str) -> String {
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path);
    let stem = file.strip_suffix(".exe").or_else(|| file.strip_suffix(".EXE")).unwrap_or(file);
    let mut c = stem.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

// ---------------------------------------------------------------- Tarjetas

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    /// Identificador de la ventana (número)
    pub hwnd: i64,
    pub pid: u32,
    pub kind: Kind,
    pub title: String,
    /// Nombre del programa ("Spotify")
    pub app: String,
    pub exe: String,
    pub icon: Option<String>,
    /// Miniatura PNG (ruta local), si se pudo sacar
    pub thumb: Option<String>,
    /// Memoria en uso (bytes)
    pub memory: u64,
    pub minimized: bool,
    /// Windows dice que no responde
    pub hung: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub cards: Vec<Card>,
    /// Se abrió desde fuera de la consola (B vuelve a esa ventana)
    pub from_outside: bool,
    pub memory: crate::memory::Status,
}

fn text(h: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetWindowTextW(h, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

fn class(h: HWND) -> String {
    let mut buf = [0u16; 128];
    let n = unsafe { GetClassNameW(h, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

fn cloaked(h: HWND) -> bool {
    use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
    let mut v = 0u32;
    unsafe { DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut v as *mut _ as *mut _, 4).is_ok() && v != 0 }
}

fn pid_of(h: HWND) -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
    pid
}

pub fn hwnd(raw: i64) -> HWND {
    HWND(raw as isize as *mut core::ffi::c_void)
}

fn all_windows() -> Vec<HWND> {
    unsafe extern "system" fn each(h: HWND, l: LPARAM) -> BOOL {
        let list = unsafe { &mut *(l.0 as *mut Vec<HWND>) };
        list.push(h);
        BOOL(1)
    }
    let mut list: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut list as *mut _ as isize));
    }
    list
}

/// Las apps de la Tienda viven dentro de ApplicationFrameHost: su proceso de verdad es el de la ventana hija
fn real_pid(h: HWND, pid: u32) -> u32 {
    unsafe extern "system" fn each(c: HWND, l: LPARAM) -> BOOL {
        let data = unsafe { &mut *(l.0 as *mut (u32, u32)) };
        let p = pid_of(c);
        if p != data.0 {
            data.1 = p;
            return BOOL(0);
        }
        BOOL(1)
    }
    let mut data = (pid, 0u32);
    unsafe {
        let _ = EnumChildWindows(Some(h), Some(each), LPARAM(&mut data as *mut _ as isize));
    }
    if data.1 != 0 { data.1 } else { pid }
}

static NAMES: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// Nombre del programa: su descripción (como el Administrador de tareas) o el del archivo
fn friendly_name(path: &str) -> String {
    if let Some(n) = NAMES.lock().unwrap().get_or_insert_with(HashMap::new).get(path) {
        return n.clone();
    }
    let name = description(path).filter(|d| !d.trim().is_empty()).unwrap_or_else(|| name_from_exe(path));
    NAMES.lock().unwrap().get_or_insert_with(HashMap::new).insert(path.to_string(), name.clone());
    name
}

fn description(path: &str) -> Option<String> {
    use windows::Win32::Storage::FileSystem::*;
    use windows::core::HSTRING;
    unsafe {
        let file = HSTRING::from(path);
        let size = GetFileVersionInfoSizeW(&file, None);
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        GetFileVersionInfoW(&file, None, size, data.as_mut_ptr() as *mut _).ok()?;
        let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        let lang = if VerQueryValueW(data.as_ptr() as *const _, &HSTRING::from("\\VarFileInfo\\Translation"), &mut ptr, &mut len).as_bool() && len >= 4 {
            let pair = std::slice::from_raw_parts(ptr as *const u16, 2);
            format!("{:04x}{:04x}", pair[0], pair[1])
        } else {
            "040904b0".into()
        };
        let key = HSTRING::from(format!("\\StringFileInfo\\{lang}\\FileDescription"));
        if !VerQueryValueW(data.as_ptr() as *const _, &key, &mut ptr, &mut len).as_bool() || len == 0 {
            return None;
        }
        let s = std::slice::from_raw_parts(ptr as *const u16, len as usize);
        Some(String::from_utf16_lossy(s).trim_end_matches('\0').trim().to_string())
    }
}

/// Icono ya guardado (sacarlo es lento: lo hace la segunda pasada)
fn cached_icon(exe: &str) -> Option<String> {
    let p = crate::paths::data_dir().join("iconos").join(format!("{}.png", crate::apps::stable_id(exe)));
    p.exists().then(|| p.to_string_lossy().into_owned())
}

/// Etiquetas de las ventanas de la consola
fn own_windows(app: &AppHandle) -> HashMap<isize, String> {
    app.webview_windows()
        .into_iter()
        .filter_map(|(label, w)| w.hwnd().ok().map(|h| (h.0 as isize, label)))
        .collect()
}

/// Lista rápida de ventanas (sin iconos nuevos ni miniaturas)
pub fn enumerate(app: &AppHandle) -> Vec<Card> {
    use crate::memory::win;
    use windows::Win32::System::Threading::{PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ};
    let own = own_windows(app);
    let me = std::process::id();
    let thumbs = THUMBS.lock().unwrap().clone();
    let mut cards = Vec::new();
    for h in all_windows() {
        let pid = pid_of(h);
        let raw = RawWindow {
            title: text(h),
            class: class(h),
            visible: unsafe { IsWindowVisible(h).as_bool() },
            cloaked: cloaked(h),
            has_owner: unsafe { GetWindow(h, GW_OWNER) }.is_ok_and(|o| !o.is_invalid()),
            ex_style: unsafe { GetWindowLongPtrW(h, GWL_EXSTYLE) } as u32,
            own_label: (pid == me).then(|| own.get(&(h.0 as isize)).cloned().unwrap_or_default()),
        };
        let Some(kind) = classify(&raw) else { continue };
        let minimized = unsafe { IsIconic(h).as_bool() };
        let thumb = thumbs.get(&(h.0 as isize)).and_then(|t| t.1.clone());
        if kind != Kind::App {
            let name = if kind == Kind::Navegador { "Navegador" } else { "Xbox Cloud Gaming" };
            cards.push(Card { hwnd: h.0 as i64, pid, kind, title: raw.title, app: name.into(), exe: String::new(), icon: None, thumb, memory: 0, minimized, hung: false });
            continue;
        }
        let pid = if raw.class == "ApplicationFrameWindow" { real_pid(h, pid) } else { pid };
        let proc = win::open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ).or_else(|| win::open(pid, PROCESS_QUERY_LIMITED_INFORMATION));
        let (exe, memory) = proc.map(|p| (win::path(&p), win::working_set(&p))).unwrap_or_default();
        let app_name = if exe.is_empty() { raw.title.clone() } else { friendly_name(&exe) };
        cards.push(Card {
            hwnd: h.0 as i64,
            pid,
            kind,
            icon: cached_icon(&exe),
            app: app_name,
            exe,
            title: raw.title,
            thumb,
            memory,
            minimized,
            hung: unsafe { IsHungAppWindow(h).as_bool() },
        });
    }
    cards
}

// ---------------------------------------------------------------- Miniaturas

/// Miniatura por ventana: (cuándo, ruta)
static THUMBS: std::sync::LazyLock<Mutex<HashMap<isize, (Instant, Option<String>)>>> = std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));
const THUMB_FRESH: Duration = Duration::from_secs(4);
const THUMB_W: i32 = 320;

fn thumbs_dir() -> std::path::PathBuf {
    crate::paths::data_dir().join("iconos").join("multitarea")
}

/// Foto pequeña de una ventana (aunque esté tapada). None si sale negra o falla.
pub fn capture(h: HWND) -> Option<Vec<u8>> {
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
    unsafe {
        if IsIconic(h).as_bool() {
            return None;
        }
        let mut r = RECT::default();
        GetWindowRect(h, &mut r).ok()?;
        let (w, hh) = ((r.right - r.left).min(7680), (r.bottom - r.top).min(4320));
        if w < 16 || hh < 16 {
            return None;
        }
        let (tw, th) = (THUMB_W, (hh * THUMB_W / w).clamp(1, THUMB_W * 2));
        let screen = GetDC(None);
        let big_dc = CreateCompatibleDC(Some(screen));
        let big = CreateCompatibleBitmap(screen, w, hh);
        let old_big = SelectObject(big_dc, big.into());
        // 2 = PW_RENDERFULLCONTENT: también ventanas con DirectX (navegadores, muchos juegos)
        let ok = PrintWindow(h, big_dc, PRINT_WINDOW_FLAGS(2)).as_bool();
        let small_dc = CreateCompatibleDC(Some(screen));
        let small = CreateCompatibleBitmap(screen, tw, th);
        let old_small = SelectObject(small_dc, small.into());
        SetStretchBltMode(small_dc, HALFTONE);
        let _ = SetBrushOrgEx(small_dc, 0, 0, None);
        let _ = StretchBlt(small_dc, 0, 0, tw, th, Some(big_dc), 0, 0, w, hh, SRCCOPY);
        SelectObject(small_dc, old_small);
        SelectObject(big_dc, old_big);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: tw,
                biHeight: -th,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut px = vec![0u8; (tw * th * 4) as usize];
        let lines = GetDIBits(small_dc, small, 0, th as u32, Some(px.as_mut_ptr() as *mut _), &mut info, DIB_RGB_COLORS);
        let _ = DeleteObject(big.into());
        let _ = DeleteObject(small.into());
        let _ = DeleteDC(big_dc);
        let _ = DeleteDC(small_dc);
        ReleaseDC(None, screen);
        if !ok || lines == 0 || looks_blank(&px) {
            return None;
        }
        let mut png_bytes = Vec::new();
        for p in px.chunks_mut(4) {
            p.swap(0, 2);
            p[3] = 255;
        }
        {
            let mut enc = png::Encoder::new(&mut png_bytes, tw as u32, th as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().ok()?.write_image_data(&px).ok()?;
        }
        Some(png_bytes)
    }
}

/// ¿Imagen vacía? (negra o de un solo color: muchos juegos no se dejan fotografiar)
pub fn looks_blank(bgra: &[u8]) -> bool {
    let n = (bgra.len() / 4).max(1);
    let (mut sum, mut min, mut max) = (0u64, 255u8, 0u8);
    for p in bgra.chunks(4) {
        let l = ((u16::from(p[0]) + u16::from(p[1]) * 2 + u16::from(p[2])) / 4) as u8;
        sum += u64::from(l);
        min = min.min(l);
        max = max.max(l);
    }
    sum / n as u64 <= 4 || max.saturating_sub(min) <= 3
}

/// Miniatura en caché unos segundos; si no, se saca otra (con nombre nuevo, así nada se queda viejo)
fn thumb(h: HWND) -> Option<String> {
    let key = h.0 as isize;
    if let Some((at, path)) = THUMBS.lock().unwrap().get(&key) {
        if at.elapsed() < THUMB_FRESH {
            return path.clone();
        }
    }
    let path = capture(h).and_then(|bytes| {
        let dir = thumbs_dir();
        std::fs::create_dir_all(&dir).ok()?;
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        let file = dir.join(format!("{key}-{stamp}.png"));
        std::fs::write(&file, bytes).ok()?;
        Some(file.to_string_lossy().into_owned())
    });
    let old = THUMBS.lock().unwrap().insert(key, (Instant::now(), path.clone()));
    if let Some((_, Some(old))) = old {
        if Some(&old) != path.as_ref() {
            let _ = std::fs::remove_file(old);
        }
    }
    path
}

/// Segunda pasada: iconos que faltan y miniaturas. Avisa a la pantalla con la lista completa.
fn enrich(app: AppHandle, mut cards: Vec<Card>) {
    std::thread::spawn(move || {
        let missing: Vec<Option<String>> = cards.iter().map(|c| (c.icon.is_none() && !c.exe.is_empty()).then(|| c.exe.clone())).collect();
        if missing.iter().any(Option::is_some) {
            let icons = crate::apps::icons::get_many(missing);
            for (c, i) in cards.iter_mut().zip(icons) {
                if c.icon.is_none() {
                    c.icon = i;
                }
            }
            if SHOWING.load(Ordering::Relaxed) {
                let _ = app.emit_to("main", "multitask-cards", &cards);
            }
        }
        for c in cards.iter_mut() {
            c.thumb = thumb(hwnd(c.hwnd));
        }
        // Las de ventanas que ya no existen
        let alive: Vec<isize> = cards.iter().map(|c| c.hwnd as isize).collect();
        THUMBS.lock().unwrap().retain(|k, v| {
            let keep = alive.contains(k);
            if !keep {
                if let Some(p) = &v.1 {
                    let _ = std::fs::remove_file(p);
                }
            }
            keep
        });
        if SHOWING.load(Ordering::Relaxed) {
            let _ = app.emit_to("main", "multitask-cards", &cards);
        }
    });
}

// ---------------------------------------------------------------- Traer ventanas delante

/// Pone una ventana delante aunque otro programa tenga el foco (unirse un instante a su cola de teclado)
pub fn bring_to_front(h: HWND) {
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    unsafe {
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, None);
        let me = GetCurrentThreadId();
        let attached = fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, true).as_bool();
        // El programa podrá traerse delante él solo si lo necesita (algunos juegos lo hacen)
        let _ = AllowSetForegroundWindow(ASFW_ANY);
        let _ = BringWindowToTop(h);
        let _ = SetForegroundWindow(h);
        if attached {
            let _ = AttachThreadInput(me, fg_thread, false);
        }
    }
}

fn main_hwnd(app: &AppHandle) -> Option<HWND> {
    app.get_webview_window("main").and_then(|w| w.hwnd().ok()).map(|h| HWND(h.0))
}

/// Proceso de la ventana que estaba delante al abrir Multitarea (para no tocarlo al liberar memoria)
pub fn previous_pid() -> Option<u32> {
    let p = PREV.load(Ordering::Relaxed);
    (p != 0).then(|| pid_of(HWND(p as *mut _)))
}

pub fn snapshot(app: &AppHandle) -> Snapshot {
    Snapshot { cards: enumerate(app), from_outside: PREV.load(Ordering::Relaxed) != 0, memory: crate::memory::status() }
}

/// Abre Multitarea desde donde sea
pub fn open(app: &AppHandle) {
    let t0 = Instant::now();
    let Some(main) = app.get_webview_window("main") else { return };
    let main_h = main_hwnd(app);
    let fg = unsafe { GetForegroundWindow() };
    // Lo de delante, si no es la propia consola (ni su asistente flotante)
    let own_pid = std::process::id();
    let outside = !fg.is_invalid() && Some(fg) != main_h && (pid_of(fg) != own_pid || own_windows(app).get(&(fg.0 as isize)).is_some_and(|l| l == "navegador" || l == "nube"));
    PREV.store(if outside { fg.0 as isize } else { 0 }, Ordering::Relaxed);
    let snap = snapshot(app);
    SHOWING.store(true, Ordering::Relaxed);
    crate::memory::wake(app);
    let _ = main.emit("multitask-open", &snap);
    let _ = main.unminimize();
    let _ = main.show();
    if let Some(h) = main_h {
        bring_to_front(h);
    }
    let _ = main.set_focus();
    log::info!("Multitarea: {} ventanas en {} ms", snap.cards.len(), t0.elapsed().as_millis());
    enrich(app.clone(), snap.cards);
}

/// Botón Xbox / PS (corto) o Ctrl + Alt + Inicio: abre Multitarea, o vuelve si ya estaba abierta
pub fn toggle(app: &AppHandle) {
    let main_front = main_hwnd(app).is_some_and(|h| h == unsafe { GetForegroundWindow() });
    if SHOWING.load(Ordering::Relaxed) && main_front {
        // La pantalla sabe volver (y deja la consola donde estaba)
        let _ = app.emit_to("main", "multitask-back", ());
    } else {
        open(app);
    }
}

/// Desde cualquier hilo (gancho del teclado, botón Xbox…)
pub fn toggle_global() {
    if let Some(app) = APP.get() {
        let app = app.clone();
        std::thread::spawn(move || toggle(&app));
    }
}

/// Escucha al proceso del mando de PlayStation ("multitask" / "home")
pub fn start(app: AppHandle) {
    let _ = APP.set(app.clone());
    // Nombres de los programas abiertos en caché desde el principio: la primera apertura también es rápida
    let warm = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(4));
        enumerate(&warm);
    });
    std::thread::spawn(move || {
        let Ok(listener) = std::net::TcpListener::bind(("127.0.0.1", PORT)) else {
            log::warn!("Multitarea: el puerto {PORT} está ocupado");
            return;
        };
        for conn in listener.incoming().flatten() {
            let _ = conn.set_read_timeout(Some(Duration::from_millis(500)));
            let mut line = String::new();
            if BufReader::new(conn).read_line(&mut line).is_err() {
                continue;
            }
            match line.trim() {
                "multitask" => toggle_global(),
                "home" => {
                    let app = app.clone();
                    std::thread::spawn(move || crate::browser::go_home(&app));
                }
                _ => {}
            }
        }
    });
}

/// Para el proceso del mando: pide algo a la consola. false = la consola no está abierta.
pub fn send(what: &str) -> bool {
    use std::io::Write;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], PORT));
    match std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(300)) {
        Ok(mut s) => s.write_all(format!("{what}\n").as_bytes()).is_ok(),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------- Órdenes

/// Error con clave de texto: la pantalla lo traduce (multitask.json)
fn err(key: &str) -> String {
    key.to_string()
}

#[tauri::command(async)]
pub fn multitask_list(app: AppHandle) -> Snapshot {
    let snap = snapshot(&app);
    enrich(app, snap.cards.clone());
    snap
}

/// La pantalla está (o deja de estar) a la vista
#[tauri::command(async)]
pub fn multitask_visible(on: bool) {
    SHOWING.store(on, Ordering::Relaxed);
}

/// A: cambiar a esa ventana
#[tauri::command(async)]
pub fn multitask_switch(app: AppHandle, hwnd_id: i64) -> Result<(), String> {
    let h = hwnd(hwnd_id);
    if !unsafe { IsWindow(Some(h)).as_bool() } {
        return Err(err("multitask.gone"));
    }
    SHOWING.store(false, Ordering::Relaxed);
    PREV.store(0, Ordering::Relaxed);
    bring_to_front(h);
    crate::memory::poke();
    let _ = app;
    Ok(())
}

/// B: volver a lo que estaba delante. true si era otra ventana (un juego, un programa…)
#[tauri::command(async)]
pub fn multitask_back() -> bool {
    SHOWING.store(false, Ordering::Relaxed);
    let p = PREV.swap(0, Ordering::Relaxed);
    let h = HWND(p as *mut _);
    if p == 0 || !unsafe { IsWindow(Some(h)).as_bool() } {
        return false;
    }
    bring_to_front(h);
    crate::memory::poke();
    true
}

/// X: pedir con educación que se cierre (como pulsar la ✕ de la ventana)
#[tauri::command(async)]
pub fn multitask_close(app: AppHandle, hwnd_id: i64) -> Result<(), String> {
    let h = hwnd(hwnd_id);
    if let Some((_, w)) = app.webview_windows().into_iter().find(|(l, w)| (l == "navegador" || l == "nube") && w.hwnd().is_ok_and(|x| x.0 == h.0)) {
        return w.close().map_err(|e| e.to_string());
    }
    if pid_of(h) == std::process::id() {
        return Err(err("multitask.cant_close"));
    }
    unsafe { PostMessageW(Some(h), WM_CLOSE, Default::default(), Default::default()) }.map_err(|e| e.to_string())
}

/// ¿Sigue abierta esa ventana?
#[tauri::command(async)]
pub fn multitask_alive(hwnd_id: i64) -> bool {
    let h = hwnd(hwnd_id);
    unsafe { IsWindow(Some(h)).as_bool() && IsWindowVisible(h).as_bool() }
}

/// Forzar cierre (tras confirmarlo): termina el programa de esa ventana
#[tauri::command(async)]
pub fn multitask_force_close(hwnd_id: i64) -> Result<(), String> {
    use crate::memory::win;
    use windows::Win32::System::Threading::{PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, TerminateProcess};
    let h = hwnd(hwnd_id);
    let pid = pid_of(h);
    let pid = if class(h) == "ApplicationFrameWindow" { real_pid(h, pid) } else { pid };
    if pid == 0 || pid == std::process::id() {
        return Err(err("multitask.cant_close"));
    }
    let p = win::open(pid, PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION).ok_or_else(|| err("multitask.no_permission"))?;
    let exe = win::path(&p).to_lowercase();
    if PROTECTED.iter().any(|x| exe.ends_with(&format!("\\{x}"))) {
        return Err(err("multitask.cant_close"));
    }
    unsafe { TerminateProcess(p.0, 1) }.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(title: &str) -> RawWindow {
        RawWindow { title: title.into(), class: "Chrome_WidgetWin_1".into(), visible: true, ..Default::default() }
    }

    #[test]
    fn que_ventanas_salen() {
        assert_eq!(classify(&w("Spotify Premium")), Some(Kind::App));
        assert_eq!(classify(&RawWindow { visible: false, ..w("x") }), None, "invisible");
        assert_eq!(classify(&RawWindow { cloaked: true, ..w("Calculadora") }), None, "oculta por Windows (otro escritorio, app suspendida)");
        assert_eq!(classify(&w("   ")), None, "sin título");
        assert_eq!(classify(&RawWindow { ex_style: WS_EX_TOOLWINDOW, ..w("Paleta") }), None, "de herramientas");
        assert_eq!(classify(&RawWindow { ex_style: WS_EX_NOACTIVATE, ..w("Aviso") }), None);
        assert_eq!(classify(&RawWindow { has_owner: true, ..w("Guardar como") }), None, "diálogo con dueño");
        assert_eq!(classify(&RawWindow { has_owner: true, ex_style: WS_EX_APPWINDOW, ..w("Juego") }), Some(Kind::App), "APPWINDOW manda");
        assert_eq!(classify(&RawWindow { class: "Shell_TrayWnd".into(), ..w("Barra") }), None);
        assert_eq!(classify(&RawWindow { class: "Progman".into(), ..w("Program Manager") }), None);
        // Las de la consola
        assert_eq!(classify(&RawWindow { own_label: Some("navegador".into()), ..w("Navegador") }), Some(Kind::Navegador));
        assert_eq!(classify(&RawWindow { own_label: Some("nube".into()), ..w("Xbox Cloud Gaming") }), Some(Kind::Nube));
        assert_eq!(classify(&RawWindow { own_label: Some("main".into()), ..w("CouchDesk") }), None);
        assert_eq!(classify(&RawWindow { own_label: Some("asistente".into()), ..w("Claude") }), None);
        assert_eq!(classify(&RawWindow { own_label: Some(String::new()), ..w("ventana interna") }), None);
    }

    #[test]
    fn nombres_y_miniaturas_vacias() {
        assert_eq!(name_from_exe("C:\\Program Files\\Notepad++\\notepad++.exe"), "Notepad++");
        assert_eq!(name_from_exe("D:\\Juegos\\hades2.EXE"), "Hades2");
        assert_eq!(name_from_exe(""), "");
        assert!(looks_blank(&[0, 0, 0, 255].repeat(100)), "negra");
        assert!(looks_blank(&[200, 200, 200, 255].repeat(100)), "de un color");
        let mut img = [10, 10, 10, 255].repeat(50);
        img.extend([240, 230, 220, 255].repeat(50));
        assert!(!looks_blank(&img));
    }

    /// En este PC: enseña las ventanas que saldrían en Multitarea (sin la consola abierta)
    #[test]
    #[ignore]
    fn ventanas_reales() {
        use crate::memory::win;
        use windows::Win32::System::Threading::{PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ};
        for pass in 1..=2 {
        let t0 = Instant::now();
        let mut n = 0;
        for h in all_windows() {
            let raw = RawWindow {
                title: text(h),
                class: class(h),
                visible: unsafe { IsWindowVisible(h).as_bool() },
                cloaked: cloaked(h),
                has_owner: unsafe { GetWindow(h, GW_OWNER) }.is_ok_and(|o| !o.is_invalid()),
                ex_style: unsafe { GetWindowLongPtrW(h, GWL_EXSTYLE) } as u32,
                own_label: None,
            };
            if classify(&raw).is_none() {
                continue;
            }
            n += 1;
            let pid = pid_of(h);
            let pid = if raw.class == "ApplicationFrameWindow" { real_pid(h, pid) } else { pid };
            let p = win::open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ).or_else(|| win::open(pid, PROCESS_QUERY_LIMITED_INFORMATION));
            let (exe, mem) = p.map(|p| (win::path(&p), win::working_set(&p))).unwrap_or_default();
            println!("{:>10}  {:<28} {:>7} MB  {:<40} [{}]", h.0 as isize, friendly_name(&exe), mem / 1024 / 1024, raw.title.chars().take(40).collect::<String>(), raw.class);
        }
        println!("pasada {pass}: {n} ventanas en {} ms (sin miniaturas; la 2.ª con los nombres ya en caché)", t0.elapsed().as_millis());
        }
        let t1 = Instant::now();
        let mut ok = 0;
        for h in all_windows() {
            let raw = RawWindow { title: text(h), class: class(h), visible: unsafe { IsWindowVisible(h).as_bool() }, cloaked: cloaked(h), ..Default::default() };
            if classify(&raw).is_some() && unsafe { GetWindow(h, GW_OWNER) }.is_err() {
                if let Some(png) = capture(h) {
                    ok += 1;
                    println!("miniatura {}: {} KB", raw.title.chars().take(30).collect::<String>(), png.len() / 1024);
                }
            }
        }
        println!("{ok} miniaturas en {} ms", t1.elapsed().as_millis());
    }
}
