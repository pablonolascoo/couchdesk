//! Carpeta de CouchDesk: %LOCALAPPDATA%\CouchDesk (el programa y sus datos).
//! Hasta la 1.1.0 la app se llamaba «Consola» y vivía en %LOCALAPPDATA%\Consola: al arrancar,
//! `migrate_legacy()` mueve los datos de allí (emuladores, Claude, fondos…) sin copiar nada.

use std::path::{Path, PathBuf};

pub const APP_DIR: &str = "CouchDesk";
pub const LEGACY_DIR: &str = "Consola";

fn local_appdata() -> PathBuf {
    PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into()))
}

/// Carpeta de datos de CouchDesk
pub fn data_dir() -> PathBuf {
    local_appdata().join(APP_DIR)
}

/// Lo que nunca se mueve de la carpeta antigua: el programa viejo y su desinstalador
fn is_old_program(name: &str) -> bool {
    let n = name.to_lowercase();
    n == "consola.exe" || n == "uninstall.exe" || n == "couchdesk.scr"
}

/// Mueve los datos de `old` a `new`. Lo que ya exista en `new` se deja (gana lo nuevo).
/// Devuelve los nombres movidos.
pub fn migrate(old: &Path, new: &Path) -> Vec<String> {
    let mut moved = Vec::new();
    let Ok(entries) = std::fs::read_dir(old) else { return moved };
    let _ = std::fs::create_dir_all(new);
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if is_old_program(&name) {
            continue;
        }
        let target = new.join(&name);
        if target.exists() {
            continue;
        }
        if std::fs::rename(e.path(), &target).is_ok() {
            moved.push(name);
        }
    }
    // Si solo queda el programa viejo (o nada), la carpeta antigua sobra
    let leftovers: Vec<_> = std::fs::read_dir(old).map(|r| r.flatten().map(|e| e.file_name()).collect()).unwrap_or_default();
    if leftovers.iter().all(|n| is_old_program(&n.to_string_lossy())) {
        for n in leftovers {
            let _ = std::fs::remove_file(old.join(n));
        }
        let _ = std::fs::remove_dir(old);
    }
    moved
}

/// Al arrancar: trae los datos de la antigua «Consola» (solo si este programa ya no vive allí)
pub fn migrate_legacy() {
    remove_legacy_uninstall_entry();
    let old = local_appdata().join(LEGACY_DIR);
    let me = std::env::current_exe().unwrap_or_default();
    if !old.exists() || me.starts_with(&old) || me.display().to_string().to_lowercase().contains("\\target\\") {
        return;
    }
    // xTend corre desde la carpeta antigua: con él abierto, Windows no deja moverla
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("taskkill").args(["/IM", "xtend.exe", "/F"]).creation_flags(0x0800_0000).output();
    std::thread::sleep(std::time::Duration::from_millis(400));
    let moved = migrate(&old, &data_dir());
    if !moved.is_empty() {
        log::info!("Datos traídos de Consola: {moved:?}");
    }
}

/// Quita «Consola» de Agregar o quitar programas si su desinstalador ya no existe
fn remove_legacy_uninstall_entry() {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Consola";
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(key) = hkcu.open_subkey(path) else { return };
    let uninstaller: String = key.get_value("UninstallString").unwrap_or_default();
    let file = uninstaller.trim().trim_matches('"').to_string();
    if !file.is_empty() && !Path::new(&file).exists() {
        drop(key);
        let _ = hkcu.delete_subkey_all(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mueve_los_datos_y_deja_el_programa_viejo() {
        let base = std::env::temp_dir().join(format!("couchdesk-migra-{}", std::process::id()));
        let (old, new) = (base.join("Consola"), base.join("CouchDesk"));
        std::fs::create_dir_all(old.join("emuladores").join("pcsx2")).unwrap();
        std::fs::write(old.join("emuladores").join("pcsx2").join("bios.bin"), "x").unwrap();
        std::fs::write(old.join("consola.exe"), "viejo").unwrap();
        std::fs::write(old.join("mando.txt"), "viejo").unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(new.join("mando.txt"), "nuevo").unwrap();

        let moved = migrate(&old, &new);
        assert_eq!(moved, vec!["emuladores".to_string()]);
        assert!(new.join("emuladores").join("pcsx2").join("bios.bin").exists());
        assert_eq!(std::fs::read_to_string(new.join("mando.txt")).unwrap(), "nuevo");
        // mando.txt viejo sigue allí, así que la carpeta antigua no se borra
        assert!(old.join("mando.txt").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn borra_la_carpeta_vieja_si_solo_queda_el_programa() {
        let base = std::env::temp_dir().join(format!("couchdesk-migra2-{}", std::process::id()));
        let (old, new) = (base.join("Consola"), base.join("CouchDesk"));
        std::fs::create_dir_all(old.join("claude")).unwrap();
        std::fs::write(old.join("consola.exe"), "viejo").unwrap();
        std::fs::write(old.join("uninstall.exe"), "viejo").unwrap();
        migrate(&old, &new);
        assert!(new.join("claude").exists());
        assert!(!old.exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
