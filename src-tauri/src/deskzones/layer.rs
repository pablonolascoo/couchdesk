//! La capa de marcos detrás de los iconos y encima del fondo de pantalla, como hace Lively Wallpaper:
//! se pide a Progman (mensaje 0x052C) que cree su WorkerW y nuestra ventana pasa a ser hija de ese WorkerW.
//! - Windows clásico (hasta 23H2): WorkerW de nivel superior con SHELLDLL_DefView dentro → el WorkerW
//!   hermano que va justo después es el del fondo.
//! - Windows 11 24H2 en adelante: SHELLDLL_DefView vive dentro de Progman y el WorkerW del fondo es
//!   hijo de Progman (debajo de los iconos).
//! También: una ventana oculta que oye "TaskbarCreated" (explorer reiniciado) y cambios de pantalla.

use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MapWindowPoints, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, w};

/// Algo cambió fuera: explorer se reinició, cambió la resolución, el ppp o la barra de tareas
pub static EXPLORER_RESTARTED: AtomicBool = AtomicBool::new(false);
pub static DISPLAY_CHANGED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// WorkerW hermano tras el que lleva los iconos
    Classic,
    /// 24H2+: WorkerW hijo de Progman
    Raised,
}

impl Kind {
    pub fn code(self) -> &'static str {
        match self {
            Kind::Classic => "clasico",
            Kind::Raised => "24h2",
        }
    }
}

fn find(parent: Option<HWND>, after: Option<HWND>, class: windows::core::PCWSTR) -> Option<HWND> {
    unsafe { FindWindowExW(parent, after, class, None) }.ok().filter(|h| !h.is_invalid())
}

pub fn progman() -> Option<HWND> {
    unsafe { FindWindowW(w!("Progman"), None) }.ok().filter(|h| !h.is_invalid())
}

/// Ventana de nivel superior que contiene SHELLDLL_DefView (los iconos), sea Progman o un WorkerW
pub fn icons_host() -> Option<HWND> {
    unsafe extern "system" fn each(top: HWND, out: LPARAM) -> BOOL {
        if find(Some(top), None, w!("SHELLDLL_DefView")).is_some() {
            unsafe { *(out.0 as *mut HWND) = top };
            return BOOL(0);
        }
        BOOL(1)
    }
    let mut host = HWND::default();
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut host as *mut HWND as isize));
    }
    (!host.is_invalid()).then_some(host)
}

/// ¿Está el escritorio de explorer (con sus iconos)? Devuelve Progman como identificador:
/// si cambia, explorer se ha reiniciado.
pub fn desktop() -> Option<HWND> {
    let p = progman()?;
    icons_host().map(|_| p)
}

/// Qué disposición de ventanas hay ahora (sin tocar nada): para el informe y las pruebas
pub fn detect() -> Option<(Kind, HWND)> {
    let p = progman()?;
    let host = icons_host()?;
    if host != p {
        // Clásico: el WorkerW del fondo es el siguiente hermano del que lleva los iconos
        if let Some(ww) = find(None, Some(host), w!("WorkerW")) {
            return Some((Kind::Classic, ww));
        }
    }
    find(Some(p), None, w!("WorkerW")).map(|ww| (Kind::Raised, ww))
}

/// Pide a Progman su WorkerW del fondo y devuelve dónde colgar la capa
pub fn target() -> Result<(Kind, HWND), String> {
    let p = progman().ok_or("no-desktop")?;
    unsafe {
        let mut r = 0usize;
        // Lively: 0x052C con 0xD/0x1 (Windows 10/11); algunos Windows antiguos solo responden con 0/0
        let _ = SendMessageTimeoutW(p, 0x052C, WPARAM(0xD), LPARAM(0x1), SMTO_NORMAL, 1000, Some(&mut r));
        let _ = SendMessageTimeoutW(p, 0x052C, WPARAM(0), LPARAM(0), SMTO_NORMAL, 1000, Some(&mut r));
    }
    detect().ok_or_else(|| "no-workerw".into())
}

/// Monitor principal: (rectángulo entero, zona de trabajo sin la barra de tareas), en pantalla
pub fn primary_monitor() -> (RECT, RECT) {
    let mut mi = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    unsafe {
        let m = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let _ = GetMonitorInfoW(m, &mut mi);
    }
    (mi.rcMonitor, mi.rcWork)
}

/// Cuelga `child` del WorkerW, del tamaño del monitor principal, sin foco y sin recibir clics
pub fn attach(child: HWND, parent: HWND, monitor: RECT) -> Result<(), String> {
    unsafe {
        let ex = GetWindowLongPtrW(child, GWL_EXSTYLE);
        SetWindowLongPtrW(child, GWL_EXSTYLE, ex | (WS_EX_NOACTIVATE.0 | WS_EX_TRANSPARENT.0 | WS_EX_TOOLWINDOW.0) as isize);
        SetParent(child, Some(parent)).map_err(|e| format!("attach-failed: {e}"))?;
        // Coordenadas de pantalla → coordenadas dentro del WorkerW (que empieza en la esquina del escritorio virtual)
        let mut pts = [POINT { x: monitor.left, y: monitor.top }];
        MapWindowPoints(None, Some(parent), &mut pts);
        SetWindowPos(
            child,
            Some(HWND_TOP),
            pts[0].x,
            pts[0].y,
            monitor.right - monitor.left,
            monitor.bottom - monitor.top,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .map_err(|e| format!("attach-failed: {e}"))?;
    }
    Ok(())
}

/// ¿Sigue la capa colgada de ese WorkerW?
pub fn is_attached(child: HWND, parent: HWND) -> bool {
    unsafe { IsWindow(Some(child)).as_bool() && IsWindow(Some(parent)).as_bool() && GetParent(child).ok() == Some(parent) }
}

pub fn detach(child: HWND) {
    unsafe {
        if IsWindow(Some(child)).as_bool() {
            let _ = ShowWindow(child, SW_HIDE);
            let _ = SetParent(child, None);
        }
    }
}

// ---------- Oído: explorer reiniciado y cambios de pantalla ----------

static TASKBAR_CREATED: std::sync::OnceLock<u32> = std::sync::OnceLock::new();

unsafe extern "system" fn listener(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if Some(&msg) == TASKBAR_CREATED.get() {
        EXPLORER_RESTARTED.store(true, Ordering::Relaxed);
        return LRESULT(0);
    }
    match msg {
        WM_DISPLAYCHANGE | WM_DPICHANGED => DISPLAY_CHANGED.store(true, Ordering::Relaxed),
        // La barra de tareas cambió de sitio o de tamaño (SPI_SETWORKAREA = 0x2F)
        WM_SETTINGCHANGE if wp.0 == 0x2F => DISPLAY_CHANGED.store(true, Ordering::Relaxed),
        _ => {}
    }
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

/// Ventana oculta (de nivel superior: las de solo mensajes no oyen los avisos a todos)
pub fn create_listener() -> Option<HWND> {
    unsafe {
        let _ = TASKBAR_CREATED.set(RegisterWindowMessageW(w!("TaskbarCreated")));
        let inst = GetModuleHandleW(None).ok()?;
        let class = w!("CouchDeskDeskZones");
        let wc = WNDCLASSW { lpfnWndProc: Some(listener), hInstance: inst.into(), lpszClassName: class, ..Default::default() };
        RegisterClassW(&wc);
        CreateWindowExW(WS_EX_TOOLWINDOW, class, w!("CouchDesk"), WS_POPUP, 0, 0, 0, 0, None, None, Some(inst.into()), None).ok()
    }
}
