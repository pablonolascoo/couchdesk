//! Rendimiento: ajustes seguros de Windows (sin permisos de administrador) y plan de energía al jugar.
//!
//! · Ajustes de HKCU (se aplican al momento y se pueden deshacer): Modo Juego, grabación en segundo
//!   plano de la Xbox Game Bar, apps de la Tienda en segundo plano y transparencias.
//!   Lo que había antes se guarda en %LOCALAPPDATA%\CouchDesk\rendimiento-antes.json
//!   (clave "ruta\valor" → número o null si no existía). Deshacer lo devuelve tal cual.
//! · "Máximo rendimiento al jugar": con un juego a pantalla completa delante se pasa al plan de
//!   Máximo / Alto rendimiento y, al terminar, se vuelve al de antes. Guardado en rendimiento.json.
//! · Tiempo de arranque: main.rs marca el inicio; la página avisa con `perf_app_ready` al pintar.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

// ---------------- Tiempo de arranque ----------------

static START: OnceLock<Instant> = OnceLock::new();
static SETUP: OnceLock<Instant> = OnceLock::new();

/// Al empezar el proceso (main.rs)
pub fn mark_start() {
    let _ = START.set(Instant::now());
}

/// Al empezar `setup` de Tauri (lib.rs)
pub fn mark_setup() {
    let _ = SETUP.set(Instant::now());
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ready {
    /// Desde que arrancó el proceso hasta el primer pintado de la página
    process_ms: u64,
    /// Desde el `setup` de Tauri
    setup_ms: u64,
}

/// La página ya se ha pintado: apunta cuánto ha tardado
#[tauri::command(async)]
pub fn perf_app_ready(page_ms: f64) -> Ready {
    let ms = |t: Option<&Instant>| t.map(|t| t.elapsed().as_millis() as u64).unwrap_or(0);
    let r = Ready { process_ms: ms(START.get()), setup_ms: ms(SETUP.get()) };
    log::info!("[arranque] primer pintado: {} ms desde el proceso, {} ms desde setup, {page_ms:.0} ms dentro de la página", r.process_ms, r.setup_ms);
    eprintln!("[arranque] primer pintado: {} ms desde el proceso, {} ms desde setup, {page_ms:.0} ms dentro de la página", r.process_ms, r.setup_ms);
    r
}

// ---------------- Registro ----------------

/// Lo mínimo del registro que hace falta (así se puede probar sin tocar Windows)
pub trait Reg {
    fn get(&self, path: &str, name: &str) -> Option<u32>;
    fn set(&mut self, path: &str, name: &str, value: u32) -> Result<(), String>;
    fn delete(&mut self, path: &str, name: &str) -> Result<(), String>;
}

/// HKEY_CURRENT_USER de verdad
pub struct Hkcu;

impl Reg for Hkcu {
    fn get(&self, path: &str, name: &str) -> Option<u32> {
        winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey(path).ok()?.get_value::<u32, _>(name).ok()
    }
    fn set(&mut self, path: &str, name: &str, value: u32) -> Result<(), String> {
        let (key, _) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).create_subkey(path).map_err(|e| e.to_string())?;
        key.set_value(name, &value).map_err(|e| e.to_string())
    }
    fn delete(&mut self, path: &str, name: &str) -> Result<(), String> {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
        match winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(path, KEY_SET_VALUE) {
            Ok(k) => {
                let _ = k.delete_value(name);
                Ok(())
            }
            Err(_) => Ok(()),
        }
    }
}

pub struct Tweak {
    pub id: &'static str,
    /// (ruta en HKCU, valor, número que se pone)
    pub values: &'static [(&'static str, &'static str, u32)],
}

pub const TWEAKS: &[Tweak] = &[
    Tweak { id: "gameMode", values: &[("Software\\Microsoft\\GameBar", "AutoGameModeEnabled", 1)] },
    Tweak {
        id: "recording",
        values: &[
            ("System\\GameConfigStore", "GameDVR_Enabled", 0),
            ("Software\\Microsoft\\Windows\\CurrentVersion\\GameDVR", "AppCaptureEnabled", 0),
        ],
    },
    Tweak {
        id: "backgroundApps",
        values: &[("Software\\Microsoft\\Windows\\CurrentVersion\\BackgroundAccessApplications", "GlobalUserDisabled", 1)],
    },
    Tweak { id: "transparency", values: &[("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize", "EnableTransparency", 0)] },
];

/// Lo que había antes de cada valor tocado: "ruta\valor" → número (o `None` si no existía)
pub type Before = BTreeMap<String, Option<u32>>;

fn key(path: &str, name: &str) -> String {
    format!("{path}\\{name}")
}

pub fn find(id: &str) -> Option<&'static Tweak> {
    TWEAKS.iter().find(|t| t.id == id)
}

pub fn is_on(reg: &dyn Reg, t: &Tweak) -> bool {
    t.values.iter().all(|(p, n, want)| reg.get(p, n) == Some(*want))
}

/// Activa un ajuste y apunta lo de antes (solo la primera vez: así "deshacer" vuelve al original)
pub fn apply(reg: &mut dyn Reg, before: &mut Before, t: &Tweak) -> Result<(), String> {
    for (p, n, want) in t.values {
        before.entry(key(p, n)).or_insert_with(|| reg.get(p, n));
        reg.set(p, n, *want)?;
    }
    Ok(())
}

/// Deja un ajuste como estaba antes de CouchDesk. Si no hay nada apuntado, no se toca.
pub fn undo(reg: &mut dyn Reg, before: &mut Before, t: &Tweak) -> Result<(), String> {
    for (p, n, _) in t.values {
        if let Some(old) = before.remove(&key(p, n)) {
            match old {
                Some(v) => reg.set(p, n, v)?,
                None => reg.delete(p, n)?,
            }
        }
    }
    Ok(())
}

pub fn can_undo(before: &Before, t: &Tweak) -> bool {
    t.values.iter().any(|(p, n, _)| before.contains_key(&key(p, n)))
}

// ---------------- Archivos ----------------

fn dir() -> PathBuf {
    std::env::var("COUCHDESK_PERF_DIR").map(PathBuf::from).unwrap_or_else(|_| crate::paths::data_dir())
}

fn read_json<T: for<'a> Deserialize<'a> + Default>(name: &str) -> T {
    std::fs::read_to_string(dir().join(name)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn write_json<T: Serialize>(name: &str, v: &T) -> Result<(), String> {
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    std::fs::write(dir().join(name), serde_json::to_string_pretty(v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

const BEFORE_FILE: &str = "rendimiento-antes.json";
const SETTINGS_FILE: &str = "rendimiento.json";

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// `None` = sin elegir: activado si el PC está enchufado
    pub game_plan: Option<bool>,
    /// Plan que había antes de pasar al de máximo rendimiento (si ahora mismo está cambiado)
    pub switched_from: Option<String>,
}

static LOCK: Mutex<()> = Mutex::new(());

// ---------------- Planes de energía ----------------

/// Plan que crea scripts/optimizar-consola.cmd (copia de Máximo rendimiento con GUID fijo)
const COUCHDESK: &str = "c0ced35c-7a3f-4e7b-9c1d-5e0c0de5c0de";
const ULTIMATE: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";
const HIGH: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";

#[derive(Debug, PartialEq, Clone)]
pub struct Plan {
    pub guid: String,
    pub name: String,
    pub active: bool,
}

fn is_guid(s: &str) -> bool {
    s.len() == 36 && s.chars().filter(|c| *c == '-').count() == 4 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// Salida de `powercfg /list` (cualquier idioma)
pub fn parse_plans(text: &str) -> Vec<Plan> {
    text.lines()
        .filter_map(|l| {
            let guid = l.split_whitespace().find(|w| is_guid(w))?.to_lowercase();
            let name = match (l.find('('), l.rfind(')')) {
                (Some(a), Some(b)) if b > a => l[a + 1..b].trim().to_string(),
                _ => String::new(),
            };
            Some(Plan { guid, name, active: l.trim_end().ends_with('*') })
        })
        .collect()
}

/// El plan más rápido que tenga el PC: el de CouchDesk, Máximo rendimiento, Alto rendimiento o uno que lo diga en el nombre
pub fn pick_fast(plans: &[Plan]) -> Option<&Plan> {
    plans
        .iter()
        .find(|p| p.guid == COUCHDESK)
        .or_else(|| plans.iter().find(|p| p.guid == ULTIMATE))
        .or_else(|| plans.iter().find(|p| p.guid == HIGH))
        .or_else(|| {
            plans.iter().find(|p| {
                let n = p.name.to_lowercase();
                ["máximo", "maximo", "ultimate", "high perf", "alto rend", "performance", "rendimiento", "leistung", "prestazioni", "desempenho"]
                    .iter()
                    .any(|w| n.contains(w))
                    && !n.contains("equilibr")
                    && !n.contains("balanced")
            })
        })
}

fn powercfg(args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("powercfg").args(args).creation_flags(0x0800_0000).output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(String::from_utf8_lossy(&out.stdout).into_owned()) } else { Err(String::from_utf8_lossy(&out.stdout).trim().to_string()) }
}

fn plans() -> Vec<Plan> {
    powercfg(&["/list"]).map(|t| parse_plans(&t)).unwrap_or_default()
}

fn on_ac() -> bool {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut s = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut s) }.is_err() {
        return true;
    }
    // Sin batería (sobremesa) o enchufado
    s.BatteryFlag & 128 != 0 || s.ACLineStatus == 1
}

pub fn game_plan_enabled(s: &Settings, ac: bool) -> bool {
    s.game_plan.unwrap_or(ac)
}

/// ¿Hay un juego (u otro programa) a pantalla completa delante, que no sea CouchDesk ni el escritorio?
fn game_in_front() -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
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
        if matches!(String::from_utf16_lossy(&class[..n]).as_str(), "WorkerW" | "Progman" | "Shell_TrayWnd") {
            return false;
        }
        let mut r = RECT::default();
        if GetWindowRect(fg, &mut r).is_err() {
            return false;
        }
        let mut mi = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(MonitorFromWindow(fg, MONITOR_DEFAULTTOPRIMARY), &mut mi);
        let m = mi.rcMonitor;
        r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom
    }
}

/// Vuelve al plan de antes si se había cambiado
fn restore_plan(s: &mut Settings) {
    if let Some(prev) = s.switched_from.take() {
        let _ = powercfg(&["/setactive", &prev]);
        let _ = write_json(SETTINGS_FILE, s);
    }
}

static WATCHING: AtomicBool = AtomicBool::new(false);

/// Vigila si hay un juego delante (cada 3 s: casi no gasta)
pub fn start() {
    if WATCHING.swap(true, Ordering::Relaxed) {
        return;
    }
    std::thread::spawn(|| {
        // Lo primero: si la consola se cerró a mitad de partida, el plan vuelve a su sitio
        std::thread::sleep(Duration::from_secs(5));
        let mut quiet = 0u32;
        loop {
            {
                let _g = LOCK.lock().unwrap();
                let mut s: Settings = read_json(SETTINGS_FILE);
                let game = game_in_front();
                if game && game_plan_enabled(&s, on_ac()) {
                    quiet = 0;
                    if s.switched_from.is_none() {
                        let all = plans();
                        if let (Some(cur), Some(fast)) = (all.iter().find(|p| p.active), pick_fast(&all)) {
                            if cur.guid != fast.guid && powercfg(&["/setactive", &fast.guid]).is_ok() {
                                s.switched_from = Some(cur.guid.clone());
                                let _ = write_json(SETTINGS_FILE, &s);
                            }
                        }
                    }
                } else if s.switched_from.is_some() {
                    // Unos segundos sin juego (cambiar de ventana un momento no cuenta)
                    quiet += 1;
                    if quiet >= 3 || !game_plan_enabled(&s, on_ac()) {
                        restore_plan(&mut s);
                        quiet = 0;
                    }
                }
            }
            std::thread::sleep(Duration::from_secs(3));
        }
    });
}

// ---------------- Órdenes ----------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TweakState {
    id: &'static str,
    on: bool,
    can_undo: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    tweaks: Vec<TweakState>,
    any_undo: bool,
    game_plan: bool,
    /// Nombre del plan rápido que se usará (None: este PC no tiene ninguno)
    fast_plan: Option<String>,
    ac_power: bool,
    /// Ahora mismo está puesto el plan rápido por un juego
    switched: bool,
    /// La optimización a fondo (optimizar-consola.cmd) está aplicada: hay algo que deshacer
    optimized: bool,
}

/// optimizar-consola.cmd se copia a la carpeta de datos (scr::run_optimizer) y deja ahí lo que
/// hace falta para deshacerlo; deshacer-optimizacion.cmd lo borra al terminar.
fn optimized() -> bool {
    crate::paths::data_dir().join("deshacer-registro.cmd").exists()
}

fn status() -> Status {
    let reg = Hkcu;
    let before: Before = read_json(BEFORE_FILE);
    let s: Settings = read_json(SETTINGS_FILE);
    let ac = on_ac();
    let tweaks: Vec<TweakState> = TWEAKS.iter().map(|t| TweakState { id: t.id, on: is_on(&reg, t), can_undo: can_undo(&before, t) }).collect();
    Status {
        any_undo: tweaks.iter().any(|t| t.can_undo),
        tweaks,
        game_plan: game_plan_enabled(&s, ac),
        fast_plan: pick_fast(&plans()).map(|p| p.name.clone()),
        ac_power: ac,
        switched: s.switched_from.is_some(),
        optimized: optimized(),
    }
}

#[tauri::command(async)]
pub fn perf_status() -> Status {
    status()
}

/// Activa o desactiva un ajuste. Desactivar = dejarlo como estaba antes de CouchDesk.
#[tauri::command(async)]
pub fn perf_tweak_set(id: String, on: bool) -> Result<Status, String> {
    let t = find(&id).ok_or_else(|| crate::i18n::tr("common.bad_action").to_string())?;
    {
        let _g = LOCK.lock().unwrap();
        let mut before: Before = read_json(BEFORE_FILE);
        let mut reg = Hkcu;
        if on {
            apply(&mut reg, &mut before, t)?;
        } else if can_undo(&before, t) {
            undo(&mut reg, &mut before, t)?;
        } else {
            // Ya estaba así antes de CouchDesk: se pone el valor contrario por defecto de Windows
            for (p, n, want) in t.values {
                reg.set(p, n, u32::from(*want == 0))?;
            }
        }
        write_json(BEFORE_FILE, &before)?;
    }
    Ok(status())
}

/// Deshace todos los ajustes de esta pantalla
#[tauri::command(async)]
pub fn perf_undo_all() -> Result<Status, String> {
    {
        let _g = LOCK.lock().unwrap();
        let mut before: Before = read_json(BEFORE_FILE);
        let mut reg = Hkcu;
        for t in TWEAKS {
            undo(&mut reg, &mut before, t)?;
        }
        write_json(BEFORE_FILE, &before)?;
    }
    Ok(status())
}

#[tauri::command(async)]
pub fn perf_game_plan_set(on: bool) -> Result<Status, String> {
    {
        let _g = LOCK.lock().unwrap();
        let mut s: Settings = read_json(SETTINGS_FILE);
        s.game_plan = Some(on);
        if !on {
            restore_plan(&mut s);
        }
        write_json(SETTINGS_FILE, &s)?;
    }
    Ok(status())
}

/// Abre Windows Update en Configuración (solo estas páginas)
#[tauri::command(async)]
pub fn perf_open_settings(page: String) -> Result<(), String> {
    const ALLOWED: [&str; 3] = ["windowsupdate", "windowsupdate-activehours", "windowsupdate-options"];
    if !ALLOWED.contains(&page.as_str()) {
        return Err(crate::i18n::tr("common.bad_action").into());
    }
    std::process::Command::new("cmd")
        .raw_arg(format!("/C start \"\" ms-settings:{page}"))
        .creation_flags(0x0800_0000)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Fake(HashMap<String, u32>);
    impl Reg for Fake {
        fn get(&self, p: &str, n: &str) -> Option<u32> {
            self.0.get(&key(p, n)).copied()
        }
        fn set(&mut self, p: &str, n: &str, v: u32) -> Result<(), String> {
            self.0.insert(key(p, n), v);
            Ok(())
        }
        fn delete(&mut self, p: &str, n: &str) -> Result<(), String> {
            self.0.remove(&key(p, n));
            Ok(())
        }
    }

    #[test]
    fn aplicar_y_deshacer_vuelve_al_original() {
        let rec = find("recording").unwrap();
        let mut reg = Fake::default();
        // Antes: grabación encendida en uno, el otro valor no existía
        reg.set("System\\GameConfigStore", "GameDVR_Enabled", 1).unwrap();
        let mut before = Before::new();
        assert!(!is_on(&reg, rec));
        apply(&mut reg, &mut before, rec).unwrap();
        assert!(is_on(&reg, rec));
        assert!(can_undo(&before, rec));
        // Aplicar dos veces no pisa lo guardado
        apply(&mut reg, &mut before, rec).unwrap();
        assert_eq!(before.get("System\\GameConfigStore\\GameDVR_Enabled"), Some(&Some(1)));
        undo(&mut reg, &mut before, rec).unwrap();
        assert_eq!(reg.get("System\\GameConfigStore", "GameDVR_Enabled"), Some(1));
        assert_eq!(reg.get("Software\\Microsoft\\Windows\\CurrentVersion\\GameDVR", "AppCaptureEnabled"), None, "lo que no existía se borra");
        assert!(before.is_empty());
    }

    #[test]
    fn se_guarda_y_se_lee_igual() {
        let mut before = Before::new();
        before.insert("a\\b".into(), Some(1));
        before.insert("c\\d".into(), None);
        let json = serde_json::to_string(&before).unwrap();
        assert_eq!(json, r#"{"a\\b":1,"c\\d":null}"#);
        assert_eq!(serde_json::from_str::<Before>(&json).unwrap(), before);

        let s = Settings { game_plan: Some(false), switched_from: Some(HIGH.into()) };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains(r#""gamePlan":false"#) && json.contains("switchedFrom"));
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), s);
        // Archivo viejo o vacío: valores por defecto
        assert_eq!(serde_json::from_str::<Settings>("{}").unwrap(), Settings::default());
        assert!(game_plan_enabled(&Settings::default(), true));
        assert!(!game_plan_enabled(&Settings::default(), false));
        assert!(!game_plan_enabled(&Settings { game_plan: Some(false), ..Default::default() }, true));
    }

    #[test]
    fn planes_de_energia() {
        let es = "Esquemas de energía existentes (* Activo)\n-----------------------------------\n\
            GUID de plan de energía: 381b4222-f694-41f0-9685-ff5bb260df2e  (Equilibrado) *\n\
            GUID de plan de energía: 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c  (Alto rendimiento)\n";
        let p = parse_plans(es);
        assert_eq!(p.len(), 2);
        assert!(p[0].active && p[0].name == "Equilibrado");
        assert_eq!(pick_fast(&p).unwrap().guid, HIGH);
        // Un plan propio con nombre claro
        let own = vec![
            Plan { guid: "381b4222-f694-41f0-9685-ff5bb260df2e".into(), name: "Balanced".into(), active: true },
            Plan { guid: "11111111-2222-3333-4444-555555555555".into(), name: "Juego - Maximo rendimiento".into(), active: false },
        ];
        assert_eq!(pick_fast(&own).unwrap().name, "Juego - Maximo rendimiento");
        assert!(pick_fast(&own[..1]).is_none());
        // El plan del optimizador va antes que Alto rendimiento
        let mut with_cd = p.clone();
        with_cd.push(Plan { guid: COUCHDESK.into(), name: "CouchDesk - Maximo rendimiento".into(), active: false });
        assert_eq!(pick_fast(&with_cd).unwrap().guid, COUCHDESK);
    }

    #[test]
    fn el_script_usa_el_mismo_plan() {
        let script = include_str!("../../scripts/optimizar-consola.cmd");
        assert!(script.contains(COUCHDESK), "el GUID del plan tiene que coincidir con optimizar-consola.cmd");
        // cmd.exe lee los .cmd con la página de códigos del sistema: solo ASCII
        assert!(script.is_ascii() && include_str!("../../scripts/deshacer-optimizacion.cmd").is_ascii());
    }
}
