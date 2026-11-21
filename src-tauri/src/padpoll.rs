//! Mando sin retraso para la ventana principal.
//!
//! El navegador interno (WebView2) lee los mandos cada ~16 ms y la página solo los ve en el
//! siguiente fotograma: hasta ~33 ms desde que pulsas. Aquí un hilo propio, con prioridad de
//! juego (MMCSS "Games"), lee los mandos de Xbox (XInput, ranuras 0–3) y los de PlayStation (HID)
//! cada 2 ms y, en cuanto algo cambia, avisa a la ventana con el evento "pad".
//!
//! Evento "pad" (a la ventana "main"), con la distribución ESTÁNDAR de la API Gamepad:
//!   A=0 B=1 X=2 Y=3 LB=4 RB=5 LT=6 RT=7 Back=8 Start=9 LS=10 RS=11 ↑=12 ↓=13 ←=14 →=15 Guía=16 (touchpad PS=17)
//!   { pad: "xinput-0" | "ps-054c-0ce6-…", slot, kind: "xbox"|"ps", buttons: máscara de bits,
//!     axes: [lx, ly, rx, ry] en pasos de 0,05 (arriba = negativo), sync, connected }
//!   `sync = true`: estado al volver a la ventana (lo pulsado ya estaba pulsado: no cuenta como pulsación).
//!
//! Solo trabaja con la ventana delante; si no, duerme. Medir: variable de entorno
//! `COUCHDESK_PAD_TIMING=1` escribe en la consola cuánto tarda de la lectura al aviso (µs).

use crate::psmando;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use windows::Win32::UI::Input::XboxController::*;

static FOCUSED: AtomicBool = AtomicBool::new(false);
static STARTED: AtomicBool = AtomicBool::new(false);

/// lib.rs lo llama al ganar o perder el foco la ventana principal
pub fn set_focused(on: bool) {
    FOCUSED.store(on, Ordering::Relaxed);
}

/// Cada cuánto se lee el mando con la ventana delante
const POLL: Duration = Duration::from_millis(2);
/// Sin mandos conectados, o con la ventana detrás: casi sin gastar
const IDLE: Duration = Duration::from_millis(8);
/// Una ranura vacía de XInput es lenta de preguntar: se mira una vez por segundo
const PROBE_EMPTY: Duration = Duration::from_secs(1);
const PS_SEARCH: Duration = Duration::from_secs(2);
/// Gatillo pulsado (como XINPUT_GAMEPAD_TRIGGER_THRESHOLD)
const TRIGGER: u8 = 30;
/// Pasos por unidad de eje: 20 → 0,05 (los umbrales de la consola son 0,4 / 0,6 / 0,8)
const STEPS: f32 = 20.0;
/// Zona muerta mínima para que el temblor del stick no mande avisos
const AXIS_DEAD: f32 = 0.1;

// ---------------- Traducción a la distribución estándar ----------------

/// Botón de Xbox (bits de XInput) → índice estándar
const XINPUT_MAP: [(u16, u32); 15] = [
    (0x1000, 0),  // A
    (0x2000, 1),  // B
    (0x4000, 2),  // X
    (0x8000, 3),  // Y
    (0x0100, 4),  // LB
    (0x0200, 5),  // RB
    (0x0020, 8),  // Back / View
    (0x0010, 9),  // Start / Menú
    (0x0040, 10), // L3
    (0x0080, 11), // R3
    (0x0001, 12), // ↑
    (0x0002, 13), // ↓
    (0x0004, 14), // ←
    (0x0008, 15), // →
    (0x0400, 16), // Guía (solo con XInputGetStateEx)
];

/// Botones de un mando de Xbox como máscara estándar (gatillos 6 y 7 incluidos)
pub fn xinput_to_standard(buttons: u16, lt: u8, rt: u8) -> u32 {
    let mut out = XINPUT_MAP.iter().filter(|(m, _)| buttons & m != 0).fold(0u32, |a, (_, i)| a | 1 << i);
    if lt > TRIGGER {
        out |= 1 << 6;
    }
    if rt > TRIGGER {
        out |= 1 << 7;
    }
    out
}

/// Botones de un mando de PlayStation como máscara estándar (igual que Chromium)
pub fn ps_to_standard(s: &psmando::State) -> u32 {
    use psmando::btn::*;
    let map: [(u32, u32); 18] = [
        (CROSS, 0), (CIRCLE, 1), (SQUARE, 2), (TRIANGLE, 3), (L1, 4), (R1, 5), (L2, 6), (R2, 7),
        (SHARE, 8), (OPTIONS, 9), (L3, 10), (R3, 11), (UP, 12), (DOWN, 13), (LEFT, 14), (RIGHT, 15),
        (PS, 16), (PAD, 17),
    ];
    let mut out = map.iter().filter(|(m, _)| s.buttons & m != 0).fold(0u32, |a, (_, i)| a | 1 << i);
    if s.l2 > TRIGGER {
        out |= 1 << 6;
    }
    if s.r2 > TRIGGER {
        out |= 1 << 7;
    }
    out
}

/// Eje −1..1 → paso entero (−20..20), con una zona muerta pequeña
pub fn quantize(v: f32) -> i8 {
    let v = v.clamp(-1.0, 1.0);
    if v.abs() < AXIS_DEAD { 0 } else { (v * STEPS).round() as i8 }
}

/// Stick de XInput (arriba = positivo) → estándar (arriba = negativo)
fn xaxis(v: i16) -> f32 {
    f32::from(v) / 32767.0
}

/// Lo que importa de un mando en un instante; si cambia, se avisa
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Snap {
    pub buttons: u32,
    pub axes: [i8; 4],
}

impl Snap {
    pub fn from_xinput(g: &XINPUT_GAMEPAD) -> Snap {
        Snap {
            buttons: xinput_to_standard(g.wButtons.0, g.bLeftTrigger, g.bRightTrigger),
            axes: [quantize(xaxis(g.sThumbLX)), quantize(-xaxis(g.sThumbLY)), quantize(xaxis(g.sThumbRX)), quantize(-xaxis(g.sThumbRY))],
        }
    }

    pub fn from_ps(s: &psmando::State) -> Snap {
        let a = |v: u8| quantize((f32::from(v) - 128.0) / 127.0);
        Snap { buttons: ps_to_standard(s), axes: [a(s.lx), a(s.ly), a(s.rx), a(s.ry)] }
    }
}

/// ¿Hay que avisar? Al volver a la ventana, siempre (para ponerse al día)
pub fn changed(prev: Option<&Snap>, now: &Snap, sync: bool) -> bool {
    sync || prev != Some(now)
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PadEvent {
    pub pad: String,
    pub slot: u8,
    pub kind: &'static str,
    pub buttons: u32,
    pub axes: [f32; 4],
    pub sync: bool,
    pub connected: bool,
}

impl PadEvent {
    pub fn new(pad: &str, slot: u8, kind: &'static str, s: &Snap, sync: bool, connected: bool) -> Self {
        PadEvent {
            pad: pad.to_string(),
            slot,
            kind,
            buttons: s.buttons,
            axes: s.axes.map(|q| f32::from(q) / STEPS),
            sync,
            connected,
        }
    }
}

// ---------------- Lectura ----------------

type GetStateFn = unsafe extern "system" fn(u32, *mut XINPUT_STATE) -> u32;

/// XInputGetStateEx (ordinal 100) también ve el botón Guía; si no está, la normal
fn xinput_reader() -> GetStateFn {
    unsafe extern "system" fn plain(i: u32, s: *mut XINPUT_STATE) -> u32 {
        unsafe { XInputGetState(i, s) }
    }
    unsafe {
        use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
        let Ok(dll) = LoadLibraryW(windows::core::w!("xinput1_4.dll")) else { return plain };
        match GetProcAddress(dll, windows::core::PCSTR(100 as *const u8)) {
            Some(p) => std::mem::transmute::<_, GetStateFn>(p),
            None => plain,
        }
    }
}

const DS4: &[u16] = &[0x05C4, 0x09CC, 0x0BA0];
const DS5: &[u16] = &[0x0CE6, 0x0DF2];

/// Mando de PlayStation abierto aquí (además del proceso de fondo: Windows da a cada uno su copia)
struct Ps {
    dev: hidapi::HidDevice,
    kind: psmando::Kind,
    id: String,
    last: psmando::State,
}

/// Nombre estable del mando: con el fabricante y el modelo (como lo ve Chromium) y su número de serie
pub fn ps_id(product: u16, serial: Option<&str>) -> String {
    let serial: String = serial.unwrap_or("").chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
    if serial.is_empty() { format!("ps-054c-{product:04x}") } else { format!("ps-054c-{product:04x}-{serial}") }
}

fn open_ps(api: &hidapi::HidApi) -> Option<Ps> {
    for d in api.device_list().filter(|d| d.vendor_id() == psmando::SONY) {
        let pid = d.product_id();
        let kind = if DS4.contains(&pid) {
            psmando::Kind::Ds4
        } else if DS5.contains(&pid) {
            psmando::Kind::Ds5
        } else {
            continue;
        };
        let Ok(dev) = d.open_device(api) else { continue };
        // Por Bluetooth pide el informe completo (como psmando)
        let mut f = [0u8; 64];
        f[0] = if kind == psmando::Kind::Ds4 { 0x02 } else { 0x05 };
        let _ = dev.get_feature_report(&mut f);
        let _ = dev.set_blocking_mode(false);
        return Some(Ps { dev, kind, id: ps_id(pid, d.serial_number()), last: psmando::State::default() });
    }
    None
}

impl Ps {
    /// Último estado; `None` = desconectado
    fn read(&mut self) -> Option<psmando::State> {
        let mut buf = [0u8; 128];
        loop {
            match self.dev.read_timeout(&mut buf, 0) {
                Ok(0) => return Some(self.last),
                Ok(n) => {
                    if let Some(s) = psmando::parse(self.kind, &buf[..n]) {
                        self.last = s;
                    }
                }
                Err(_) => return None,
            }
        }
    }
}

/// Reloj de Windows a 1 ms mientras vive (para dormir 2 ms de verdad) y prioridad de juego
struct Precise {
    mmcss: Option<windows::Win32::Foundation::HANDLE>,
}

impl Precise {
    fn new() -> Self {
        use windows::Win32::Media::timeBeginPeriod;
        use windows::Win32::System::Threading::*;
        unsafe {
            timeBeginPeriod(1);
            let mut index = 0u32;
            let mmcss = AvSetMmThreadCharacteristicsW(windows::core::w!("Games"), &mut index).ok();
            if mmcss.is_none() {
                let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
            }
            Precise { mmcss }
        }
    }
}

impl Drop for Precise {
    fn drop(&mut self) {
        use windows::Win32::Media::timeEndPeriod;
        unsafe {
            if let Some(h) = self.mmcss {
                let _ = windows::Win32::System::Threading::AvRevertMmThreadCharacteristics(h);
            }
            timeEndPeriod(1);
        }
    }
}

/// Tiempos de lectura → aviso, para medir
#[derive(Default)]
struct Timing {
    on: bool,
    n: u32,
    sum: u128,
    max: u128,
}

impl Timing {
    fn add(&mut self, us: u128) {
        if !self.on {
            return;
        }
        self.n += 1;
        self.sum += us;
        self.max = self.max.max(us);
        if self.n == 50 {
            eprintln!("[pad] lectura→aviso: media {} µs, máx {} µs (50 avisos)", self.sum / 50, self.max);
            *self = Timing { on: true, ..Default::default() };
        }
    }
}

fn run(app: AppHandle) {
    let get = xinput_reader();
    let mut xs: [Option<Snap>; 4] = [None; 4];
    let mut probe = [Instant::now(); 4];
    let mut api: Option<hidapi::HidApi> = None;
    let mut ps: Option<Ps> = None;
    let mut ps_snap: Option<Snap> = None;
    let mut ps_search: Option<Instant> = None;
    let mut was_focused = false;
    let mut timing = Timing { on: std::env::var_os("COUCHDESK_PAD_TIMING").is_some(), ..Default::default() };
    let mut precise: Option<Precise> = None;

    let emit = |ev: PadEvent, at: Instant, timing: &mut Timing| {
        let _ = app.emit_to("main", "pad", ev);
        timing.add(at.elapsed().as_micros());
    };

    loop {
        if !FOCUSED.load(Ordering::Relaxed) {
            // Detrás: sin reloj fino ni prioridad, y nada de leer
            was_focused = false;
            precise = None;
            std::thread::sleep(IDLE);
            continue;
        }
        if precise.is_none() {
            precise = Some(Precise::new());
        }
        let sync = !was_focused;
        was_focused = true;
        let now = Instant::now();

        for slot in 0..4u8 {
            let i = usize::from(slot);
            if xs[i].is_none() && !sync && now < probe[i] {
                continue;
            }
            let mut st = XINPUT_STATE::default();
            if unsafe { get(u32::from(slot), &mut st) } == 0 {
                let at = Instant::now();
                let snap = Snap::from_xinput(&st.Gamepad);
                if changed(xs[i].as_ref(), &snap, sync) {
                    emit(PadEvent::new(&format!("xinput-{slot}"), slot, "xbox", &snap, sync, true), at, &mut timing);
                }
                xs[i] = Some(snap);
            } else {
                if xs[i].take().is_some() {
                    emit(PadEvent::new(&format!("xinput-{slot}"), slot, "xbox", &Snap::default(), false, false), now, &mut timing);
                }
                probe[i] = now + PROBE_EMPTY;
            }
        }

        if ps.is_none() && ps_search.is_none_or(|t| t.elapsed() > PS_SEARCH) {
            ps_search = Some(Instant::now());
            match api.as_mut() {
                Some(a) => {
                    let _ = a.refresh_devices();
                }
                None => api = hidapi::HidApi::new().ok(),
            }
            ps = api.as_ref().and_then(open_ps);
        }
        if let Some(p) = ps.as_mut() {
            match p.read() {
                Some(s) => {
                    let at = Instant::now();
                    let snap = Snap::from_ps(&s);
                    if changed(ps_snap.as_ref(), &snap, sync) {
                        emit(PadEvent::new(&p.id, 4, "ps", &snap, sync, true), at, &mut timing);
                    }
                    ps_snap = Some(snap);
                }
                None => {
                    emit(PadEvent::new(&p.id, 4, "ps", &Snap::default(), false, false), now, &mut timing);
                    ps = None;
                    ps_snap = None;
                }
            }
        }

        let any = xs.iter().any(Option::is_some) || ps.is_some();
        std::thread::sleep(if any { POLL } else { IDLE });
    }
}

/// Arranca el hilo (una sola vez)
pub fn start(app: AppHandle) {
    if STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    std::thread::Builder::new()
        .name("mando".into())
        .spawn(move || {
            // El hilo del mando nunca entra en «modo eficiencia», aunque CouchDesk esté detrás
            crate::memory::keep_thread_responsive();
            run(app)
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xbox_a_estandar() {
        // A + View + Start + cruceta arriba + Guía
        let b = xinput_to_standard(0x1000 | 0x0020 | 0x0010 | 0x0001 | 0x0400, 0, 0);
        assert_eq!(b, 1 << 0 | 1 << 8 | 1 << 9 | 1 << 12 | 1 << 16);
        // LB, RB, L3, R3, X, Y, B
        assert_eq!(xinput_to_standard(0x0100 | 0x0200, 0, 0), 1 << 4 | 1 << 5);
        assert_eq!(xinput_to_standard(0x0040 | 0x0080, 0, 0), 1 << 10 | 1 << 11);
        assert_eq!(xinput_to_standard(0x2000 | 0x4000 | 0x8000, 0, 0), 1 << 1 | 1 << 2 | 1 << 3);
        // Cruceta completa
        assert_eq!(xinput_to_standard(0x0002 | 0x0004 | 0x0008, 0, 0), 1 << 13 | 1 << 14 | 1 << 15);
        // Gatillos: solo pasado el umbral
        assert_eq!(xinput_to_standard(0, 30, 31), 1 << 7);
        assert_eq!(xinput_to_standard(0, 255, 0), 1 << 6);
    }

    #[test]
    fn playstation_a_estandar() {
        use psmando::btn::*;
        let s = psmando::State { buttons: CROSS | CIRCLE | OPTIONS | SHARE | PS | PAD | LEFT, r2: 200, ..Default::default() };
        assert_eq!(ps_to_standard(&s), 1 << 0 | 1 << 1 | 1 << 9 | 1 << 8 | 1 << 16 | 1 << 17 | 1 << 14 | 1 << 7);
    }

    #[test]
    fn ejes_en_pasos() {
        assert_eq!(quantize(0.0), 0);
        assert_eq!(quantize(0.09), 0, "temblor del stick");
        assert_eq!(quantize(0.6), 12);
        assert_eq!(quantize(-1.0), -20);
        assert_eq!(quantize(1.7), 20, "se recorta");
        assert_eq!(quantize(0.61), quantize(0.62), "cambios mínimos no avisan");
        // Xbox: arriba en el stick = negativo en estándar
        let g = XINPUT_GAMEPAD { sThumbLY: 32767, sThumbLX: -32768, ..Default::default() };
        let s = Snap::from_xinput(&g);
        assert_eq!(s.axes[0], -20);
        assert_eq!(s.axes[1], -20);
        // PlayStation: 0 arriba → −1, 128 centro → 0
        let p = psmando::State { lx: 255, ly: 0, ..Default::default() };
        let s = Snap::from_ps(&p);
        assert_eq!((s.axes[0], s.axes[1], s.axes[2]), (20, -20, 0));
    }

    #[test]
    fn solo_avisa_si_cambia() {
        let a = Snap { buttons: 1, axes: [0; 4] };
        let b = Snap { buttons: 1, axes: [0, 12, 0, 0] };
        assert!(changed(None, &a, false), "mando nuevo");
        assert!(!changed(Some(&a), &a, false), "igual: nada");
        assert!(changed(Some(&a), &b, false), "eje cruza un paso");
        assert!(changed(Some(&a), &a, true), "al volver a la ventana, siempre");
        let ev = PadEvent::new("xinput-1", 1, "xbox", &b, false, true);
        assert_eq!(ev.axes, [0.0, 0.6, 0.0, 0.0]);
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains(r#""pad":"xinput-1""#) && json.contains(r#""buttons":1"#) && json.contains(r#""connected":true"#));
    }

    #[test]
    fn nombre_de_mando_ps() {
        assert_eq!(ps_id(0x0CE6, None), "ps-054c-0ce6");
        assert_eq!(ps_id(0x09CC, Some("A4:AE:12:34:56:78")), "ps-054c-09cc-a4ae12345678");
    }

    /// Coste real de una vuelta del bucle (sin la ventana): leer las 4 ranuras y preparar el aviso
    #[test]
    #[ignore]
    fn coste_de_una_lectura() {
        let get = xinput_reader();
        let _p = Precise::new();
        let mut worst = 0u128;
        let mut sum = 0u128;
        let mut sleep_sum = 0u128;
        for _ in 0..500 {
            let t = Instant::now();
            for slot in 0..4 {
                let mut st = XINPUT_STATE::default();
                if unsafe { get(slot, &mut st) } == 0 {
                    let s = Snap::from_xinput(&st.Gamepad);
                    let _ = serde_json::to_string(&PadEvent::new("xinput-0", 0, "xbox", &s, false, true));
                }
            }
            let us = t.elapsed().as_micros();
            worst = worst.max(us);
            sum += us;
            let t = Instant::now();
            std::thread::sleep(POLL);
            sleep_sum += t.elapsed().as_micros();
        }
        println!("lectura de 4 ranuras: media {} µs, peor {} µs; dormir 2 ms dura {} µs de media", sum / 500, worst, sleep_sum / 500);
    }
}
