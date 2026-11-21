//! Ajustes nativos del PC: volumen, brillo, batería, energía y Wi-Fi.

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::process::Command;

const NO_WINDOW: u32 = 0x0800_0000;

/// Ejecuta un programa sin abrir ventana y devuelve su salida como texto.
fn run(exe: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(exe)
        .args(args)
        .creation_flags(NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    // netsh/powercfg escriben en la página de códigos OEM; lossy evita fallos con la ñ
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    if out.status.success() { Ok(text) } else { Err(text.trim().to_string()) }
}

fn powershell(script: &str) -> Result<String, String> {
    run("powershell.exe", &["-NoProfile", "-NonInteractive", "-Command", script])
}

// ---------- Sonido ----------

pub(crate) mod audio {
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

    fn endpoint() -> windows::core::Result<IAudioEndpointVolume> {
        unsafe {
            // Si el hilo ya tiene COM iniciado en otro modo, se puede usar igual
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
        }
    }

    pub fn get() -> windows::core::Result<(u8, bool)> {
        let ep = endpoint()?;
        unsafe {
            let level = ep.GetMasterVolumeLevelScalar()?;
            let muted = ep.GetMute()?.as_bool();
            Ok(((level * 100.0).round() as u8, muted))
        }
    }

    pub fn set_level(percent: u8) -> windows::core::Result<()> {
        let ep = endpoint()?;
        unsafe { ep.SetMasterVolumeLevelScalar(f32::from(percent.min(100)) / 100.0, std::ptr::null()) }
    }

    pub fn set_mute(mute: bool) -> windows::core::Result<()> {
        let ep = endpoint()?;
        unsafe { ep.SetMute(mute, std::ptr::null()) }
    }
}

#[derive(Serialize)]
pub struct Volume { level: u8, muted: bool }

#[tauri::command]
pub async fn get_volume() -> Result<Volume, String> {
    audio::get().map(|(level, muted)| Volume { level, muted }).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_volume(level: u8) -> Result<(), String> {
    audio::set_level(level).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_mute(muted: bool) -> Result<(), String> {
    audio::set_mute(muted).map_err(|e| e.to_string())
}

// ---------- Brillo (panel del portátil, vía WMI) ----------

pub(crate) fn brightness_get() -> Result<Option<u8>, String> {
    let out = powershell("(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness -ErrorAction SilentlyContinue | Select-Object -First 1).CurrentBrightness")?;
    Ok(out.trim().parse().ok())
}

pub(crate) fn brightness_set(level: u8) -> Result<(), String> {
    let level = level.min(100);
    powershell(&format!(
        "Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods | Invoke-CimMethod -MethodName WmiSetBrightness -Arguments @{{Timeout=0; Brightness={level}}} | Out-Null"
    ))
    .map(|_| ())
}

#[tauri::command]
pub async fn get_brightness() -> Result<Option<u8>, String> {
    brightness_get()
}

#[tauri::command]
pub async fn set_brightness(level: u8) -> Result<(), String> {
    brightness_set(level)
}

// ---------- Batería y energía ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Battery { has_battery: bool, percent: Option<u8>, plugged_in: bool }

#[tauri::command(async)]
pub fn get_battery() -> Result<Battery, String> {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut s = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut s) }.map_err(|e| e.to_string())?;
    Ok(Battery {
        has_battery: s.BatteryFlag & 128 == 0,
        percent: (s.BatteryLifePercent <= 100).then_some(s.BatteryLifePercent),
        plugged_in: s.ACLineStatus == 1,
    })
}

#[derive(Serialize)]
pub struct PowerPlan { guid: String, name: String, active: bool }

/// Interpreta la salida de `powercfg /list` (vale para cualquier idioma).
fn parse_power_plans(text: &str) -> Vec<PowerPlan> {
    text.lines()
        .filter_map(|line| {
            let (open, close) = (line.find('(')?, line.rfind(')')?);
            let guid = line[..open].split_whitespace().last()?.to_string();
            (guid.len() == 36).then(|| PowerPlan {
                guid,
                name: line[open + 1..close].trim().to_string(),
                active: line.trim_end().ends_with('*'),
            })
        })
        .collect()
}

pub(crate) fn power_plans() -> Result<Vec<PowerPlan>, String> {
    run("powercfg", &["/list"]).map(|t| parse_power_plans(&t))
}

#[tauri::command]
pub async fn get_power_plans() -> Result<Vec<PowerPlan>, String> {
    power_plans()
}

#[tauri::command]
pub async fn set_power_plan(guid: String) -> Result<(), String> {
    plan_set(guid)
}

pub(crate) fn plan_set(guid: String) -> Result<(), String> {
    // Solo GUIDs que existen de verdad, nada que venga sin comprobar
    let plans = run("powercfg", &["/list"]).map(|t| parse_power_plans(&t))?;
    if !plans.iter().any(|p| p.guid == guid) {
        return Err(crate::i18n::tr("settings.plan_not_found").into());
    }
    run("powercfg", &["/setactive", &guid]).map(|_| ())
}

#[tauri::command(async)]
pub fn power_action(action: String) -> Result<(), String> {
    let (exe, args): (&str, &[&str]) = match action.as_str() {
        "apagar" => ("shutdown", &["/s", "/t", "0"]),
        "reiniciar" => ("shutdown", &["/r", "/t", "0"]),
        "suspender" => ("rundll32.exe", &["powrprof.dll,SetSuspendState", "0,1,0"]),
        _ => return Err(crate::i18n::tr("common.bad_action").into()),
    };
    Command::new(exe).args(args).creation_flags(NO_WINDOW).spawn().map(|_| ()).map_err(|e| e.to_string())
}

// ---------- Wi-Fi (netsh) ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Wifi { has_adapter: bool, connected: bool, ssid: Option<String>, signal: Option<u8>, saved: Vec<String> }

/// Valor tras los dos puntos de una línea "Clave : valor"
fn after_colon(line: &str) -> Option<String> {
    line.split_once(':').map(|(_, v)| v.trim().to_string()).filter(|v| !v.is_empty())
}

fn parse_interfaces(text: &str) -> (bool, Option<String>, Option<u8>) {
    let has_adapter = text.lines().any(|l| l.trim_start().starts_with("GUID"));
    let ssid = text
        .lines()
        .find(|l| l.trim_start().starts_with("SSID") && !l.trim_start().starts_with("SSID de") && !l.contains("BSSID"))
        .and_then(after_colon);
    let signal = text
        .lines()
        .filter_map(after_colon)
        .find_map(|v| v.strip_suffix('%').and_then(|n| n.trim().parse().ok()));
    (has_adapter, ssid, signal)
}

fn parse_profiles(text: &str) -> Vec<String> {
    // Las líneas de perfil van tras la línea de guiones y llevan " : nombre"
    text.lines()
        .skip_while(|l| !l.trim_start().starts_with("---"))
        .filter(|l| l.contains(" : "))
        .filter_map(after_colon)
        .collect()
}

#[tauri::command]
pub async fn get_wifi() -> Result<Wifi, String> {
    wifi_status()
}

pub(crate) fn wifi_status() -> Result<Wifi, String> {
    let interfaces = run("netsh", &["wlan", "show", "interfaces"]).unwrap_or_default();
    let (has_adapter, ssid, signal) = parse_interfaces(&interfaces);
    let saved = run("netsh", &["wlan", "show", "profiles"]).map(|t| parse_profiles(&t)).unwrap_or_default();
    Ok(Wifi { has_adapter, connected: ssid.is_some(), ssid, signal, saved })
}

#[tauri::command]
pub async fn connect_wifi(name: String) -> Result<(), String> {
    // Solo redes ya guardadas en Windows
    let saved = run("netsh", &["wlan", "show", "profiles"]).map(|t| parse_profiles(&t))?;
    if !saved.contains(&name) {
        return Err(crate::i18n::tr("settings.wifi_not_saved").into());
    }
    run("netsh", &["wlan", "connect", &format!("name={name}")]).map(|_| ())
}

#[tauri::command]
pub async fn disconnect_wifi() -> Result<(), String> {
    run("netsh", &["wlan", "disconnect"]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_planes_de_energia() {
        let t = "GUID de plan de energía: 381b4222-f694-41f0-9685-ff5bb260df2e  (Equilibrado)\nGUID de plan de energía: e1477891-417f-46d8-99a9-3ae9d087202f  (Juego - Maximo rendimiento) *";
        let p = parse_power_plans(t);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].name, "Equilibrado");
        assert!(!p[0].active);
        assert!(p[1].active);
    }

    #[test]
    fn lee_wifi_conectado_y_desconectado() {
        let on = "    GUID                   : 403753d6-6f2c\n    SSID                   : MiCasa\n    BSSID                  : aa:bb\n    Señal                  : 87%\n";
        assert_eq!(parse_interfaces(on), (true, Some("MiCasa".into()), Some(87)));
        let off = "    GUID                   : 403753d6\n    Estado                  : desconectado\n";
        assert_eq!(parse_interfaces(off), (true, None, None));
        assert_eq!(parse_interfaces("No hay interfaz"), (false, None, None));
    }

    /// Lecturas reales del sistema (solo leen, no cambian nada)
    #[test]
    fn lecturas_reales_del_pc() {
        let b = get_battery().expect("batería");
        println!("bateria: has={} pct={:?} enchufado={}", b.has_battery, b.percent, b.plugged_in);
        let (lvl, muted) = audio::get().expect("volumen");
        assert!(lvl <= 100);
        println!("volumen: {lvl} silencio={muted}");
        let plans = parse_power_plans(&run("powercfg", &["/list"]).unwrap());
        assert!(plans.iter().any(|p| p.active), "debe haber un plan activo");
    }

    #[test]
    fn lee_redes_guardadas() {
        let t = "Perfiles de usuario\n-------------------\n    Perfil de todos los usuarios     : DIGIFIBRA-URxC\n    Perfil de todos los usuarios     : Movil 5G\n";
        assert_eq!(parse_profiles(t), vec!["DIGIFIBRA-URxC", "Movil 5G"]);
    }
}
