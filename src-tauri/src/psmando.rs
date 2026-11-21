//! Mandos de PlayStation (PS4 y PS5), leídos directamente de Windows (HID), sin drivers.
//! xTend solo entiende mandos de Xbox; esto hace lo mismo con los de PlayStation:
//! · Touchpad = ratón (un dedo mueve, dos dedos desplazan, pulsarlo hace clic)
//! · OPTIONS + SHARE cambia de modo: mando → ratón → teclado → mando (igual que START + BACK)
//! · Botón PS abre la consola si está cerrada
//! Funciona por cable (USB) y por Bluetooth. Va en un proceso aparte (`consola.exe --mando-ps`)
//! para seguir activo aunque cierres la consola y vayas al escritorio.

use crate::input;
use hidapi::{HidApi, HidDevice};
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::KeyboardAndMouse::*;

pub const SONY: u16 = 0x054C;
const DS4: &[u16] = &[0x05C4, 0x09CC, 0x0BA0];
const DS5: &[u16] = &[0x0CE6, 0x0DF2];
/// Puerto local que hace de "candado" para que solo haya un proceso de fondo
const LOCK_PORT: u16 = 47801; // lejos del mando móvil (7777) y de sus pruebas

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Ds4,
    Ds5,
}

/// Botones, con nombres de PlayStation
pub mod btn {
    pub const CROSS: u32 = 1;
    pub const CIRCLE: u32 = 1 << 1;
    pub const SQUARE: u32 = 1 << 2;
    pub const TRIANGLE: u32 = 1 << 3;
    pub const L1: u32 = 1 << 4;
    pub const R1: u32 = 1 << 5;
    pub const L2: u32 = 1 << 6;
    pub const R2: u32 = 1 << 7;
    pub const SHARE: u32 = 1 << 8;
    pub const OPTIONS: u32 = 1 << 9;
    pub const L3: u32 = 1 << 10;
    pub const R3: u32 = 1 << 11;
    pub const PS: u32 = 1 << 12;
    pub const PAD: u32 = 1 << 13;
    pub const UP: u32 = 1 << 14;
    pub const DOWN: u32 = 1 << 15;
    pub const LEFT: u32 = 1 << 16;
    pub const RIGHT: u32 = 1 << 17;
}

/// Lo que dice el mando en un instante. Sticks: 0..255, centro 128, arriba = 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub lx: u8,
    pub ly: u8,
    pub rx: u8,
    pub ry: u8,
    pub l2: u8,
    pub r2: u8,
    pub buttons: u32,
    /// Hasta dos dedos sobre el touchpad (x 0..1919, y 0..~1080)
    pub touch: [Option<(u16, u16)>; 2],
}

impl Default for State {
    fn default() -> Self {
        State { lx: 128, ly: 128, rx: 128, ry: 128, l2: 0, r2: 0, buttons: 0, touch: [None, None] }
    }
}

impl State {
    pub fn has(&self, b: u32) -> bool {
        self.buttons & b == b
    }
}

fn point(p: &[u8]) -> Option<(u16, u16)> {
    // Bit alto a 1 = ese dedo no está tocando
    if p.len() < 4 || p[0] & 0x80 != 0 {
        return None;
    }
    let x = u16::from(p[1]) | (u16::from(p[2] & 0x0F) << 8);
    let y = u16::from(p[2] >> 4) | (u16::from(p[3]) << 4);
    Some((x, y))
}

fn buttons(b0: u8, b1: u8, b2: u8) -> u32 {
    use btn::*;
    let mut b = 0;
    // Cruceta: 0 arriba, en sentido horario de 45 en 45 grados; 8 = suelta
    b |= match b0 & 0x0F {
        0 => UP,
        1 => UP | RIGHT,
        2 => RIGHT,
        3 => DOWN | RIGHT,
        4 => DOWN,
        5 => DOWN | LEFT,
        6 => LEFT,
        7 => UP | LEFT,
        _ => 0,
    };
    let bits = |byte: u8, list: &[(u8, u32)]| list.iter().filter(|(m, _)| byte & m != 0).fold(0, |a, (_, v)| a | v);
    b |= bits(b0, &[(0x10, SQUARE), (0x20, CROSS), (0x40, CIRCLE), (0x80, TRIANGLE)]);
    b |= bits(b1, &[(0x01, L1), (0x02, R1), (0x04, L2), (0x08, R2), (0x10, SHARE), (0x20, OPTIONS), (0x40, L3), (0x80, R3)]);
    b |= bits(b2, &[(0x01, PS), (0x02, PAD)]);
    b
}

/// Traduce un informe del mando. Admite PS4 y PS5, por cable y por Bluetooth.
pub fn parse(kind: Kind, r: &[u8]) -> Option<State> {
    let id = *r.first()?;
    // `o` = dónde empiezan los datos; `full` = informe completo (con touchpad)
    let (o, full) = match (kind, id) {
        (_, 0x01) if r.len() >= 64 => (1, true), // cable
        (Kind::Ds4, 0x11) if r.len() >= 46 => (3, true), // Bluetooth PS4
        (Kind::Ds5, 0x31) if r.len() >= 42 => (2, true), // Bluetooth PS5
        (_, 0x01) if r.len() >= 10 => (1, false), // Bluetooth básico (sin touchpad)
        _ => return None,
    };
    let d = &r[o..];
    let mut s = State { lx: d[0], ly: d[1], rx: d[2], ry: d[3], ..Default::default() };
    if kind == Kind::Ds5 && full {
        s.l2 = d[4];
        s.r2 = d[5];
        s.buttons = buttons(d[7], d[8], d[9]);
        s.touch = [point(&d[32..]), point(&d[36..])];
    } else {
        s.buttons = buttons(d[4], d[5], d[6]);
        s.l2 = d[7];
        s.r2 = d[8];
        if full {
            s.touch = [point(&d[34..]), point(&d[38..])];
        }
    }
    Some(s)
}

/// Un mando de PlayStation abierto
pub struct Pad {
    dev: HidDevice,
    pub kind: Kind,
    last: State,
}

impl Pad {
    /// Busca el primer mando de PlayStation conectado
    pub fn open(api: &HidApi) -> Option<Pad> {
        for d in api.device_list().filter(|d| d.vendor_id() == SONY) {
            let kind = if DS4.contains(&d.product_id()) {
                Kind::Ds4
            } else if DS5.contains(&d.product_id()) {
                Kind::Ds5
            } else {
                continue;
            };
            let Ok(dev) = d.open_device(api) else { continue };
            // Por Bluetooth el mando manda un informe corto hasta que se le pide la calibración;
            // entonces pasa al completo (con touchpad). Por cable no cambia nada.
            let mut f = [0u8; 64];
            f[0] = if kind == Kind::Ds4 { 0x02 } else { 0x05 };
            let _ = dev.get_feature_report(&mut f);
            return Some(Pad { dev, kind, last: State::default() });
        }
        None
    }

    /// Estado más reciente (espera hasta `wait_ms` si no hay nada nuevo).
    /// `None` = el mando se ha desconectado.
    pub fn poll(&mut self, wait_ms: i32) -> Option<State> {
        let mut buf = [0u8; 600];
        let mut wait = wait_ms;
        loop {
            match self.dev.read_timeout(&mut buf, wait) {
                Ok(0) => break,
                Ok(n) => {
                    if let Some(s) = parse(self.kind, &buf[..n]) {
                        self.last = s;
                    }
                }
                Err(_) => return None,
            }
            wait = 0;
        }
        Some(self.last)
    }
}

// ---------------- Proceso de fondo ----------------

/// Arranca el proceso de fondo (si ya hay uno, el nuevo se cierra solo)
pub fn spawn_helper() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).arg("--mando-ps").spawn();
    }
}

/// ¿Está la consola (o su navegador) delante?
fn console_in_front() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
    unsafe {
        let mut title = [0u16; 64];
        let n = GetWindowTextW(GetForegroundWindow(), &mut title) as usize;
        let t = String::from_utf16_lossy(&title[..n]);
        t == "Consola" || t == "Navegador"
    }
}

/// ¿Hay un juego (u otra app) a pantalla completa delante? Entonces el touchpad es del juego.
fn game_in_front() -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        let h = GetForegroundWindow();
        if h.is_invalid() {
            return false;
        }
        let mut class = [0u16; 64];
        let n = GetClassNameW(h, &mut class) as usize;
        let class = String::from_utf16_lossy(&class[..n]);
        if ["Progman", "WorkerW", "Shell_TrayWnd"].contains(&class.as_str()) {
            return false;
        }
        let mut title = [0u16; 64];
        let n = GetWindowTextW(h, &mut title) as usize;
        let title = String::from_utf16_lossy(&title[..n]);
        if title == "Consola" || title == "Navegador" {
            return false;
        }
        let mut r = RECT::default();
        if GetWindowRect(h, &mut r).is_err() {
            return false;
        }
        let (w, hgt) = input::screen_size();
        r.left <= 0 && r.top <= 0 && r.right >= w && r.bottom >= hgt
    }
}

/// Convierte lo que hace el mando en ratón y teclado
struct Mapper {
    mode: u8,
    prev: u32,
    /// Última posición de un dedo / altura media de dos dedos
    touch1: Option<(u16, u16)>,
    touch2: Option<u16>,
    fingers: usize,
    acc: (f32, f32),
    wheel: f32,
    left_down: bool,
    pad_down_right: bool,
    held: [Option<Instant>; 4],
    last: Instant,
    scale: f32,
}

impl Mapper {
    fn new() -> Self {
        let (w, _) = input::screen_size();
        Mapper {
            mode: 0,
            prev: 0,
            touch1: None,
            touch2: None,
            fingers: 0,
            acc: (0.0, 0.0),
            wheel: 0.0,
            left_down: false,
            pad_down_right: false,
            held: [None; 4],
            last: Instant::now(),
            scale: (w.max(800) as f32) / 1920.0,
        }
    }

    /// Suelta lo que esté pulsado (al cambiar de modo o desconectar el mando)
    fn release(&mut self) {
        if self.left_down {
            input::button(false, false);
            self.left_down = false;
        }
        self.touch1 = None;
        self.touch2 = None;
        self.held = [None; 4];
    }

    fn step(&mut self, s: &State) {
        use btn::*;
        let dt = self.last.elapsed().as_secs_f32().min(0.05);
        self.last = Instant::now();
        let pressed = s.buttons & !self.prev;
        let released = self.prev & !s.buttons;

        // OPTIONS + SHARE: cambiar de modo (igual que START + BACK en xTend)
        let combo = SHARE | OPTIONS;
        if s.has(combo) && self.prev & combo != combo {
            self.release();
            self.mode = (self.mode + 1) % 3;
            self.prev = s.buttons;
            return;
        }
        // Botón PS fuera de la consola: abrirla o traerla al frente (nunca se abre una segunda:
        // la consola solo admite una copia y la nueva despierta a la que ya está)
        if pressed & PS != 0 && !console_in_front() {
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe).spawn();
            }
        }

        if self.mode != 0 || !game_in_front() {
            self.touchpad(s, pressed, released);
        } else {
            self.touch1 = None;
            self.touch2 = None;
        }

        match self.mode {
            1 => self.mouse_mode(s, pressed, released, dt),
            2 => self.keyboard_mode(s, pressed, dt),
            _ => {}
        }
        self.prev = s.buttons;
    }

    fn touchpad(&mut self, s: &State, pressed: u32, released: u32) {
        let fingers: Vec<(u16, u16)> = s.touch.iter().flatten().copied().collect();
        if fingers.len() != self.fingers {
            // Al poner o quitar un dedo se empieza de cero (sin saltos)
            self.fingers = fingers.len();
            self.touch1 = None;
            self.touch2 = None;
            self.acc = (0.0, 0.0);
            self.wheel = 0.0;
        }
        match fingers.as_slice() {
            [a] => {
                if let Some(p) = self.touch1 {
                    let k = 0.9 * self.scale;
                    self.acc.0 += (f32::from(a.0) - f32::from(p.0)) * k;
                    self.acc.1 += (f32::from(a.1) - f32::from(p.1)) * k;
                    let (mx, my) = (self.acc.0.trunc(), self.acc.1.trunc());
                    self.acc.0 -= mx;
                    self.acc.1 -= my;
                    input::move_by(mx as i32, my as i32);
                }
                self.touch1 = Some(*a);
            }
            [a, b] => {
                // Dos dedos: desplazar como en un portátil (dedos arriba = la página sube)
                let y = (a.1 + b.1) / 2;
                if let Some(p) = self.touch2 {
                    self.wheel += (f32::from(p) - f32::from(y)) * 3.0;
                    let w = self.wheel.trunc();
                    self.wheel -= w;
                    input::scroll(0, w as i32);
                }
                self.touch2 = Some(y);
            }
            _ => {}
        }
        // Pulsar el touchpad: clic (mantener = arrastrar). Con dos dedos: clic derecho.
        if pressed & btn::PAD != 0 {
            self.pad_down_right = fingers.len() >= 2;
            input::button(self.pad_down_right, true);
        }
        if released & btn::PAD != 0 {
            input::button(self.pad_down_right, false);
        }
    }

    /// Flechas con repetición al mantener (cruceta o stick izquierdo)
    fn arrows(&mut self, s: &State) {
        let axis = |v: u8| (f32::from(v) - 128.0) / 127.0;
        let (lx, ly) = (axis(s.lx), axis(s.ly));
        let dirs = [
            (s.has(btn::UP) || ly < -0.6, VK_UP),
            (s.has(btn::DOWN) || ly > 0.6, VK_DOWN),
            (s.has(btn::LEFT) || lx < -0.6, VK_LEFT),
            (s.has(btn::RIGHT) || lx > 0.6, VK_RIGHT),
        ];
        for (i, (on, vk)) in dirs.into_iter().enumerate() {
            match (on, self.held[i]) {
                (false, _) => self.held[i] = None,
                (true, None) => {
                    input::combo(&[vk]);
                    self.held[i] = Some(Instant::now());
                }
                (true, Some(t)) if t.elapsed() > Duration::from_millis(400) => {
                    input::combo(&[vk]);
                    // siguiente repetición en ~110 ms
                    self.held[i] = Some(Instant::now() - Duration::from_millis(290));
                }
                _ => {}
            }
        }
    }

    fn scroll_stick(&mut self, s: &State, dt: f32) {
        let v = (f32::from(s.ry) - 128.0) / 127.0;
        if v.abs() > 0.15 {
            self.wheel += v * v.abs() * 2400.0 * dt;
            let w = self.wheel.trunc();
            self.wheel -= w;
            input::scroll(0, w as i32);
        }
    }

    fn mouse_mode(&mut self, s: &State, pressed: u32, released: u32, dt: f32) {
        use btn::*;
        // Stick izquierdo: mover el puntero (suave cerca del centro, rápido al fondo)
        let curve = |v: u8| {
            let x = (f32::from(v) - 128.0) / 127.0;
            if x.abs() < 0.12 { 0.0 } else { x * x.abs() }
        };
        let speed = 1700.0 * self.scale * dt;
        self.acc.0 += curve(s.lx) * speed;
        self.acc.1 += curve(s.ly) * speed;
        let (mx, my) = (self.acc.0.trunc(), self.acc.1.trunc());
        self.acc.0 -= mx;
        self.acc.1 -= my;
        input::move_by(mx as i32, my as i32);
        self.scroll_stick(s, dt);

        // ✕ o R2: clic izquierdo (mantener = arrastrar) · ○: clic derecho
        if pressed & (CROSS | R2) != 0 && !self.left_down {
            input::button(false, true);
            self.left_down = true;
        }
        if released & (CROSS | R2) != 0 && self.left_down && s.buttons & (CROSS | R2) == 0 {
            input::button(false, false);
            self.left_down = false;
        }
        if pressed & CIRCLE != 0 {
            input::click(true);
        }
        if pressed & SQUARE != 0 {
            input::combo(&[VK_ESCAPE]);
        }
        self.common(pressed);
        // Cruceta: flechas
        for (b, vk) in [(UP, VK_UP), (DOWN, VK_DOWN), (LEFT, VK_LEFT), (RIGHT, VK_RIGHT)] {
            if pressed & b != 0 {
                input::combo(&[vk]);
            }
        }
    }

    fn keyboard_mode(&mut self, s: &State, pressed: u32, dt: f32) {
        use btn::*;
        self.arrows(s);
        self.scroll_stick(s, dt);
        for (b, keys) in [
            (CROSS, &[VK_RETURN][..]),
            (CIRCLE, &[VK_ESCAPE][..]),
            (SQUARE, &[VK_BACK][..]),
            (R2, &[VK_SPACE][..]),
            (R1, &[VK_TAB][..]),
            (L2, &[VK_SHIFT, VK_TAB][..]),
        ] {
            if pressed & b != 0 {
                input::combo(keys);
            }
        }
        self.common(pressed);
    }

    /// Igual en modo ratón y teclado
    fn common(&mut self, pressed: u32) {
        use btn::*;
        // △: teclado en pantalla de Windows (Win + Ctrl + O)
        if pressed & TRIANGLE != 0 {
            input::combo(&[VK_LWIN, VK_CONTROL, VK_O]);
        }
        // L1 / R1 en modo ratón: atrás / adelante (en teclado R1 ya es Tab)
        if self.mode == 1 {
            if pressed & L1 != 0 {
                input::combo(&[VK_MENU, VK_LEFT]);
            }
            if pressed & R1 != 0 {
                input::combo(&[VK_MENU, VK_RIGHT]);
            }
        }
    }
}

/// Bucle del proceso de fondo: busca el mando, lo lee y lo traduce
pub fn run_helper() {
    let Ok(_lock) = std::net::TcpListener::bind(("127.0.0.1", LOCK_PORT)) else {
        return; // ya hay otro funcionando
    };
    let mut api: Option<HidApi> = None;
    let mut pad: Option<Pad> = None;
    let mut map = Mapper::new();
    loop {
        let Some(p) = pad.as_mut() else {
            match api.as_mut() {
                Some(a) => {
                    let _ = a.refresh_devices();
                }
                None => api = HidApi::new().ok(),
            }
            pad = api.as_ref().and_then(Pad::open);
            if pad.is_none() {
                std::thread::sleep(Duration::from_secs(2));
            }
            continue;
        };
        match p.poll(10) {
            Some(s) => map.step(&s),
            None => {
                map.release();
                pad = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(x: u16, y: u16) -> [u8; 4] {
        [0x01, (x & 0xFF) as u8, ((x >> 8) as u8 & 0x0F) | (((y & 0x0F) as u8) << 4), (y >> 4) as u8]
    }

    #[test]
    fn ps4_por_cable() {
        let mut r = [0u8; 64];
        r[0] = 0x01;
        r[1..5].copy_from_slice(&[0, 255, 128, 128]);
        r[5] = 0x20 | 2; // ✕ + cruceta derecha
        r[6] = 0x20 | 0x10; // OPTIONS + SHARE
        r[7] = 0x02; // touchpad pulsado
        r[9] = 200; // R2
        r[35..39].copy_from_slice(&touch(1500, 700));
        r[39] = 0x80; // segundo dedo no toca
        let s = parse(Kind::Ds4, &r).unwrap();
        assert_eq!((s.lx, s.ly, s.r2), (0, 255, 200));
        assert!(s.has(btn::CROSS | btn::RIGHT | btn::OPTIONS | btn::SHARE | btn::PAD));
        assert!(!s.has(btn::CIRCLE) && !s.has(btn::UP));
        assert_eq!(s.touch, [Some((1500, 700)), None]);
    }

    #[test]
    fn ps4_por_bluetooth() {
        let mut r = [0u8; 78];
        r[0] = 0x11;
        r[3..7].copy_from_slice(&[10, 20, 30, 40]);
        r[7] = 0x08 | 0x80; // cruceta suelta + △
        r[37..41].copy_from_slice(&touch(5, 900));
        r[41..45].copy_from_slice(&touch(1900, 100));
        let s = parse(Kind::Ds4, &r).unwrap();
        assert_eq!((s.lx, s.ly, s.rx, s.ry), (10, 20, 30, 40));
        assert_eq!(s.buttons, btn::TRIANGLE);
        assert_eq!(s.touch, [Some((5, 900)), Some((1900, 100))]);
    }

    #[test]
    fn ps5_por_cable_y_bluetooth() {
        let mut r = [0u8; 64];
        r[0] = 0x01;
        r[1] = 77;
        r[6] = 255; // R2
        r[8] = 0x40 | 0x08; // ○, cruceta suelta
        r[10] = 0x01; // PS
        r[33..37].copy_from_slice(&touch(960, 540));
        r[37] = 0x80;
        let s = parse(Kind::Ds5, &r).unwrap();
        assert_eq!((s.lx, s.r2), (77, 255));
        assert_eq!(s.buttons, btn::CIRCLE | btn::PS);
        assert_eq!(s.touch, [Some((960, 540)), None]);

        // Bluetooth: un byte más al principio
        let mut bt = [0u8; 78];
        bt[0] = 0x31;
        bt[2..65].copy_from_slice(&r[1..64]);
        assert_eq!(parse(Kind::Ds5, &bt).unwrap(), s);
    }

    /// Con un mando de verdad conectado: muestra lo que lee durante 12 s
    #[test]
    #[ignore]
    fn mando_real() {
        let api = HidApi::new().unwrap();
        let mut pad = Pad::open(&api).expect("no hay mando de PlayStation conectado");
        println!("Mando: {:?}", pad.kind);
        for _ in 0..3 {
            let mut buf = [0u8; 600];
            let n = pad.dev.read_timeout(&mut buf, 500).unwrap();
            println!("informe crudo ({n} bytes): {:02x?}", &buf[..n.min(48)]);
        }
        let t = Instant::now();
        let mut last = State::default();
        let mut reports = 0;
        while t.elapsed() < Duration::from_secs(12) {
            let s = pad.poll(50).expect("desconectado");
            reports += 1;
            if s.buttons != last.buttons || s.touch != last.touch || (s.lx as i16 - last.lx as i16).abs() > 20 {
                println!("{:5} ms  botones {:018b}  stick ({:3},{:3})  touch {:?}", t.elapsed().as_millis(), s.buttons, s.lx, s.ly, s.touch);
                last = s;
            }
        }
        println!("lecturas: {reports}");
    }

    #[test]
    fn bluetooth_basico_sin_touchpad() {
        let r = [0x01, 128, 128, 128, 128, 0x08 | 0x10, 0x01, 0x00, 0, 0];
        let s = parse(Kind::Ds5, &r).unwrap();
        assert_eq!(s.buttons, btn::SQUARE | btn::L1);
        assert_eq!(s.touch, [None, None]);
        assert!(parse(Kind::Ds4, &[0x05, 1, 2]).is_none());
    }
}
