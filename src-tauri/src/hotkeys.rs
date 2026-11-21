//! Salidas de emergencia que ninguna web ni juego puede bloquear:
//! · Mantener Esc 1 segundo, o pulsar Esc 3 veces seguidas, estando en la consola, su navegador o la nube
//! · El botón central (Xbox) del mando mantenido 0,8 s, desde cualquier sitio
//! Todas llevan al Inicio de la consola y cierran el navegador o la nube si estaban abiertos.
//! Multitarea (ventanas abiertas, cambiar, cerrar, liberar memoria), desde cualquier sitio:
//! · Botón Xbox pulsado y soltado (corto) · Ctrl + Alt + Inicio (otra vez = volver)
//! Además, el asistente flotante de Claude (encima de juegos y programas):
//! · Ctrl + Alt + Espacio, desde cualquier sitio (solo se queda con esa combinación)
//! · BACK (View) + Y del mando de Xbox mantenidos medio segundo

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_ESCAPE, VK_HOME, VK_SPACE};
use windows::Win32::UI::WindowsAndMessaging::*;

static APP: OnceLock<AppHandle> = OnceLock::new();
static ESC_DOWN: Mutex<Option<Instant>> = Mutex::new(None);
static ESC_TAPS: Mutex<Vec<Instant>> = Mutex::new(Vec::new());
static FIRED: AtomicBool = AtomicBool::new(false);
/// La barra espaciadora de Ctrl+Alt+Espacio está abajo (para quedarse también con su "soltar")
static COMBO_SPACE: AtomicBool = AtomicBool::new(false);
/// Igual con Inicio de Ctrl+Alt+Inicio
static COMBO_HOME: AtomicBool = AtomicBool::new(false);

/// ¿Es Ctrl + Alt + Espacio? Separado para poder probarlo.
pub fn is_assistant_combo(vk: u32, ctrl: bool, alt: bool) -> bool {
    vk == u32::from(VK_SPACE.0) && ctrl && alt
}

/// ¿Es Ctrl + Alt + Inicio (Multitarea)?
pub fn is_multitask_combo(vk: u32, ctrl: bool, alt: bool) -> bool {
    vk == u32::from(VK_HOME.0) && ctrl && alt
}

/// Qué pide el botón central (Xbox o PS)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GuideAction {
    /// Pulsado y soltado antes de 0,8 s: Multitarea
    Short,
    /// Mantenido 0,8 s: Inicio directo (sin esperar a soltar)
    Long,
}

pub const GUIDE_LONG: Duration = Duration::from_millis(800);

/// Pulsación del botón central: corta al soltar, larga en cuanto pasa el tiempo
#[derive(Default)]
pub struct GuidePress {
    since: Option<Instant>,
    long_fired: bool,
}

impl GuidePress {
    pub fn update(&mut self, pressed: bool, now: Instant) -> Option<GuideAction> {
        match (pressed, self.since) {
            (true, None) => {
                self.since = Some(now);
                self.long_fired = false;
                None
            }
            (true, Some(t)) if !self.long_fired && now.duration_since(t) >= GUIDE_LONG => {
                self.long_fired = true;
                Some(GuideAction::Long)
            }
            (false, Some(_)) => {
                self.since = None;
                (!std::mem::take(&mut self.long_fired)).then_some(GuideAction::Short)
            }
            _ => None,
        }
    }
}

/// Combinación de botones mantenida un tiempo: avisa una sola vez por pulsación
#[derive(Default)]
pub struct HoldCombo {
    since: Option<Instant>,
    fired: bool,
}

impl HoldCombo {
    pub fn update(&mut self, on: bool, now: Instant, hold: Duration) -> bool {
        if !on {
            *self = Self::default();
            return false;
        }
        let since = *self.since.get_or_insert(now);
        if !self.fired && now.duration_since(since) >= hold {
            self.fired = true;
            return true;
        }
        false
    }
}

/// BACK (View) + Y
pub const ASSISTANT_BUTTONS: u16 = 0x0020 | 0x8000;

fn toggle_assistant() {
    if let Some(app) = APP.get() {
        crate::claude::toggle_assistant(app);
    }
}

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
        matches!(String::from_utf16_lossy(&title[..n]).as_str(), "CouchDesk" | "Consola" | "Navegador" | "Xbox Cloud Gaming")
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
        let down = matches!(w.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        if k.vkCode == u32::from(VK_HOME.0) {
            if down {
                let ctrl = unsafe { GetAsyncKeyState(i32::from(VK_CONTROL.0)) } as u16 & 0x8000 != 0;
                let alt = k.flags.0 & LLKHF_ALTDOWN.0 != 0;
                if is_multitask_combo(k.vkCode, ctrl, alt) {
                    if !COMBO_HOME.swap(true, Ordering::Relaxed) {
                        crate::multitask::toggle_global();
                    }
                    return LRESULT(1);
                }
            } else if COMBO_HOME.swap(false, Ordering::Relaxed) {
                return LRESULT(1);
            }
        }
        if k.vkCode == u32::from(VK_SPACE.0) {
            if down {
                let ctrl = unsafe { GetAsyncKeyState(i32::from(VK_CONTROL.0)) } as u16 & 0x8000 != 0;
                let alt = k.flags.0 & LLKHF_ALTDOWN.0 != 0;
                if is_assistant_combo(k.vkCode, ctrl, alt) {
                    // Al mantenerla llegan repeticiones: solo la primera cuenta
                    if !COMBO_SPACE.swap(true, Ordering::Relaxed) {
                        toggle_assistant();
                    }
                    return LRESULT(1);
                }
            } else if COMBO_SPACE.swap(false, Ordering::Relaxed) {
                return LRESULT(1);
            }
        }
        if k.vkCode == u32::from(VK_ESCAPE.0) {
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
    // Responde al instante aunque la consola esté en modo eficiencia
    crate::memory::keep_thread_responsive();
    let mut guide = GuidePress::default();
    let mut assistant = HoldCombo::default();
    // Preguntar por un hueco sin mando es lento: los vacíos se miran solo una vez por segundo
    let mut connected = [true; 4];
    let mut last_scan = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(30));
        let rescan = last_scan.elapsed() > Duration::from_secs(1);
        if rescan {
            last_scan = Instant::now();
        }
        let mut buttons = 0u16;
        for i in 0..4 {
            if !connected[i] && !rescan {
                continue;
            }
            let mut s = XINPUT_STATE::default();
            connected[i] = unsafe { get(i as u32, &mut s) } == 0;
            if connected[i] {
                buttons |= s.Gamepad.wButtons.0;
            }
        }
        match guide.update(buttons & 0x0400 != 0, Instant::now()) {
            Some(GuideAction::Short) => crate::multitask::toggle_global(),
            Some(GuideAction::Long) => trigger(),
            None => {}
        }
        if assistant.update(buttons & ASSISTANT_BUTTONS == ASSISTANT_BUTTONS, Instant::now(), Duration::from_millis(500)) {
            toggle_assistant();
        }
    }
}

pub fn start(app: AppHandle) {
    let _ = APP.set(app);
    std::thread::spawn(|| unsafe {
        crate::memory::keep_thread_responsive();
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
    fn combinaciones_del_asistente() {
        let space = u32::from(VK_SPACE.0);
        assert!(is_assistant_combo(space, true, true));
        assert!(!is_assistant_combo(space, true, false));
        assert!(!is_assistant_combo(space, false, true));
        assert!(!is_assistant_combo(u32::from(VK_ESCAPE.0), true, true));
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let hold = Duration::from_millis(500);
        let mut c = HoldCombo::default();
        assert!(!c.update(true, ms(0), hold));
        assert!(!c.update(true, ms(400), hold));
        assert!(c.update(true, ms(520), hold), "a los 0,5 s");
        assert!(!c.update(true, ms(900), hold), "una vez por pulsación");
        assert!(!c.update(false, ms(950), hold));
        assert!(!c.update(true, ms(1000), hold));
        assert!(!c.update(false, ms(1200), hold), "soltar antes de tiempo no cuenta");
        assert!(!c.update(true, ms(1300), hold));
        assert!(c.update(true, ms(1850), hold));
    }

    #[test]
    fn boton_central_corto_y_largo() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let mut g = GuidePress::default();
        // Corto: al soltar
        assert_eq!(g.update(true, ms(0)), None);
        assert_eq!(g.update(true, ms(200)), None);
        assert_eq!(g.update(false, ms(260)), Some(GuideAction::Short));
        assert_eq!(g.update(false, ms(300)), None, "soltado: nada más");
        // Largo: a los 0,8 s sin esperar a soltar, y soltar después no cuenta como corto
        assert_eq!(g.update(true, ms(1000)), None);
        assert_eq!(g.update(true, ms(1700)), None);
        assert_eq!(g.update(true, ms(1810)), Some(GuideAction::Long));
        assert_eq!(g.update(true, ms(2500)), None, "una vez por pulsación");
        assert_eq!(g.update(false, ms(2600)), None);
        // Y vuelve a funcionar el corto
        assert_eq!(g.update(true, ms(3000)), None);
        assert_eq!(g.update(false, ms(3100)), Some(GuideAction::Short));
        // Ctrl + Alt + Inicio
        let home = u32::from(VK_HOME.0);
        assert!(is_multitask_combo(home, true, true));
        assert!(!is_multitask_combo(home, true, false));
        assert!(!is_multitask_combo(home, false, true));
        assert!(!is_multitask_combo(u32::from(VK_SPACE.0), true, true));
    }

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
