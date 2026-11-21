//! Arrancar Windows directamente en la consola (en lugar del escritorio), solo para este usuario.
//! Se guarda en HKCU\...\Winlogon\Shell y se deshace borrando ese valor.
//! Vuelta atrás: Ajustes → Arranque, el botón Escritorio, o `consola.exe --restaurar-escritorio`.

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::process::Command;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;

fn winlogon_path() -> String {
    // Las pruebas usan otra clave para no tocar el arranque real
    std::env::var("CONSOLA_WINLOGON_KEY").unwrap_or_else(|_| "Software\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon".into())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellStatus { enabled: bool, exe: String, running_as_shell: bool }

fn current_shell() -> Option<String> {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(winlogon_path(), KEY_READ).ok()?.get_value("Shell").ok()
}

/// ¿Está funcionando el escritorio de Windows (explorer.exe)?
pub fn explorer_running() -> bool {
    Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq explorer.exe", "/NH"])
        .creation_flags(0x0800_0000)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains("explorer.exe"))
        .unwrap_or(true)
}

pub fn set_shell(enabled: bool) -> Result<(), String> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(winlogon_path()).map_err(|e| e.to_string())?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value("Shell", &format!("\"{}\"", exe.display())).map_err(|e| e.to_string())
    } else {
        match key.delete_value("Shell") {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[tauri::command(async)]
pub fn shell_status() -> ShellStatus {
    let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
    ShellStatus {
        enabled: current_shell().is_some_and(|s| s.to_lowercase().contains("consola")),
        running_as_shell: !explorer_running(),
        exe,
    }
}

#[tauri::command(async)]
pub fn shell_set(enabled: bool) -> Result<ShellStatus, String> {
    set_shell(enabled)?;
    Ok(shell_status())
}

/// Sin escritorio nadie arranca los programas de inicio: los lanzamos nosotros
pub fn run_startup_items() {
    let sources = [
        (HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
        (HKEY_LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
        (HKEY_LOCAL_MACHINE, "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run"),
    ];
    for (hive, path) in sources {
        let Ok(key) = RegKey::predef(hive).open_subkey_with_flags(path, KEY_READ) else { continue };
        for (name, _) in key.enum_values().flatten() {
            let Ok(cmd) = key.get_value::<String, _>(&name) else { continue };
            let _ = Command::new("cmd").raw_arg(format!("/C start \"\" {cmd}")).creation_flags(0x0800_0000).spawn();
        }
    }
    // Carpeta Inicio del usuario
    if let Ok(appdata) = std::env::var("APPDATA") {
        let dir = std::path::Path::new(&appdata).join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let _ = Command::new("explorer").arg(e.path()).spawn();
        }
    }
}

/// Ir al escritorio: si la consola es el "escritorio", arranca explorer antes de cerrarse
pub fn open_windows_desktop() {
    if !explorer_running() {
        let _ = Command::new("explorer.exe").spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activar_y_desactivar_en_clave_de_prueba() {
        unsafe { std::env::set_var("CONSOLA_WINLOGON_KEY", "Software\\Consola\\PruebaWinlogon") };
        set_shell(true).unwrap();
        assert!(current_shell().unwrap().contains("\""));
        set_shell(false).unwrap();
        assert!(current_shell().is_none());
        set_shell(false).unwrap(); // desactivar dos veces no falla
        let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all("Software\\Consola");
    }
}
