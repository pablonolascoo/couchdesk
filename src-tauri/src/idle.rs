//! Salvapantallas: un hilo vigila cuánto tiempo lleva el PC sin tocarse (teclado, ratón y
//! mandos de Xbox) y, pasado el tiempo elegido, enseña la ventana "salvapantallas" encima de
//! todo (también de juegos y programas) sin quitarles el foco. Cualquier tecla, un movimiento
//! claro del ratón o cualquier botón o stick del mando la esconde y todo sigue como estaba.
//! Ajustes en %LOCALAPPDATA%\Consola\salvapantallas.json (así funciona aunque la consola esté oculta).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_GAMEPAD, XINPUT_STATE};
use windows::Win32::UI::WindowsAndMessaging::*;

pub const LABEL: &str = "salvapantallas";
/// Tras enseñarlo, un momento sin vigilar (soltar el botón de "Probar ahora", ruido del stick…)
const GRACE: Duration = Duration::from_millis(1500);
/// Píxeles que el ratón puede moverse sin despertar (mesa que vibra, ratón óptico nervioso)
const MOUSE_SLACK: i32 = 12;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub enabled: bool,
    /// Minutos sin usar el PC antes de empezar
    pub after_min: u32,
    /// "paisajes" | "bajo-consumo" | "reloj"
    pub style: String,
    /// Colecciones de vídeos: costas, montanas, bosques, ciudades, nubes, agua
    pub collections: Vec<String>,
    pub show_weather: bool,
    /// "auto" (ubicación aproximada por internet) | "ciudad"
    pub location_mode: String,
    pub place: Option<crate::weather::Place>,
    /// No empezar si delante hay un programa o juego a pantalla completa
    pub skip_fullscreen: bool,
    /// Minutos de salvapantallas antes de apagar la pantalla (0 = nunca)
    pub monitor_off_min: u32,
    /// Sonido de ambiente bajo los vídeos (lo hace la página)
    pub sound: bool,
    /// 0-100
    pub sound_volume: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            after_min: 5,
            style: "paisajes".into(),
            collections: ["costas", "montanas", "bosques", "ciudades", "nubes", "agua"].map(String::from).to_vec(),
            show_weather: true,
            location_mode: "ciudad".into(),
            place: None,
            skip_fullscreen: true,
            monitor_off_min: 0,
            sound: false,
            sound_volume: 50,
        }
    }
}

static SETTINGS: Mutex<Option<Settings>> = Mutex::new(None);
static SHOWN: AtomicBool = AtomicBool::new(false);
static PREVIEW: AtomicBool = AtomicBool::new(false);
static SHOWN_AT: Mutex<Option<Instant>> = Mutex::new(None);
static MONITOR_OFF: AtomicBool = AtomicBool::new(false);
/// La consola lo pide mientras se ve algo a propósito (modo ambiente)
static INHIBIT: AtomicBool = AtomicBool::new(false);

fn settings_path() -> PathBuf {
    PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into()))
        .join("Consola")
        .join("salvapantallas.json")
}

/// Ajustes actuales (se leen del disco la primera vez)
pub fn settings() -> Settings {
    let mut s = SETTINGS.lock().unwrap();
    s.get_or_insert_with(|| {
        std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    })
    .clone()
}

// ---------------- Lógica pura (con pruebas) ----------------

/// ¿Toca empezar? `blocked`: hay algo a pantalla completa y se pidió esperar
pub fn should_start(s: &Settings, idle: Duration, blocked: bool) -> bool {
    s.enabled && !blocked && idle >= Duration::from_secs(u64::from(s.after_min.max(1)) * 60)
}

/// ¿Toca apagar la pantalla tras `shown_for` de salvapantallas?
pub fn should_power_off(s: &Settings, shown_for: Duration) -> bool {
    s.monitor_off_min > 0 && shown_for >= Duration::from_secs(u64::from(s.monitor_off_min) * 60)
}

/// Una lectura de la entrada: último toque de teclado/ratón, puntero y mandos
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Sample {
    pub tick: u32,
    pub cursor: (i32, i32),
    /// Estado resumido de cada mando (None = no conectado)
    pub pads: [Option<u32>; 4],
}

/// ¿Alguien ha tocado algo? `base_cursor`: puntero al empezar a vigilar; `prev`: lectura anterior.
pub fn is_wake(base_cursor: (i32, i32), prev: &Sample, now: &Sample) -> bool {
    // Un mando que cambia (si se apaga solo por inactividad, no cuenta)
    let pad = prev.pads.iter().zip(now.pads.iter()).any(|(a, b)| matches!((a, b), (Some(x), Some(y)) if x != y));
    if pad {
        return true;
    }
    if now.tick != prev.tick {
        // Entrada sin mover el puntero: tecla, clic o rueda
        if now.cursor == prev.cursor {
            return true;
        }
        let d = (now.cursor.0 - base_cursor.0).abs().max((now.cursor.1 - base_cursor.1).abs());
        return d > MOUSE_SLACK;
    }
    false
}

/// Botones, gatillos y sticks (solo pasada la zona muerta) en un número
pub fn quantize(g: &XINPUT_GAMEPAD) -> u32 {
    const DEAD: i16 = 12000;
    let dir = |v: i16| -> u32 {
        if v > DEAD { 1 } else if v < -DEAD { 2 } else { 0 }
    };
    u32::from(g.wButtons.0)
        | (u32::from(g.bLeftTrigger > 40) << 16)
        | (u32::from(g.bRightTrigger > 40) << 17)
        | (dir(g.sThumbLX) << 18)
        | (dir(g.sThumbLY) << 20)
        | (dir(g.sThumbRX) << 22)
        | (dir(g.sThumbRY) << 24)
}

// ---------------- Lecturas de Windows ----------------

/// XInputGetStateEx (ordinal 100) también ve el botón central; si no está, la normal
fn read_pads() -> [Option<u32>; 4] {
    type GetStateEx = unsafe extern "system" fn(u32, *mut XINPUT_STATE) -> u32;
    static EX: OnceLock<Option<GetStateEx>> = OnceLock::new();
    let ex = *EX.get_or_init(|| unsafe {
        use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
        let dll = LoadLibraryW(windows::core::w!("xinput1_4.dll")).ok()?;
        let p = GetProcAddress(dll, windows::core::PCSTR(100 as *const u8))?;
        Some(std::mem::transmute::<_, GetStateEx>(p))
    });
    let mut out = [None; 4];
    for (i, slot) in out.iter_mut().enumerate() {
        let mut s = XINPUT_STATE::default();
        let ok = unsafe {
            match ex {
                Some(f) => f(i as u32, &mut s) == 0,
                None => XInputGetState(i as u32, &mut s) == 0,
            }
        };
        if ok {
            *slot = Some(quantize(&s.Gamepad));
        }
    }
    out
}

fn sample() -> Sample {
    let mut li = LASTINPUTINFO { cbSize: size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    let mut pt = POINT::default();
    unsafe {
        let _ = GetLastInputInfo(&mut li);
        let _ = GetCursorPos(&mut pt);
    }
    Sample { tick: li.dwTime, cursor: (pt.x, pt.y), pads: read_pads() }
}

fn system_idle(tick: u32) -> Duration {
    Duration::from_millis(u64::from(unsafe { GetTickCount() }.wrapping_sub(tick)))
}

fn monitor_rect(of: HWND) -> RECT {
    let mut mi = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    unsafe {
        let m = MonitorFromWindow(of, MONITOR_DEFAULTTOPRIMARY);
        let _ = GetMonitorInfoW(m, &mut mi);
    }
    mi.rcMonitor
}

/// ¿Hay delante una ventana a pantalla completa que no es de la consola ni el escritorio?
fn foreground_fullscreen() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_invalid() || fg == GetShellWindow() || fg == GetDesktopWindow() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        if pid == std::process::id() {
            return false;
        }
        let mut class = [0u16; 32];
        let n = GetClassNameW(fg, &mut class) as usize;
        if matches!(String::from_utf16_lossy(&class[..n]).as_str(), "WorkerW" | "Progman") {
            return false;
        }
        let mut r = RECT::default();
        if GetWindowRect(fg, &mut r).is_err() {
            return false;
        }
        let m = monitor_rect(fg);
        r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom
    }
}

fn turn_monitor_off() {
    // PostMessage y no SendMessage: un programa colgado no bloquea este hilo
    unsafe {
        let _ = PostMessageW(Some(HWND_BROADCAST), WM_SYSCOMMAND, WPARAM(SC_MONITORPOWER as usize), LPARAM(2));
    }
}

// ---------------- Ventana ----------------

fn hwnd_of(w: &tauri::WebviewWindow) -> Option<HWND> {
    w.hwnd().ok().map(|h| HWND(h.0 as _))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub visible: bool,
    pub preview: bool,
}

fn status() -> Status {
    Status { visible: SHOWN.load(Ordering::Relaxed), preview: PREVIEW.load(Ordering::Relaxed) }
}

/// Enseña el salvapantallas encima de todo, sin quitar el foco a nadie.
/// Nunca desde una orden síncrona (en Windows crear ventanas ahí bloquea la app).
pub fn show(app: &AppHandle, preview: bool) {
    // Pantalla donde está lo que se usa ahora
    let rect = monitor_rect(unsafe { GetForegroundWindow() });
    let w = match app.get_webview_window(LABEL) {
        Some(w) => w,
        None => {
            // focused(false): aparece sin activarse, el juego o programa de delante sigue con el foco
            let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("salvapantallas".into()))
                .title(crate::i18n::tr("idle.title"))
                .decorations(false)
                .shadow(false)
                .resizable(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .focused(false)
                .focusable(false)
                .background_color(tauri::window::Color(0, 0, 0, 255))
                .additional_browser_args("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required")
                .build();
            match built {
                Ok(w) => w,
                Err(e) => {
                    log::error!("Salvapantallas: {e}");
                    return;
                }
            }
        }
    };
    let Some(h) = hwnd_of(&w) else { return };
    unsafe {
        let _ = SetWindowPos(
            h,
            Some(HWND_TOPMOST),
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
    PREVIEW.store(preview, Ordering::Relaxed);
    MONITOR_OFF.store(false, Ordering::Relaxed);
    *SHOWN_AT.lock().unwrap() = Some(Instant::now());
    SHOWN.store(true, Ordering::Relaxed);
    let _ = app.emit("salvapantallas", status());
}

/// Lo esconde (la ventana se queda creada para la próxima vez, con los vídeos en pausa)
pub fn hide(app: &AppHandle) {
    if !SHOWN.swap(false, Ordering::Relaxed) {
        return;
    }
    PREVIEW.store(false, Ordering::Relaxed);
    if let Some(h) = app.get_webview_window(LABEL).as_ref().and_then(hwnd_of) {
        unsafe {
            let _ = ShowWindow(h, SW_HIDE);
        }
    }
    // Abierta solo como salvapantallas de Windows (`/s`): al despertar, se cierra
    if crate::scr::saver_only() {
        app.exit(0);
    }
    let _ = app.emit("salvapantallas", status());
}

// ---------------- Vigilante ----------------

fn watcher(app: AppHandle) {
    let mut prev = sample();
    let mut pad_changed = Instant::now();
    let mut base: Option<(i32, i32)> = None; // puntero al empezar a vigilar el salvapantallas
    loop {
        let shown = SHOWN.load(Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(if shown { 100 } else { 500 }));
        let now = sample();
        if now.pads.iter().zip(prev.pads.iter()).any(|(a, b)| a.is_some() && a != b) {
            pad_changed = Instant::now();
        }
        if shown {
            let shown_for = SHOWN_AT.lock().unwrap().map(|t| t.elapsed()).unwrap_or_default();
            if shown_for < GRACE {
                base = None;
            } else if let Some(b) = base {
                if is_wake(b, &prev, &now) {
                    hide(&app);
                    base = None;
                } else if should_power_off(&settings(), shown_for) && !MONITOR_OFF.swap(true, Ordering::Relaxed) {
                    turn_monitor_off();
                }
            } else {
                base = Some(now.cursor);
            }
        } else {
            let s = settings();
            let idle = system_idle(now.tick).min(pad_changed.elapsed());
            // Lo caro (mirar la ventana de delante) solo cuando ya toca
            if should_start(&s, idle, INHIBIT.load(Ordering::Relaxed)) && !(s.skip_fullscreen && foreground_fullscreen()) {
                show(&app, false);
            }
        }
        prev = now;
    }
}

/// Se llama una vez al arrancar la app
pub fn start(app: AppHandle) {
    std::thread::spawn(move || watcher(app));
}

// ---------------- Órdenes ----------------

#[tauri::command(async)]
pub fn screensaver_settings_get() -> Settings {
    settings()
}

#[tauri::command(async)]
pub fn screensaver_settings_set(settings: Settings) -> Result<Settings, String> {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|_| crate::i18n::tr("idle.save_failed").to_string())?;
    *SETTINGS.lock().unwrap() = Some(settings.clone());
    Ok(settings)
}

#[tauri::command]
pub async fn screensaver_show(app: AppHandle) {
    show(&app, false);
}

#[tauri::command]
pub async fn screensaver_preview(app: AppHandle) {
    show(&app, true);
}

#[tauri::command]
pub async fn screensaver_hide(app: AppHandle) {
    hide(&app);
}

#[tauri::command(async)]
pub fn screensaver_status() -> Status {
    status()
}

/// `true` mientras se ve algo que no debe taparse (modo ambiente); `false` al salir
#[tauri::command(async)]
pub fn screensaver_inhibit(on: bool) {
    INHIBIT.store(on, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empieza_tras_el_tiempo() {
        let mut s = Settings { after_min: 5, ..Default::default() };
        assert!(!should_start(&s, Duration::from_secs(299), false));
        assert!(should_start(&s, Duration::from_secs(300), false));
        assert!(!should_start(&s, Duration::from_secs(900), true), "pantalla completa delante");
        s.enabled = false;
        assert!(!should_start(&s, Duration::from_secs(900), false));
        s = Settings { after_min: 0, ..Default::default() };
        assert!(!should_start(&s, Duration::from_secs(30), false), "mínimo 1 minuto");
    }

    #[test]
    fn apagar_pantalla() {
        let mut s = Settings::default();
        assert!(!should_power_off(&s, Duration::from_secs(99_999)), "nunca");
        s.monitor_off_min = 15;
        assert!(!should_power_off(&s, Duration::from_secs(14 * 60)));
        assert!(should_power_off(&s, Duration::from_secs(15 * 60)));
    }

    #[test]
    fn despertar() {
        let base = (500, 500);
        let a = Sample { tick: 1, cursor: (500, 500), pads: [Some(0), None, None, None] };
        assert!(!is_wake(base, &a, &a), "nada cambia");
        // Ratón nervioso: unos píxeles no despiertan
        let jitter = Sample { tick: 2, cursor: (504, 497), ..a };
        assert!(!is_wake(base, &a, &jitter));
        // Movimiento claro
        let moved = Sample { tick: 3, cursor: (540, 500), ..a };
        assert!(is_wake(base, &jitter, &moved));
        // Tecla: cambia el tick sin mover el puntero (aunque antes hubiera temblor)
        let key = Sample { tick: 4, ..jitter };
        assert!(is_wake(base, &jitter, &key));
        // Mando: botón o stick
        let pad = Sample { pads: [Some(0x1000), None, None, None], ..a };
        assert!(is_wake(base, &a, &pad));
        // Mando que se apaga solo: no despierta
        let off = Sample { pads: [None; 4], ..a };
        assert!(!is_wake(base, &a, &off));
    }

    #[test]
    fn sticks_con_zona_muerta() {
        let quiet = XINPUT_GAMEPAD { sThumbLX: 3000, sThumbRY: -5000, bLeftTrigger: 10, ..Default::default() };
        assert_eq!(quantize(&quiet), 0, "ruido del stick no cuenta");
        let pushed = XINPUT_GAMEPAD { sThumbLX: 20000, ..Default::default() };
        assert_ne!(quantize(&pushed), 0);
    }

    #[test]
    fn ajustes_parciales() {
        // Un archivo antiguo o a medias se completa con los valores por defecto
        let s: Settings = serde_json::from_str(r#"{"afterMin":10,"style":"bajo-consumo"}"#).unwrap();
        assert_eq!(s.after_min, 10);
        assert_eq!(s.style, "bajo-consumo");
        assert!(s.enabled && s.show_weather);
    }
}
