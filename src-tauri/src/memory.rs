//! Gestor de memoria (como en iOS): la consola se hace pequeña cuando juegas y vuelve al instante.
//! a. CouchDesk detrás (un juego, un programa o su navegador delante): a los ~1,5 s su WebView pide
//!    poca memoria, el proceso baja a prioridad "por debajo de lo normal" con modo eficiencia
//!    (EcoQoS) y suelta su memoria de trabajo una vez. Al volver delante, todo se deshace al momento.
//! b. El juego o programa a pantalla completa que está delante sube a prioridad "por encima de lo
//!    normal" (nunca alta ni tiempo real) y sin modo eficiencia. Al salir de delante, vuelve a la suya.
//! c. Programas en segundo plano (del usuario, no de Windows, sin sonar, fuera de la lista protegida):
//!    a los 30 s pasan a modo eficiencia, igual que hace el Administrador de tareas (EcoQoS + prioridad
//!    baja) y sueltan su memoria de trabajo una vez. Al volver delante, se deshace.
//! d. "Liberar memoria": vacía la memoria de trabajo de los programas en segundo plano.
//! Todo lo cambiado se apunta (también en disco) y se deshace al cerrar la consola o al arrancar.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const MB: u64 = 1024 * 1024;
/// Tiempo detrás antes de aligerar la consola (cambios rápidos de ventana no cuentan)
const OWN_DELAY: Duration = Duration::from_millis(1500);
/// Tiempo en segundo plano antes del modo eficiencia
const BACKGROUND_DELAY: Duration = Duration::from_secs(30);
/// Cada cuánto se repasan los programas en segundo plano
const SCAN_EVERY: Duration = Duration::from_secs(5);

// Clases de prioridad (valores de Windows)
pub const IDLE: u32 = 0x40;
pub const BELOW_NORMAL: u32 = 0x4000;
pub const NORMAL: u32 = 0x20;
pub const ABOVE_NORMAL: u32 = 0x8000;

/// Nunca se tocan (en minúsculas). Lo de la carpeta de Windows tampoco (ver `eligible`).
pub const NEVER: &[&str] = &[
    // Windows
    "explorer.exe", "dwm.exe", "csrss.exe", "winlogon.exe", "services.exe", "lsass.exe", "svchost.exe", "smss.exe",
    "fontdrvhost.exe", "sihost.exe", "ctfmon.exe", "audiodg.exe", "taskmgr.exe", "conhost.exe",
    // La consola, su mando, sus webs y su escritorio remoto
    "couchdesk.exe", "consola.exe", "msedgewebview2.exe", "xtend.exe", "rustdesk.exe",
    // Steam (su cliente sostiene los juegos de Steam) y superposiciones de juego
    "steam.exe", "steamwebhelper.exe", "steamservice.exe", "gamebar.exe", "gamebarftserver.exe",
    "nvcontainer.exe", "nvidia share.exe", "nvidia overlay.exe", "discord.exe", "obs64.exe", "obs32.exe",
    // Red
    "cloudflare warp.exe", "warp-svc.exe",
    // Antivirus
    "msmpeng.exe", "nissrv.exe", "securityhealthsystray.exe", "securityhealthservice.exe", "mbam.exe", "mbamtray.exe",
    "avp.exe", "avpui.exe", "avastui.exe", "avgui.exe", "bdagent.exe", "vsserv.exe", "egui.exe", "ekrn.exe",
    "mcshield.exe", "nortonsecurity.exe", "ns.exe",
];

/// Carpetas típicas de juegos (en minúsculas)
const GAME_DIRS: &[&str] = &[
    "\\steamapps\\common\\", "\\epic games\\", "\\xboxgames\\", "\\gog games\\", "\\gog galaxy\\games\\",
    "\\riot games\\", "\\ea games\\", "\\ubisoft game launcher\\games\\", "\\rockstar games\\", "\\games\\",
    "\\couchdesk\\emuladores\\", "\\consola\\emuladores\\",
];

// ---------------------------------------------------------------- Ajustes

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Gestión de memoria automática (a, b y c)
    #[serde(default = "yes")]
    pub auto: bool,
    /// Modo eficiencia para apps en segundo plano (c)
    #[serde(default = "yes")]
    pub eco: bool,
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings { auto: true, eco: true }
    }
}

static SETTINGS: Mutex<Option<Settings>> = Mutex::new(None);

fn settings_file() -> std::path::PathBuf {
    crate::paths::data_dir().join("memoria.json")
}

pub fn settings() -> Settings {
    let mut s = SETTINGS.lock().unwrap();
    *s.get_or_insert_with(|| {
        std::fs::read_to_string(settings_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    })
}

fn save_settings(s: Settings) {
    *SETTINGS.lock().unwrap() = Some(s);
    let _ = std::fs::create_dir_all(crate::paths::data_dir());
    let _ = std::fs::write(settings_file(), serde_json::to_string_pretty(&s).unwrap_or_default());
}

// ---------------------------------------------------------------- Reglas (sin efectos: se prueban)

/// Lo que se sabe de un proceso para decidir
#[derive(Clone, Debug, Default)]
pub struct ProcInfo {
    pub pid: u32,
    /// Nombre del programa en minúsculas ("spotify.exe")
    pub exe: String,
    /// Ruta completa en minúsculas (vacía si Windows no la da)
    pub path: String,
    pub same_session: bool,
    /// Es la consola (o su proceso del mando)
    pub ours: bool,
    pub foreground: bool,
    /// Tiene una sesión de sonido activa (música, llamada…)
    pub audio: bool,
}

/// ¿Es de Windows? (su carpeta, o sin ruta legible: entonces mejor no tocarlo)
pub fn is_system(path: &str, windir: &str) -> bool {
    path.is_empty() || (!windir.is_empty() && path.starts_with(windir))
}

/// ¿Se puede pasar a modo eficiencia o aligerar este proceso en segundo plano?
pub fn eligible(p: &ProcInfo, windir: &str) -> bool {
    p.pid > 4
        && p.same_session
        && !p.ours
        && !p.foreground
        && !p.audio
        && !is_system(&p.path, windir)
        && !NEVER.contains(&p.exe.as_str())
}

/// ¿Parece un juego por su carpeta?
pub fn is_game_path(path: &str) -> bool {
    let p = path.to_lowercase();
    GAME_DIRS.iter().any(|d| p.contains(d))
}

/// ¿Sube de prioridad lo que está delante? Solo juegos o pantalla completa, y solo si estaba en "normal"
/// (si el programa eligió otra, se respeta).
pub fn should_boost(p: &ProcInfo, fullscreen: bool, priority: u32, windir: &str) -> bool {
    p.pid > 4
        && !p.ours
        && (fullscreen || is_game_path(&p.path))
        && priority == NORMAL
        && !is_system(&p.path, windir)
        && !NEVER.contains(&p.exe.as_str())
}

/// ¿Lleva bastante tiempo en segundo plano?
pub fn background_due(since: Instant, now: Instant) -> bool {
    now.duration_since(since) >= BACKGROUND_DELAY
}

/// ¿Toca aligerar la consola? (lleva `OWN_DELAY` detrás)
pub fn own_due(away_since: Option<Instant>, now: Instant) -> bool {
    away_since.is_some_and(|t| now.duration_since(t) >= OWN_DELAY)
}

/// Prioridad a devolver: solo si sigue siendo la que pusimos (si otro la cambió, se deja)
pub fn restore_priority(current: u32, ours: u32, original: u32) -> Option<u32> {
    (current == ours && original != ours).then_some(original)
}

// ---------------------------------------------------------------- Windows

pub mod win {
    use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, HWND, RECT};
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::System::ProcessStatus::*;
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows::Win32::System::Threading::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::PWSTR;

    /// Proceso abierto (se cierra solo)
    pub struct Proc(pub HANDLE);
    impl Drop for Proc {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    pub fn open(pid: u32, access: PROCESS_ACCESS_RIGHTS) -> Option<Proc> {
        unsafe { OpenProcess(access, false, pid).ok().map(Proc) }
    }

    pub fn path(p: &Proc) -> String {
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        unsafe {
            if QueryFullProcessImageNameW(p.0, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok() {
                return String::from_utf16_lossy(&buf[..len as usize]);
            }
        }
        String::new()
    }

    /// Momento en que arrancó (para no confundir un proceso con otro que reutiliza su número)
    pub fn created(p: &Proc) -> u64 {
        let (mut c, mut e, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        unsafe {
            if GetProcessTimes(p.0, &mut c, &mut e, &mut k, &mut u).is_ok() {
                return (u64::from(c.dwHighDateTime) << 32) | u64::from(c.dwLowDateTime);
            }
        }
        0
    }

    pub fn priority(p: &Proc) -> u32 {
        unsafe { GetPriorityClass(p.0) }
    }

    pub fn set_priority(p: &Proc, class: u32) -> bool {
        unsafe { SetPriorityClass(p.0, PROCESS_CREATION_FLAGS(class)).is_ok() }
    }

    /// Modo eficiencia (EcoQoS): Some(true) activado, Some(false) prohibido, None lo decide Windows
    pub fn set_eco(p: &Proc, on: Option<bool>) -> bool {
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: if on.is_some() { PROCESS_POWER_THROTTLING_EXECUTION_SPEED } else { 0 },
            StateMask: if on == Some(true) { PROCESS_POWER_THROTTLING_EXECUTION_SPEED } else { 0 },
        };
        unsafe {
            SetProcessInformation(p.0, ProcessPowerThrottling, &state as *const _ as *const _, std::mem::size_of_val(&state) as u32).is_ok()
        }
    }

    /// Memoria de trabajo (lo que el proceso tiene ahora mismo en RAM), en bytes
    pub fn working_set(p: &Proc) -> u64 {
        let mut c = PROCESS_MEMORY_COUNTERS_EX::default();
        let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        c.cb = size;
        unsafe {
            if K32GetProcessMemoryInfo(p.0, &mut c as *mut _ as *mut PROCESS_MEMORY_COUNTERS, size).as_bool() {
                return c.WorkingSetSize as u64;
            }
        }
        0
    }

    pub fn trim(p: &Proc) -> bool {
        unsafe { K32EmptyWorkingSet(p.0).as_bool() }
    }

    pub fn session(pid: u32) -> Option<u32> {
        let mut s = 0;
        unsafe { windows::Win32::System::RemoteDesktop::ProcessIdToSessionId(pid, &mut s).ok().map(|_| s) }
    }

    pub fn pids() -> Vec<u32> {
        let mut buf = vec![0u32; 4096];
        let mut needed = 0u32;
        unsafe {
            if !K32EnumProcesses(buf.as_mut_ptr(), (buf.len() * 4) as u32, &mut needed).as_bool() {
                return Vec::new();
            }
        }
        buf.truncate(needed as usize / 4);
        buf
    }

    /// (total, disponible) en bytes
    pub fn ram() -> (u64, u64) {
        let mut m = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
        unsafe {
            if GlobalMemoryStatusEx(&mut m).is_ok() {
                return (m.ullTotalPhys, m.ullAvailPhys);
            }
        }
        (0, 0)
    }

    pub fn foreground() -> (HWND, u32) {
        unsafe {
            let h = GetForegroundWindow();
            let mut pid = 0;
            GetWindowThreadProcessId(h, Some(&mut pid));
            (h, pid)
        }
    }

    /// ¿La ventana ocupa todo su monitor? (juegos y vídeos a pantalla completa)
    pub fn fullscreen(h: HWND) -> bool {
        unsafe {
            if h.is_invalid() {
                return false;
            }
            let mut class = [0u16; 64];
            let n = GetClassNameW(h, &mut class) as usize;
            if ["Progman", "WorkerW", "Shell_TrayWnd"].contains(&String::from_utf16_lossy(&class[..n]).as_str()) {
                return false;
            }
            let mut r = RECT::default();
            if GetWindowRect(h, &mut r).is_err() {
                return false;
            }
            let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            if !GetMonitorInfoW(MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST), &mut mi).as_bool() {
                return false;
            }
            let m = mi.rcMonitor;
            r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom
        }
    }

    /// Procesos con sonido activo ahora mismo (Spotify sonando, una llamada…)
    pub fn audio_pids() -> std::collections::HashSet<u32> {
        use windows::Win32::Media::Audio::*;
        use windows::Win32::System::Com::*;
        use windows::core::Interface;
        let mut out = std::collections::HashSet::new();
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return out };
            let Ok(devices) = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) else { return out };
            for d in 0..devices.GetCount().unwrap_or(0) {
                let Ok(dev) = devices.Item(d) else { continue };
                let Ok(mgr) = dev.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else { continue };
                let Ok(list) = mgr.GetSessionEnumerator() else { continue };
                for i in 0..list.GetCount().unwrap_or(0) {
                    let Ok(ctl) = list.GetSession(i) else { continue };
                    if ctl.GetState().ok() != Some(AudioSessionStateActive) {
                        continue;
                    }
                    if let Ok(pid) = ctl.cast::<IAudioSessionControl2>().and_then(|c| c.GetProcessId()) {
                        if pid != 0 {
                            out.insert(pid);
                        }
                    }
                }
            }
        }
        out
    }

    /// Este hilo responde siempre al momento, aunque la consola esté en modo eficiencia
    /// (gancho del teclado, botón Xbox, mando…)
    pub fn keep_thread_responsive() {
        let state = THREAD_POWER_THROTTLING_STATE {
            Version: THREAD_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: THREAD_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: 0,
        };
        unsafe {
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
            let _ = SetThreadInformation(GetCurrentThread(), ThreadPowerThrottling, &state as *const _ as *const _, std::mem::size_of_val(&state) as u32);
        }
    }
}

pub use win::keep_thread_responsive;

// ---------------------------------------------------------------- Lo que hemos cambiado

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Change {
    pub pid: u32,
    pub created: u64,
    pub exe: String,
    /// Prioridad que tenía y la que le pusimos
    pub original: u32,
    pub set: u32,
}

#[derive(Default)]
struct State {
    /// La consola está aligerada
    own_low: bool,
    /// Desde cuándo está la consola detrás
    away_since: Option<Instant>,
    /// Nivel de memoria pedido a cada WebView (true = bajo)
    webview_low: HashMap<&'static str, bool>,
    suspended: HashSet<&'static str>,
    boosted: Option<Change>,
    eco: HashMap<u32, Change>,
    /// Desde cuándo está cada proceso en segundo plano (con su momento de arranque)
    since: HashMap<u32, (u64, Instant)>,
    last_scan: Option<Instant>,
    last_freed: Option<Freed>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
static WAKE: OnceLock<Mutex<Sender<()>>> = OnceLock::new();

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut g = STATE.lock().unwrap();
    f(g.get_or_insert_with(State::default))
}

fn changes_file() -> std::path::PathBuf {
    crate::paths::data_dir().join("memoria-cambios.json")
}

/// Apunta en disco lo cambiado (si la consola se cierra de golpe, se deshace al arrancar)
fn persist(s: &State) {
    let list: Vec<&Change> = s.eco.values().chain(s.boosted.iter()).collect();
    if list.is_empty() {
        let _ = std::fs::remove_file(changes_file());
    } else {
        let _ = std::fs::create_dir_all(crate::paths::data_dir());
        let _ = std::fs::write(changes_file(), serde_json::to_string(&list).unwrap_or_default());
    }
}

/// Devuelve un proceso a como estaba (si sigue siendo el mismo)
fn undo(c: &Change) {
    use windows::Win32::System::Threading::*;
    let Some(p) = win::open(c.pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_INFORMATION) else { return };
    if win::created(&p) != c.created {
        return;
    }
    if let Some(prio) = restore_priority(win::priority(&p), c.set, c.original) {
        win::set_priority(&p, prio);
    }
    win::set_eco(&p, None);
}

fn windir() -> String {
    std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into()).to_lowercase() + "\\"
}

fn own_exe() -> String {
    std::env::current_exe().map(|p| p.display().to_string().to_lowercase()).unwrap_or_default()
}

fn info(pid: u32, fg_pid: u32, audio: &HashSet<u32>, my_session: Option<u32>, me: &str) -> Option<(ProcInfo, u64)> {
    use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
    let p = win::open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    let path = win::path(&p).to_lowercase();
    let exe = path.rsplit('\\').next().unwrap_or_default().to_string();
    let i = ProcInfo {
        pid,
        ours: pid == std::process::id() || (!me.is_empty() && path == me),
        foreground: pid == fg_pid,
        audio: audio.contains(&pid),
        same_session: my_session.is_some() && win::session(pid) == my_session,
        exe,
        path,
    };
    Some((i, win::created(&p)))
}

// ---------------------------------------------------------------- La consola

/// ¿Hay móviles conectados como mando? Entonces la consola no se frena (el mando del móvil va por ella)
fn phone_connected(app: &AppHandle) -> bool {
    let Some(hub) = app.try_state::<std::sync::Arc<crate::remote::Hub>>() else { return false };
    serde_json::to_value(crate::remote::remote_status(hub))
        .ok()
        .and_then(|v| v.get("clients").and_then(|c| c.as_array()).map(|a| !a.is_empty()))
        .unwrap_or(false)
}

/// Pide a una WebView poca memoria (o la normal). Va por el hilo principal de la app.
fn set_webview_low(app: &AppHandle, label: &'static str, low: bool) {
    let changed = with_state(|s| s.webview_low.insert(label, low) != Some(low));
    if !changed {
        return;
    }
    let Some(w) = app.get_webview_window(label) else {
        with_state(|s| s.webview_low.remove(label));
        return;
    };
    let _ = w.with_webview(move |pw| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::*;
        use windows::core::Interface;
        let Ok(core) = pw.controller().CoreWebView2() else { return };
        if let Ok(c) = core.cast::<ICoreWebView2_19>() {
            let level = if low { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW } else { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL };
            let _ = c.SetMemoryUsageTargetLevel(level);
        }
    });
}

/// Navegador o nube escondidos o minimizados: se duermen (WebView2 los despierta al verse)
fn suspend_hidden(app: &AppHandle, fg_label: Option<&str>) {
    for label in ["navegador", "nube"] {
        let Some(w) = app.get_webview_window(label) else {
            with_state(|s| s.suspended.remove(label));
            continue;
        };
        let hidden = !w.is_visible().unwrap_or(true) || w.is_minimized().unwrap_or(false);
        let was = with_state(|s| s.suspended.contains(label));
        if hidden && !was && fg_label != Some(label) {
            with_state(|s| s.suspended.insert(label));
            let _ = w.with_webview(|pw| unsafe {
                use webview2_com::Microsoft::Web::WebView2::Win32::*;
                use windows::core::Interface;
                if let Ok(c) = pw.controller().CoreWebView2().and_then(|c| c.cast::<ICoreWebView2_3>()) {
                    let handler = webview2_com::TrySuspendCompletedHandler::create(Box::new(|_, _| Ok(())));
                    let _ = c.TrySuspend(&handler);
                }
            });
        } else if !hidden && was {
            with_state(|s| s.suspended.remove(label));
            let _ = w.with_webview(|pw| unsafe {
                use webview2_com::Microsoft::Web::WebView2::Win32::*;
                use windows::core::Interface;
                if let Ok(c) = pw.controller().CoreWebView2().and_then(|c| c.cast::<ICoreWebView2_3>()) {
                    let _ = c.Resume();
                }
            });
        }
    }
}

fn own_lighten(app: &AppHandle) {
    use windows::Win32::System::Threading::GetCurrentProcess;
    let me = win::Proc(unsafe { GetCurrentProcess() }); // seudo-identificador: cerrarlo no hace nada
    if !phone_connected(app) {
        win::set_priority(&me, BELOW_NORMAL);
        win::set_eco(&me, Some(true));
    }
    win::trim(&me);
    std::mem::forget(me);
}

fn own_restore() {
    use windows::Win32::System::Threading::GetCurrentProcess;
    let me = win::Proc(unsafe { GetCurrentProcess() });
    win::set_priority(&me, NORMAL);
    win::set_eco(&me, None);
    std::mem::forget(me);
}

/// La consola vuelve delante: todo a la normalidad al instante (lo llama también Multitarea)
pub fn wake(app: &AppHandle) {
    let was_low = with_state(|s| {
        s.away_since = None;
        std::mem::replace(&mut s.own_low, false)
    });
    if was_low {
        own_restore();
    }
    set_webview_low(app, "main", false);
}

// ---------------------------------------------------------------- Ciclo

fn tick(app: &AppHandle, now: Instant) {
    let s = settings();
    let (fg, fg_pid) = win::foreground();
    let own_pid = std::process::id();
    let ours_front = fg_pid == own_pid;
    let label_of = |label: &str| app.get_webview_window(label).and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize);
    let fg_raw = fg.0 as isize;
    let fg_label = ["main", "navegador", "nube", "asistente"].into_iter().find(|l| label_of(l) == Some(fg_raw));

    if !s.auto {
        restore_all(app);
        return;
    }

    // a. La consola
    if fg_label == Some("main") || fg_label == Some("asistente") {
        wake(app);
    } else {
        let due = with_state(|st| {
            let since = *st.away_since.get_or_insert(now);
            own_due(Some(since), now)
        });
        if due {
            // El navegador o la nube son la consola: entonces el proceso no se frena, solo la pantalla de Inicio
            set_webview_low(app, "main", true);
            let lighten = !ours_front && with_state(|st| !std::mem::replace(&mut st.own_low, true));
            if lighten {
                own_lighten(app);
            } else if ours_front && with_state(|st| std::mem::replace(&mut st.own_low, false)) {
                own_restore();
            }
        }
    }
    for label in ["navegador", "nube"] {
        if app.get_webview_window(label).is_some() {
            set_webview_low(app, label, fg_label != Some(label));
        } else {
            with_state(|st| st.webview_low.remove(label));
        }
    }
    suspend_hidden(app, fg_label);

    // b. Lo que está delante
    let windir = windir();
    let me = own_exe();
    let prev = with_state(|st| st.boosted.clone());
    if prev.as_ref().is_some_and(|b| b.pid != fg_pid) {
        undo(prev.as_ref().unwrap());
        with_state(|st| {
            st.boosted = None;
            persist(st);
        });
    }
    // Si estaba en modo eficiencia, se le quita ya
    if let Some(c) = with_state(|st| st.eco.remove(&fg_pid)) {
        undo(&c);
        with_state(|st| persist(st));
    }
    with_state(|st| st.since.remove(&fg_pid));
    if !ours_front && fg_pid > 4 && with_state(|st| st.boosted.is_none()) {
        boost(fg, fg_pid, &windir, &me);
    }

    // c. Segundo plano (cada pocos segundos)
    let scan = with_state(|st| {
        let due = st.last_scan.is_none_or(|t| now.duration_since(t) >= SCAN_EVERY);
        if due {
            st.last_scan = Some(now);
        }
        due
    });
    if scan {
        if s.eco {
            background(fg_pid, &windir, &me, now);
        } else {
            let all: Vec<Change> = with_state(|st| st.eco.drain().map(|(_, c)| c).collect());
            all.iter().for_each(undo);
            with_state(|st| persist(st));
        }
    }
}

fn boost(fg: windows::Win32::Foundation::HWND, pid: u32, windir: &str, me: &str) {
    use windows::Win32::System::Threading::*;
    let Some(p) = win::open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_INFORMATION) else { return };
    let path = win::path(&p).to_lowercase();
    let info = ProcInfo {
        pid,
        exe: path.rsplit('\\').next().unwrap_or_default().to_string(),
        ours: path == me,
        path: path.clone(),
        ..Default::default()
    };
    let prio = win::priority(&p);
    if !should_boost(&info, win::fullscreen(fg), prio, windir) {
        return;
    }
    if win::set_priority(&p, ABOVE_NORMAL) {
        win::set_eco(&p, Some(false));
        let c = Change { pid, created: win::created(&p), exe: info.exe, original: prio, set: ABOVE_NORMAL };
        log::info!("Memoria: {} delante, prioridad alta-normal", c.exe);
        with_state(|st| {
            st.boosted = Some(c);
            persist(st);
        });
    }
}

fn background(fg_pid: u32, windir: &str, me: &str, now: Instant) {
    use windows::Win32::System::Threading::*;
    let audio = win::audio_pids();
    let my_session = win::session(std::process::id());
    let pids = win::pids();
    let alive: HashSet<u32> = pids.iter().copied().collect();
    // Los que ya no existen o han empezado a sonar: fuera de la lista (y deshecho si suena)
    let stale: Vec<Change> = with_state(|st| {
        st.since.retain(|pid, _| alive.contains(pid));
        let gone: Vec<u32> = st.eco.keys().copied().filter(|p| !alive.contains(p) || audio.contains(p)).collect();
        gone.into_iter().filter_map(|p| st.eco.remove(&p)).collect()
    });
    for c in &stale {
        if alive.contains(&c.pid) {
            undo(c);
        }
    }
    let mut changed = !stale.is_empty();
    for pid in pids {
        if with_state(|st| st.eco.contains_key(&pid) || st.boosted.as_ref().is_some_and(|b| b.pid == pid)) {
            continue;
        }
        let Some((info, created)) = info(pid, fg_pid, &audio, my_session, me) else { continue };
        if !eligible(&info, windir) {
            with_state(|st| st.since.remove(&pid));
            continue;
        }
        let since = with_state(|st| {
            let e = st.since.entry(pid).or_insert((created, now));
            if e.0 != created {
                *e = (created, now);
            }
            e.1
        });
        if !background_due(since, now) {
            continue;
        }
        let Some(p) = win::open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_INFORMATION | PROCESS_SET_QUOTA) else {
            with_state(|st| st.since.remove(&pid)); // sin permiso (p. ej. como administrador): no se insiste
            continue;
        };
        let original = win::priority(&p);
        if !win::set_eco(&p, Some(true)) {
            with_state(|st| st.since.remove(&pid));
            continue;
        }
        // Lo mismo que el Administrador de tareas: además, prioridad baja (solo si estaba en normal)
        let set = if original == NORMAL && win::set_priority(&p, IDLE) { IDLE } else { original };
        win::trim(&p);
        with_state(|st| {
            st.eco.insert(pid, Change { pid, created, exe: info.exe.clone(), original, set });
        });
        changed = true;
    }
    if changed {
        with_state(|st| persist(st));
    }
}

/// Deshace todo (al cerrar la consola o al apagar la gestión automática)
pub fn restore_all(app: &AppHandle) {
    let (changes, own_low) = with_state(|st| {
        let mut list: Vec<Change> = st.eco.drain().map(|(_, c)| c).collect();
        list.extend(st.boosted.take());
        st.since.clear();
        st.away_since = None;
        (list, std::mem::replace(&mut st.own_low, false))
    });
    changes.iter().for_each(undo);
    if own_low {
        own_restore();
    }
    for label in ["main", "navegador", "nube"] {
        set_webview_low(app, label, false);
    }
    if !changes.is_empty() {
        with_state(|st| persist(st));
    }
}

/// Al arrancar: deshace lo que quedara de una vez anterior (si la consola se cerró de golpe)
fn heal_previous() {
    let Ok(t) = std::fs::read_to_string(changes_file()) else { return };
    let list: Vec<Change> = serde_json::from_str(&t).unwrap_or_default();
    list.iter().for_each(undo);
    let _ = std::fs::remove_file(changes_file());
}

// ---------------------------------------------------------------- Arranque

unsafe extern "system" fn on_foreground(
    _: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
    _: u32,
    _: windows::Win32::Foundation::HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    if let Some(tx) = WAKE.get() {
        let _ = tx.lock().unwrap().send(());
    }
}

/// Avisa al gestor de que algo ha cambiado (más rápido que esperar a su próxima vuelta)
pub fn poke() {
    if let Some(tx) = WAKE.get() {
        let _ = tx.lock().unwrap().send(());
    }
}

pub fn start(app: AppHandle) {
    let (tx, rx): (Sender<()>, Receiver<()>) = channel();
    let _ = WAKE.set(Mutex::new(tx));
    heal_previous();
    // Windows avisa en cuanto cambia la ventana de delante (sin consultar sin parar)
    std::thread::spawn(|| unsafe {
        use windows::Win32::UI::Accessibility::SetWinEventHook;
        use windows::Win32::UI::WindowsAndMessaging::*;
        win::keep_thread_responsive();
        let _hook = SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(on_foreground), 0, 0, WINEVENT_OUTOFCONTEXT);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
    std::thread::spawn(move || {
        win::keep_thread_responsive();
        loop {
            // Con la consola esperando a aligerarse se mira más a menudo
            let pending = with_state(|s| s.away_since.is_some() && !s.own_low);
            let wait = if pending { Duration::from_millis(400) } else { Duration::from_secs(2) };
            match rx.recv_timeout(wait) {
                Ok(()) => while rx.try_recv().is_ok() {},
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => break,
            }
            tick(&app, Instant::now());
        }
    });
}

// ---------------------------------------------------------------- Liberar memoria

#[derive(Serialize, Clone, Copy, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Freed {
    /// Cuánto ha subido la memoria disponible
    pub freed_mb: u64,
    /// Cuánta memoria de trabajo han soltado los programas
    pub trimmed_mb: u64,
    pub processes: u32,
}

/// Vacía la memoria de trabajo de los programas en segundo plano. `keep`: el que estaba delante.
pub fn free_memory(keep: Option<u32>) -> Freed {
    use windows::Win32::System::Threading::*;
    let (_, before) = win::ram();
    let audio = win::audio_pids();
    let my_session = win::session(std::process::id());
    let (windir, me) = (windir(), own_exe());
    let fg = keep.unwrap_or(0);
    let mut out = Freed::default();
    let mut trimmed = 0u64;
    for pid in win::pids() {
        let Some((info, _)) = info(pid, fg, &audio, my_session, &me) else { continue };
        if !eligible(&info, &windir) {
            continue;
        }
        let Some(p) = win::open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA) else { continue };
        let ws = win::working_set(&p);
        if win::trim(&p) {
            trimmed += ws.saturating_sub(win::working_set(&p));
            out.processes += 1;
        }
    }
    let (_, after) = win::ram();
    out.freed_mb = after.saturating_sub(before) / MB;
    out.trimmed_mb = trimmed / MB;
    with_state(|s| s.last_freed = Some(out));
    out
}

/// Suelta la memoria de trabajo de la propia consola (para probar y para "aligerar")
pub fn trim_self() -> (u64, u64) {
    use windows::Win32::System::Threading::*;
    let Some(p) = win::open(std::process::id(), PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA) else { return (0, 0) };
    let before = win::working_set(&p);
    win::trim(&p);
    (before, win::working_set(&p))
}

// ---------------------------------------------------------------- Órdenes

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub total_mb: u64,
    pub used_mb: u64,
    pub last_freed_mb: Option<u64>,
    pub auto: bool,
    pub eco: bool,
    /// Programas en modo eficiencia ahora mismo
    pub eco_count: usize,
    /// Programa con prioridad subida (el juego de delante)
    pub boosted: Option<String>,
}

pub fn status() -> Status {
    let (total, avail) = win::ram();
    let s = settings();
    let (last, count, boosted) = with_state(|st| (st.last_freed.map(|f| f.freed_mb.max(f.trimmed_mb)), st.eco.len(), st.boosted.as_ref().map(|b| b.exe.clone())));
    Status { total_mb: total / MB, used_mb: total.saturating_sub(avail) / MB, last_freed_mb: last, auto: s.auto, eco: s.eco, eco_count: count, boosted }
}

#[tauri::command(async)]
pub fn memory_status() -> Status {
    status()
}

#[tauri::command(async)]
pub fn memory_settings_set(app: AppHandle, auto: bool, eco: bool) -> Status {
    save_settings(Settings { auto, eco });
    if !auto {
        restore_all(&app);
    }
    poke();
    status()
}

#[tauri::command(async)]
pub fn memory_free() -> Freed {
    free_memory(crate::multitask::previous_pid())
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDIR: &str = "c:\\windows\\";

    fn p(exe: &str, path: &str) -> ProcInfo {
        ProcInfo { pid: 1234, exe: exe.into(), path: path.into(), same_session: true, ..Default::default() }
    }

    #[test]
    fn quien_puede_ir_a_modo_eficiencia() {
        let discordia = p("notion.exe", "c:\\users\\ana\\appdata\\local\\programs\\notion\\notion.exe");
        assert!(eligible(&discordia, WINDIR));
        assert!(!eligible(&ProcInfo { foreground: true, ..discordia.clone() }, WINDIR), "lo de delante no");
        assert!(!eligible(&ProcInfo { audio: true, ..discordia.clone() }, WINDIR), "lo que suena no (Spotify)");
        assert!(!eligible(&ProcInfo { ours: true, ..discordia.clone() }, WINDIR), "la consola no");
        assert!(!eligible(&ProcInfo { same_session: false, ..discordia.clone() }, WINDIR), "servicios y otros usuarios no");
        assert!(!eligible(&ProcInfo { pid: 4, ..discordia.clone() }, WINDIR), "System no");
        assert!(!eligible(&p("explorer.exe", "c:\\windows\\explorer.exe"), WINDIR));
        assert!(!eligible(&p("searchhost.exe", "c:\\windows\\systemapps\\x\\searchhost.exe"), WINDIR), "carpeta de Windows");
        assert!(!eligible(&p("rustdesk.exe", "c:\\program files\\rustdesk\\rustdesk.exe"), WINDIR));
        assert!(!eligible(&p("steam.exe", "c:\\program files (x86)\\steam\\steam.exe"), WINDIR));
        assert!(!eligible(&p("msmpeng.exe", "c:\\programdata\\microsoft\\windows defender\\platform\\msmpeng.exe"), WINDIR));
        assert!(!eligible(&p("xtend.exe", "c:\\users\\ana\\appdata\\local\\couchdesk\\xtend\\xtend.exe"), WINDIR));
        assert!(!eligible(&p("secreto.exe", ""), WINDIR), "sin ruta legible no se toca");
    }

    #[test]
    fn subir_prioridad_al_juego_de_delante() {
        let juego = p("hades2.exe", "d:\\steamlibrary\\steamapps\\common\\hades ii\\hades2.exe");
        assert!(should_boost(&juego, false, NORMAL, WINDIR), "juego por su carpeta");
        assert!(!should_boost(&juego, false, ABOVE_NORMAL, WINDIR), "si ya eligió otra prioridad, se respeta");
        assert!(!should_boost(&juego, true, 0x80 /* HIGH */, WINDIR));
        let video = p("vlc.exe", "c:\\program files\\videolan\\vlc\\vlc.exe");
        assert!(!should_boost(&video, false, NORMAL, WINDIR), "ventana normal: nada");
        assert!(should_boost(&video, true, NORMAL, WINDIR), "a pantalla completa: sí");
        assert!(!should_boost(&p("explorer.exe", "c:\\windows\\explorer.exe"), true, NORMAL, WINDIR));
        assert!(!should_boost(&ProcInfo { ours: true, ..juego.clone() }, true, NORMAL, WINDIR), "la consola no");
        assert!(is_game_path("C:\\Users\\ana\\AppData\\Local\\CouchDesk\\emuladores\\pcsx2\\pcsx2-qt.exe"));
        assert!(is_game_path("C:\\XboxGames\\Forza\\Content\\forza.exe"));
        assert!(!is_game_path("C:\\Program Files\\Notepad++\\notepad++.exe"));
    }

    #[test]
    fn tiempos_y_deshacer() {
        let t0 = Instant::now();
        assert!(!background_due(t0, t0 + Duration::from_secs(29)));
        assert!(background_due(t0, t0 + Duration::from_secs(30)));
        assert!(!own_due(None, t0));
        assert!(!own_due(Some(t0), t0 + Duration::from_millis(1000)));
        assert!(own_due(Some(t0), t0 + Duration::from_millis(1600)));
        // Se devuelve la prioridad solo si sigue la nuestra
        assert_eq!(restore_priority(IDLE, IDLE, NORMAL), Some(NORMAL));
        assert_eq!(restore_priority(NORMAL, IDLE, NORMAL), None, "alguien ya la cambió");
        assert_eq!(restore_priority(ABOVE_NORMAL, ABOVE_NORMAL, NORMAL), Some(NORMAL));
        assert_eq!(restore_priority(NORMAL, NORMAL, NORMAL), None, "no cambiamos nada");
    }

    #[test]
    fn ajustes_por_defecto_y_viejos() {
        assert_eq!(Settings::default(), Settings { auto: true, eco: true });
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, Settings { auto: true, eco: true });
        let s: Settings = serde_json::from_str(r#"{"eco":false}"#).unwrap();
        assert_eq!(s, Settings { auto: true, eco: false });
    }

    /// En este PC: suelta la memoria de trabajo de este proceso de prueba y enseña la RAM
    #[test]
    #[ignore]
    fn liberar_memoria_propia_real() {
        // Algo de memoria tocada para que se note
        let v = vec![7u8; 200 * 1024 * 1024];
        let sum: u64 = v.iter().step_by(4096).map(|b| u64::from(*b)).sum();
        let (total, avail) = win::ram();
        println!("RAM: {} MB en total, {} MB libres", total / MB, avail / MB);
        let (before, after) = trim_self();
        println!("Este proceso: {} MB → {} MB (suma {sum})", before / MB, after / MB);
        assert!(after < before);
        drop(v);
        let audio = win::audio_pids();
        println!("Procesos con sonido activo: {audio:?}");
        let s = status();
        println!("Estado: {s:?}");
    }
}
