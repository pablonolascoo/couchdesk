//! Salidas de emergencia que ninguna web ni juego puede bloquear:
//! · Mantener Esc 1 segundo, o pulsar Esc 3 veces seguidas, estando en la consola, su navegador o la nube
//! · El botón central (Xbox) del mando, desde cualquier sitio
//! Todas llevan al Inicio de la consola y cierran el navegador o la nube si estaban abiertos.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::*;

static APP: OnceLock<AppHandle> = OnceLock::new();
static ESC_DOWN: Mutex<Option<Instant>> = Mutex::new(None);
static ESC_TAPS: Mutex<Vec<Instant>> = Mutex::new(Vec::new());
static FIRED: AtomicBool = AtomicBool::new(false);

fn trigger() {
    if let Some(app) = APP.get() {
        let app = app.clone();
        // Fuera del gancho de teclado: Windows exige que responda al instante
        std::thread::spawn(move || crate::browser::go_home(&app));
    }
}

/// ¿Está delante una ventana de la consola (Inicio, navegador o nube)?
fn ours_in_front() -> bool {
    unsafe {
        let mut title = [0u16; 64];
        let n = GetWindowTextW(GetForegroundWindow(), &mut title) as usize;
        matches!(String::from_utf16_lossy(&title[..n]).as_str(), "Consola" | "Navegador" | "Xbox Cloud Gaming")
    }
}

/// Decide si una tecla Esc (abajo/arriba) pide volver. Separado para poder probarlo.
fn on_esc(down: bool, now: Instant) -> bool {
    if !down {
        *ESC_DOWN.lock().unwrap() = None;
        FIRED.store(false, Ordering::Relaxed);
        return false;
    }
    let mut held = ESC_DOWN.lock().unwrap();
    match *held {
        // Primera pulsación: cuenta para "3 veces seguidas"
        None => {
            *held = Some(now);
            let mut taps = ESC_TAPS.lock().unwrap();
            taps.retain(|t| now.duration_since(*t) < Duration::from_millis(1200));
            taps.push(now);
            if taps.len() >= 3 {
                taps.clear();
                return true;
            }
            false
        }
        // Repeticiones mientras se mantiene: a partir de 0,9 s
        Some(t) => now.duration_since(t) > Duration::from_millis(900) && !FIRED.swap(true, Ordering::Relaxed),
    }
}

unsafe extern "system" fn keyboard_hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code >= 0 {
        let k = unsafe { &*(l.0 as *const KBDLLHOOKSTRUCT) };
        if k.vkCode == u32::from(VK_ESCAPE.0) {
            let down = matches!(w.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            if on_esc(down, Instant::now()) && ours_in_front() {
                trigger();
            }
        }
    }
    unsafe { CallNextHookEx(None, code, w, l) }
}

/// Botón central del mando de Xbox: XInput solo lo da por una función sin nombre (ordinal 100)
fn guide_watcher() {
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::Win32::UI::Input::XboxController::XINPUT_STATE;
    type GetStateEx = unsafe extern "system" fn(u32, *mut XINPUT_STATE) -> u32;
    let Ok(dll) = (unsafe { LoadLibraryW(windows::core::w!("xinput1_4.dll")) }) else { return };
    let Some(proc) = (unsafe { GetProcAddress(dll, windows::core::PCSTR(100 as *const u8)) }) else { return };
    let get: GetStateEx = unsafe { std::mem::transmute(proc) };
    let mut was = false;
    loop {
        std::thread::sleep(Duration::from_millis(80));
        let pressed = (0..4).any(|i| {
            let mut s = XINPUT_STATE::default();
            unsafe { get(i, &mut s) == 0 && s.Gamepad.wButtons.0 & 0x0400 != 0 }
        });
        if pressed && !was {
            trigger();
        }
        was = pressed;
    }
}

pub fn start(app: AppHandle) {
    let _ = APP.set(app);
    std::thread::spawn(|| unsafe {
        let Ok(_hook) = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), None, 0) else { return };
        // El gancho necesita un bucle de mensajes en su hilo
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
    std::thread::spawn(guide_watcher);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_mantenido_o_tres_veces() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        // Mantener: las repeticiones de la tecla llegan con el tiempo
        assert!(!on_esc(true, ms(0)));
        assert!(!on_esc(true, ms(500)));
        assert!(on_esc(true, ms(950)), "a los 0,9 s vuelve");
        assert!(!on_esc(true, ms(1200)), "solo una vez por pulsación");
        on_esc(false, ms(1300));
        // Tres toques rápidos
        ESC_TAPS.lock().unwrap().clear();
        for (i, at) in [3000, 3300].into_iter().enumerate() {
            assert!(!on_esc(true, ms(at)), "toque {i}");
            on_esc(false, ms(at + 60));
        }
        assert!(on_esc(true, ms(3600)), "al tercer toque vuelve");
        on_esc(false, ms(3650));
        // Dos toques y una pausa larga: no
        ESC_TAPS.lock().unwrap().clear();
        on_esc(true, ms(5000));
        on_esc(false, ms(5050));
        on_esc(true, ms(5300));
        on_esc(false, ms(5350));
        assert!(!on_esc(true, ms(7000)));
    }
}
