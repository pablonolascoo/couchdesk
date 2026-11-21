//! Escritorio organizado (como Stardock Fences): los iconos del escritorio de Windows se ordenan
//! solos en zonas (Juegos, Aplicaciones, Carpetas…) y detrás se dibujan unos marcos suaves.
//! - Solo con APIs oficiales: los iconos se mueven con IFolderView2 (shell.rs); los marcos son
//!   una ventana nuestra colgada del WorkerW del fondo (layer.rs). Nunca se toca ni se mueve un archivo.
//! - Funciona cuando está el escritorio de explorer (modo normal o "Ir al escritorio"); si explorer
//!   se reinicia o cambia la pantalla, se vuelve a colocar todo.
//! - Antes de la primera vez se guarda cómo estaba (escritorio-original.json) para poder volver.
//! Un único hilo STA hace todo el trabajo de Windows: vigila cada 2 s y atiende las órdenes.

pub mod classify;
pub mod config;
pub mod layer;
pub mod layout;
pub mod shell;

use config::Config;
use layout::{Layout, Metrics, Rect};
use serde::Serialize;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{FVM_ICON, FWF_AUTOARRANGE, FWF_SNAPTOGRID};
use windows::Win32::UI::WindowsAndMessaging::*;

pub const LABEL: &str = "capa-escritorio";
pub const EVENT: &str = "deskzones-frames";
const TICK_MS: u32 = 2000;
const WM_WAKE: u32 = WM_APP + 41;

// ---------------- Datos que ve la página ----------------

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub id: String,
    /// "" = nombre de serie (lo traduce la página)
    pub name: String,
    pub count: usize,
    pub cols: usize,
    pub rect: Rect,
}

/// Lo que dibuja la capa y la vista previa (todo en píxeles de pantalla)
#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Frames {
    pub monitor: Rect,
    pub work: Rect,
    pub cell_w: i32,
    pub cell_h: i32,
    pub pad: i32,
    pub header: i32,
    pub fits: bool,
    pub zones: Vec<Frame>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub enabled: bool,
    /// El escritorio de Windows está abierto
    pub desktop_visible: bool,
    /// "off" | "attached" | "failed"
    pub layer: String,
    /// Código del fallo de la capa: no-desktop, no-workerw, attach-failed, window-failed
    pub layer_error: Option<String>,
    /// "clasico" | "24h2"
    pub layer_kind: Option<String>,
    pub organized_at: Option<u64>,
    pub item_count: usize,
    pub fits: bool,
    pub has_backup: bool,
    pub restore_pending: bool,
    pub last_error: Option<String>,
}

static STATUS: Mutex<Option<Status>> = Mutex::new(None);
static FRAMES: Mutex<Option<Frames>> = Mutex::new(None);
static CONFIG: Mutex<Option<Config>> = Mutex::new(None);
/// Hueco real de icono por tamaño (aprendido al ordenar), para que la vista previa acierte
static SPACING: Mutex<Vec<(u32, i32, i32)>> = Mutex::new(Vec::new());
static WAKE_HWND: AtomicIsize = AtomicIsize::new(0);
static SENDER: Mutex<Option<Sender<Cmd>>> = Mutex::new(None);

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn config() -> Config {
    CONFIG.lock().unwrap().get_or_insert_with(config::load).clone()
}

fn status() -> Status {
    let mut s = STATUS.lock().unwrap().clone().unwrap_or_else(|| Status { layer: "off".into(), ..Default::default() });
    let c = config();
    s.enabled = c.enabled;
    s.restore_pending = c.restore_pending;
    s.has_backup = config::original_path().exists();
    s
}

fn set_status(f: impl FnOnce(&mut Status)) {
    let mut g = STATUS.lock().unwrap();
    let s = g.get_or_insert_with(|| Status { layer: "off".into(), ..Default::default() });
    f(s);
}

fn rect(r: windows::Win32::Foundation::RECT) -> Rect {
    Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top }
}

// ---------------- Cuentas comunes (ordenar de verdad y vista previa) ----------------

/// Agrupa por zona (en el orden de las zonas) y ordena por nombre dentro de cada una
fn group<'a, T>(items: &'a [T], item: impl Fn(&T) -> &classify::DeskItem, cfg: &Config) -> Vec<(String, Vec<&'a T>)> {
    let mut groups: Vec<(String, Vec<&T>)> = cfg.zones.iter().filter(|z| !z.hidden).map(|z| (z.id.clone(), Vec::new())).collect();
    for it in items {
        let zone = classify::zone_for(item(it), cfg);
        if let Some(g) = groups.iter_mut().find(|(id, _)| *id == zone) {
            g.1.push(it);
        }
    }
    for (_, g) in &mut groups {
        g.sort_by_key(|it| item(it).name.to_lowercase());
    }
    groups
}

fn plan<T>(groups: &[(String, Vec<&T>)], m: &Metrics, work: Rect) -> Layout {
    let counts: Vec<(String, usize)> = groups.iter().map(|(id, g)| (id.clone(), g.len())).collect();
    layout::layout(&counts, m, work)
}

fn frames_of(l: &Layout, m: &Metrics, monitor: Rect, work: Rect, cfg: &Config) -> Frames {
    Frames {
        monitor,
        work,
        cell_w: m.cell_w,
        cell_h: m.cell_h,
        pad: m.pad,
        header: m.header,
        fits: l.fits,
        zones: l
            .zones
            .iter()
            .map(|z| Frame {
                id: z.id.clone(),
                name: cfg.zones.iter().find(|c| c.id == z.id).map(|c| c.name.clone()).unwrap_or_default(),
                count: z.count,
                cols: z.cols,
                rect: z.rect,
            })
            .collect(),
    }
}

/// Hueco aproximado de un icono de `size` px si aún no lo hemos medido
fn guess_spacing(size: u32, scale: f32) -> (i32, i32) {
    if let Some((_, w, h)) = SPACING.lock().unwrap().iter().find(|(s, _, _)| *s == size) {
        return (*w, *h);
    }
    let s = size as f32;
    (((s + 40.0) * scale).round() as i32, ((s + 46.0) * scale).round() as i32)
}

fn scale_of(app: &AppHandle) -> f32 {
    app.primary_monitor().ok().flatten().map(|m| m.scale_factor() as f32).unwrap_or(1.0)
}

// ---------------- El hilo de trabajo ----------------

enum Cmd {
    /// Ajustes cambiados: aplicar ya
    Reload,
    Organize(Sender<Result<Status, String>>),
    Restore(Sender<Result<Status, String>>),
}

struct Worker {
    app: AppHandle,
    rx: Receiver<Cmd>,
    progman: Option<isize>,
    disk_sig: u64,
    display_sig: (Rect, Rect, u32),
    due: Option<Instant>,
    parent: Option<isize>,
}

fn disk_signature() -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut mix = |b: &[u8]| {
        for x in b {
            h ^= u64::from(*x);
            h = h.wrapping_mul(0x100000001b3);
        }
    };
    for dir in shell::desktop_dirs() {
        let mut names: Vec<(String, u64)> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| !e.file_name().to_string_lossy().eq_ignore_ascii_case("desktop.ini"))
            .map(|e| {
                let t = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
                (e.file_name().to_string_lossy().into_owned(), t)
            })
            .collect();
        names.sort();
        for (n, t) in names {
            mix(n.as_bytes());
            mix(&t.to_le_bytes());
        }
    }
    h
}

impl Worker {
    fn display_signature(&self) -> (Rect, Rect, u32) {
        let (m, w) = layer::primary_monitor();
        (rect(m), rect(w), (scale_of(&self.app) * 100.0) as u32)
    }

    fn layer_hwnd(&self) -> Option<HWND> {
        self.app.get_webview_window(LABEL).and_then(|w| w.hwnd().ok()).map(|h| HWND(h.0 as _))
    }

    fn drop_layer(&mut self) {
        if let Some(h) = self.layer_hwnd() {
            layer::detach(h);
        }
        self.parent = None;
        set_status(|s| {
            s.layer = "off".into();
            s.layer_kind = None;
        });
    }

    /// Crea (si hace falta) la ventana de los marcos y la cuelga detrás de los iconos
    fn ensure_layer(&mut self, monitor: Rect) {
        let fail = |code: String| {
            log::warn!("Escritorio organizado: capa no colgada ({code})");
            set_status(|s| {
                s.layer = "failed".into();
                s.layer_error = Some(code);
            });
        };
        let (kind, parent) = match layer::target() {
            Ok(t) => t,
            Err(e) => return fail(e),
        };
        let child = match self.layer_hwnd() {
            Some(h) => h,
            None => {
                let built = WebviewWindowBuilder::new(&self.app, LABEL, WebviewUrl::App("capa-escritorio".into()))
                    .title("CouchDesk")
                    .decorations(false)
                    .shadow(false)
                    .resizable(false)
                    .skip_taskbar(true)
                    .focused(false)
                    .focusable(false)
                    .transparent(true)
                    .visible(false)
                    .background_color(tauri::window::Color(0, 0, 0, 0))
                    .build();
                match built {
                    Ok(w) => {
                        let _ = w.set_ignore_cursor_events(true);
                        match w.hwnd() {
                            Ok(h) => HWND(h.0 as _),
                            Err(_) => return fail("window-failed".into()),
                        }
                    }
                    Err(e) => return fail(format!("window-failed: {e}")),
                }
            }
        };
        if self.parent == Some(parent.0 as isize) && layer::is_attached(child, parent) {
            return;
        }
        let r = windows::Win32::Foundation::RECT { left: monitor.x, top: monitor.y, right: monitor.right(), bottom: monitor.bottom() };
        match layer::attach(child, parent, r) {
            Ok(()) => {
                self.parent = Some(parent.0 as isize);
                set_status(|s| {
                    s.layer = "attached".into();
                    s.layer_error = None;
                    s.layer_kind = Some(kind.code().into());
                });
            }
            Err(e) => fail(e),
        }
    }

    /// Guarda el escritorio de antes (una sola vez), pone los iconos en sus zonas y cuelga los marcos
    fn organize(&mut self) -> Result<Status, String> {
        let cfg = config();
        let view = shell::View::open()?;
        let mask = (FWF_AUTOARRANGE.0 | FWF_SNAPTOGRID.0) as u32;
        if config::load_original().is_none() {
            let (mode, size) = view.view_mode();
            let icons = view.icons();
            config::save_original(&config::Original {
                flags: view.flags(),
                view_mode: mode,
                icon_size: size,
                icons: icons.iter().map(|i| config::SavedIcon { parsing: i.item.parsing.clone(), x: i.x, y: i.y }).collect(),
                saved_at: now_secs(),
            })?;
        }
        // Primero el tamaño (cambia el hueco de cada icono) y quitar "organizar solos"
        view.set_view_mode(FVM_ICON.0, cfg.icon_size as i32)?;
        let icons = view.icons();
        let (cw, ch) = view.spacing();
        let scale = scale_of(&self.app);
        let (cw, ch) = if cw > 0 && ch > 0 { (cw, ch) } else { guess_spacing(cfg.icon_size, scale) };
        let (mon, work) = layer::primary_monitor();
        let (mon, work) = (rect(mon), rect(work));
        // Dónde cae un icono dentro de su hueco: se deja uno en la esquina con "alinear a la
        // cuadrícula" puesto y se mira adónde lo lleva Windows (así quedan centrados en cada hueco)
        view.set_flags(mask, FWF_SNAPTOGRID.0 as u32)?;
        let offset = icons
            .first()
            .and_then(|first| {
                view.place(&[(first.item.parsing.clone(), work.x, work.y)]);
                view.position_of(&first.item.parsing)
            })
            .map(|(x, y)| ((x - work.x).rem_euclid(cw), (y - work.y).rem_euclid(ch)))
            .unwrap_or((0, 0));
        view.set_flags(mask, 0)?;
        {
            let mut sp = SPACING.lock().unwrap();
            sp.retain(|(s, _, _)| *s != cfg.icon_size);
            sp.push((cfg.icon_size, cw, ch));
        }
        let m = Metrics::for_cell(cw, ch, scale);
        let groups = group(&icons, |i| &i.item, &cfg);
        let l = plan(&groups, &m, work);
        let mut moves = Vec::new();
        if l.fits {
            for z in &l.zones {
                let g = &groups.iter().find(|(id, _)| *id == z.id).unwrap().1;
                for (k, it) in g.iter().enumerate() {
                    let (x, y) = layout::item_pos(z, k, &m);
                    moves.push((it.item.parsing.clone(), x + offset.0, y + offset.1));
                }
            }
        } else {
            let all: Vec<_> = groups.iter().flat_map(|(_, g)| g.iter()).collect();
            for (it, (x, y)) in all.iter().zip(layout::plain_grid(all.len(), &m, work)) {
                moves.push((it.item.parsing.clone(), x + offset.0, y + offset.1));
            }
        }
        let placed = view.place(&moves);
        let frames = frames_of(&l, &m, mon, work, &cfg);
        *FRAMES.lock().unwrap() = Some(frames.clone());
        let _ = self.app.emit(EVENT, &frames);
        if cfg.frames && l.fits && !frames.zones.is_empty() {
            self.ensure_layer(mon);
        } else {
            self.drop_layer();
        }
        set_status(|s| {
            s.organized_at = Some(now_secs());
            s.item_count = placed;
            s.fits = l.fits;
            s.last_error = None;
        });
        Ok(status())
    }

    /// Vuelve al escritorio de antes: posiciones, tamaño y opciones; quita la capa y se apaga
    fn restore(&mut self) -> Result<Status, String> {
        let mut cfg = config();
        cfg.enabled = false;
        self.drop_layer();
        *FRAMES.lock().unwrap() = None;
        let _ = self.app.emit(EVENT, Frames::default());
        let view = match shell::View::open() {
            Ok(v) => v,
            Err(_) => {
                // Sin escritorio a la vista: se hará en cuanto vuelva
                cfg.restore_pending = config::original_path().exists();
                save_config(cfg)?;
                return Ok(status());
            }
        };
        if let Some(o) = config::load_original() {
            let mask = (FWF_AUTOARRANGE.0 | FWF_SNAPTOGRID.0) as u32;
            view.set_flags(mask, 0)?;
            if o.view_mode > 0 && o.icon_size > 0 {
                view.set_view_mode(o.view_mode, o.icon_size)?;
            }
            let moves: Vec<_> = o.icons.iter().map(|i| (i.parsing.clone(), i.x, i.y)).collect();
            view.place(&moves);
            view.set_flags(mask, o.flags & mask)?;
            config::drop_original();
        }
        cfg.restore_pending = false;
        save_config(cfg)?;
        set_status(|s| s.organized_at = None);
        Ok(status())
    }

    fn tick(&mut self) {
        let desk = layer::desktop().map(|h| h.0 as isize);
        let restarted = layer::EXPLORER_RESTARTED.swap(false, Ordering::Relaxed);
        if desk != self.progman || restarted {
            // El escritorio aparece, desaparece o explorer se reinició: la capa vieja ya no sirve
            self.progman = desk;
            self.parent = None;
            set_status(|s| {
                s.desktop_visible = desk.is_some();
                if desk.is_none() {
                    s.layer = "off".into();
                }
            });
            // Explorer tarda un poco en pintar sus iconos
            self.due = desk.map(|_| Instant::now() + Duration::from_millis(1500));
        }
        if desk.is_none() {
            return;
        }
        let cfg = config();
        if cfg.restore_pending {
            if self.due.is_some_and(|d| Instant::now() >= d) {
                self.due = None;
                if let Err(e) = self.restore() {
                    set_status(|s| s.last_error = Some(e));
                }
            }
            return;
        }
        if !cfg.enabled {
            if self.parent.is_some() {
                self.drop_layer();
            }
            self.due = None;
            return;
        }
        let sig = disk_signature();
        if sig != self.disk_sig {
            // Cambios en el escritorio: se espera a que dejen de llegar (2 s) antes de recolocar
            if self.disk_sig != 0 {
                self.due = Some(Instant::now() + Duration::from_millis(TICK_MS as u64));
            }
            self.disk_sig = sig;
        }
        let disp = self.display_signature();
        if layer::DISPLAY_CHANGED.swap(false, Ordering::Relaxed) || disp != self.display_sig {
            if self.display_sig.2 != 0 {
                self.due = Some(Instant::now() + Duration::from_millis(800));
            }
            self.display_sig = disp;
        }
        if self.due.is_some_and(|d| Instant::now() >= d) {
            self.due = None;
            if let Err(e) = self.organize() {
                log::warn!("Escritorio organizado: {e}");
                set_status(|s| s.last_error = Some(e));
            }
        }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Reload => {
                let cfg = config();
                if cfg.enabled && self.progman.is_some() {
                    self.due = Some(Instant::now());
                }
                self.tick();
            }
            Cmd::Organize(reply) => {
                let r = if self.progman.is_some() { self.organize() } else { Err("no-desktop".into()) };
                if let Err(e) = &r {
                    set_status(|s| s.last_error = Some(e.clone()));
                }
                let _ = reply.send(r);
            }
            Cmd::Restore(reply) => {
                let _ = reply.send(self.restore());
            }
        }
    }

    fn run(mut self) {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        let Some(hwnd) = layer::create_listener() else {
            log::error!("Escritorio organizado: sin ventana de avisos");
            return;
        };
        WAKE_HWND.store(hwnd.0 as isize, Ordering::Relaxed);
        unsafe {
            SetTimer(Some(hwnd), 1, TICK_MS, None);
        }
        self.tick();
        let mut msg = MSG::default();
        while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if msg.message == WM_TIMER {
                self.tick();
            }
            while let Ok(cmd) = self.rx.try_recv() {
                self.handle(cmd);
            }
        }
    }
}

fn send(cmd: Cmd) -> Result<(), String> {
    SENDER.lock().unwrap().as_ref().ok_or("not-running")?.send(cmd).map_err(|e| e.to_string())?;
    let h = WAKE_HWND.load(Ordering::Relaxed);
    if h != 0 {
        unsafe {
            let _ = PostMessageW(Some(HWND(h as _)), WM_WAKE, WPARAM(0), LPARAM(0));
        }
    }
    Ok(())
}

fn ask(make: impl FnOnce(Sender<Result<Status, String>>) -> Cmd) -> Result<Status, String> {
    let (tx, rx) = channel();
    send(make(tx))?;
    rx.recv_timeout(Duration::from_secs(20)).map_err(|_| "timeout".to_string())?
}

fn save_config(c: Config) -> Result<Config, String> {
    let c = c.normalized();
    config::save(&c)?;
    *CONFIG.lock().unwrap() = Some(c.clone());
    Ok(c)
}

/// Se llama una vez al arrancar la app (no en modo salvapantallas)
pub fn start(app: AppHandle) {
    let (tx, rx) = channel();
    *SENDER.lock().unwrap() = Some(tx);
    let w = Worker { app, rx, progman: None, disk_sig: 0, display_sig: (Rect::default(), Rect::default(), 0), due: None, parent: None };
    std::thread::Builder::new().name("deskzones".into()).spawn(move || w.run()).ok();
}

// ---------------- Órdenes ----------------

#[tauri::command(async)]
pub fn deskzones_get() -> Config {
    config()
}

#[tauri::command(async)]
pub fn deskzones_set(config: Config) -> Result<Config, String> {
    let c = save_config(config)?;
    let _ = send(Cmd::Reload);
    Ok(c)
}

#[tauri::command(async)]
pub fn deskzones_status() -> Status {
    status()
}

#[tauri::command(async)]
pub fn deskzones_organize() -> Result<Status, String> {
    ask(Cmd::Organize)
}

#[tauri::command(async)]
pub fn deskzones_restore() -> Result<Status, String> {
    ask(Cmd::Restore)
}

/// Lo que dibuja la capa ahora mismo
#[tauri::command(async)]
pub fn deskzones_frames() -> Frames {
    FRAMES.lock().unwrap().clone().unwrap_or_default()
}

/// Vista previa con los ajustes guardados, leyendo el escritorio del disco (no hace falta explorer)
#[tauri::command(async)]
pub fn deskzones_preview(app: AppHandle) -> Frames {
    let cfg = config();
    let scale = scale_of(&app);
    // Los accesos directos se leen con COM: en un hilo propio preparado para ello
    let items = std::thread::spawn(|| {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        shell::items_from_disk()
    })
    .join()
    .unwrap_or_default();
    let (cw, ch) = guess_spacing(cfg.icon_size, scale);
    let m = Metrics::for_cell(cw, ch, scale);
    let (mon, work) = layer::primary_monitor();
    let groups = group(&items, |i| i, &cfg);
    let l = plan(&groups, &m, rect(work));
    frames_of(&l, &m, rect(mon), rect(work), &cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prueba real y solo de lectura: enseña los iconos del escritorio de explorer, su zona y posición.
    /// `cargo test --lib deskzones::tests::escritorio_real -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn escritorio_real() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        // Solo mirar: sin mandar 0x052C a Progman
        let host = layer::icons_host();
        println!(
            "Iconos dentro de: {} · WorkerW del fondo ya creado: {:?}",
            if host.is_some() && host == layer::progman() { "Progman (24H2+ o aún sin WorkerW)" } else { "un WorkerW (clásico)" },
            layer::detect().map(|(k, h)| (k.code(), h.0))
        );
        let (mon, work) = layer::primary_monitor();
        println!("Monitor {:?} · trabajo {:?}", rect(mon), rect(work));
        let view = match shell::View::open() {
            Ok(v) => v,
            Err(e) => {
                println!("Sin escritorio de explorer: {e}");
                return;
            }
        };
        let (mode, size) = view.view_mode();
        println!("Vista: modo {mode}, icono {size} px, hueco {:?}, opciones {:#x}", view.spacing(), view.flags());
        let cfg = Config::default();
        for i in view.icons() {
            println!("{:>5},{:>5}  {:<12} {}  [{}]", i.x, i.y, classify::zone_for(&i.item, &cfg), i.item.name, i.item.target.as_deref().unwrap_or(""));
        }
        let (cw, ch) = view.spacing();
        let m = Metrics::for_cell(cw, ch, 1.0);
        let icons = view.icons();
        let groups = group(&icons, |i| &i.item, &cfg);
        let l = plan(&groups, &m, rect(work));
        println!("Plan (sin aplicar): cabe={} {:?}", l.fits, l.zones.iter().map(|z| (&z.id, z.count, z.rect)).collect::<Vec<_>>());
    }
}
