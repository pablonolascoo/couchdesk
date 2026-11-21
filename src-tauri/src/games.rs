//! Detección de juegos instalados (Steam, Epic, GOG) y lanzamiento.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub title: String,
    /// "steam" | "epic" | "gog"
    pub store: String,
    pub install_dir: String,
    /// Portada local (ruta de archivo), si la tienda la guarda
    pub cover: Option<String>,
    /// Imagen panorámica para el fondo, si la tienda la guarda
    pub hero: Option<String>,
    /// Icono del programa (PNG local) cuando no hay portada
    pub icon: Option<String>,
    /// Última vez jugado (segundos Unix), si se conoce
    pub last_played: Option<i64>,
    pub size_bytes: Option<u64>,
    /// Lo que se abre para lanzar el juego (URI de la tienda o .exe). Nunca viene de la web.
    #[serde(skip)]
    pub launch: String,
}

/// Lista detectada en la última búsqueda; lanzar solo acepta ids de aquí.
#[derive(Default)]
pub struct GameCache(pub Mutex<Vec<Game>>);

// ---------- utilidades VDF/ACF (formato de texto de Steam) ----------

/// Devuelve todos los valores de `"clave"  "valor"` en un texto VDF.
fn vdf_values(text: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('"').collect();
        // línea típica: \t"clave"\t\t"valor"
        if parts.len() >= 4 && parts[1].eq_ignore_ascii_case(key) {
            out.push(parts[3].replace("\\\\", "\\"));
        }
    }
    out
}

fn vdf_value(text: &str, key: &str) -> Option<String> {
    vdf_values(text, key).into_iter().next()
}

// ---------- Steam ----------

fn steam_root() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CONSOLA_STEAM_PATH") {
        return Some(PathBuf::from(p));
    }
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let key = hkcu.open_subkey("Software\\Valve\\Steam").ok()?;
    let path: String = key.get_value("SteamPath").ok()?;
    let p = PathBuf::from(path.replace('/', "\\"));
    p.exists().then_some(p)
}

fn steam_cover(root: &Path, appid: &str) -> Option<String> {
    steam_art(root, appid, "library_600x900")
}

/// Imagen panorámica del juego (fondo de la pantalla de inicio)
fn steam_hero(root: &Path, appid: &str) -> Option<String> {
    steam_art(root, appid, "library_hero")
}

fn steam_art(root: &Path, appid: &str, kind: &str) -> Option<String> {
    let cache = root.join("appcache").join("librarycache");
    let legacy = cache.join(format!("{appid}_{kind}.jpg"));
    if legacy.exists() {
        return Some(legacy.to_string_lossy().into_owned());
    }
    // Formato nuevo: librarycache/<appid>/[<hash>/]library_600x900.jpg
    let dir = cache.join(appid);
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                // "library_hero_blur" no vale como fondo nítido
                n.starts_with(kind) && !n.contains("blur")
            }) {
                return Some(p.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn steam_games() -> Vec<Game> {
    let Some(root) = steam_root() else { return vec![] };
    let mut libraries = vec![root.clone()];
    if let Ok(text) = fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
        for p in vdf_values(&text, "path") {
            let p = PathBuf::from(p);
            if !libraries.contains(&p) {
                libraries.push(p);
            }
        }
    }
    // Herramientas de Steam que no son juegos
    const SKIP: [&str; 3] = ["228980", "1070560", "1391110"];
    let mut games = Vec::new();
    for lib in libraries {
        let apps = lib.join("steamapps");
        let Ok(entries) = fs::read_dir(&apps) else { continue };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                continue;
            }
            let Ok(text) = fs::read_to_string(e.path()) else { continue };
            let (Some(appid), Some(title)) = (vdf_value(&text, "appid"), vdf_value(&text, "name")) else { continue };
            if SKIP.contains(&appid.as_str()) || title.starts_with("Steamworks") || title.starts_with("Proton") {
                continue;
            }
            let installdir = vdf_value(&text, "installdir").unwrap_or_default();
            games.push(Game {
                id: format!("steam-{appid}"),
                cover: steam_cover(&root, &appid),
                hero: steam_hero(&root, &appid),
                icon: None,
                launch: format!("steam://rungameid/{appid}"),
                title,
                store: "steam".into(),
                install_dir: apps.join("common").join(installdir).to_string_lossy().into_owned(),
                last_played: vdf_value(&text, "LastPlayed").and_then(|v| v.parse().ok()).filter(|v| *v > 0),
                size_bytes: vdf_value(&text, "SizeOnDisk").and_then(|v| v.parse().ok()),
            });
        }
    }
    games
}

// ---------- Epic Games ----------

fn epic_games() -> Vec<Game> {
    let dir = std::env::var("CONSOLA_EPIC_MANIFESTS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests"));
    let Ok(entries) = fs::read_dir(dir) else { return vec![] };
    let mut games = Vec::new();
    for e in entries.flatten() {
        if e.path().extension().is_none_or(|x| x != "item") {
            continue;
        }
        let Ok(text) = fs::read_to_string(e.path()) else { continue };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let app = s("AppName");
        // Saltar DLC e instalaciones a medias
        if app.is_empty() || app != s("MainGameAppName") || v.get("bIsIncompleteInstall").and_then(|x| x.as_bool()) == Some(true) {
            continue;
        }
        games.push(Game {
            id: format!("epic-{app}"),
            title: s("DisplayName"),
            store: "epic".into(),
            install_dir: s("InstallLocation"),
            cover: None,
            hero: None,
            icon: None,
            last_played: None,
            size_bytes: v.get("InstallSize").and_then(|x| x.as_u64()),
            launch: format!(
                "com.epicgames.launcher://apps/{}%3A{}%3A{}?action=launch&silent=true",
                s("CatalogNamespace"), s("CatalogItemId"), app
            ),
        });
    }
    games
}

// ---------- GOG ----------

fn gog_games() -> Vec<Game> {
    let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    let Ok(root) = hklm.open_subkey("SOFTWARE\\WOW6432Node\\GOG.com\\Games") else { return vec![] };
    let mut games = Vec::new();
    for id in root.enum_keys().flatten() {
        let Ok(k) = root.open_subkey(&id) else { continue };
        let get = |n: &str| k.get_value::<String, _>(n).unwrap_or_default();
        let exe = get("exe");
        if exe.is_empty() || !Path::new(&exe).exists() {
            continue;
        }
        games.push(Game {
            id: format!("gog-{id}"),
            title: get("gameName"),
            store: "gog".into(),
            install_dir: get("path"),
            cover: None,
            hero: None,
            icon: None,
            last_played: None,
            size_bytes: None,
            launch: exe,
        });
    }
    games
}

// ---------- Xbox / Game Pass (paquetes de la Microsoft Store con MicrosoftGame.config) ----------

const XBOX_PS: &str = r#"Get-AppxPackage | Where-Object { -not $_.IsFramework -and $_.InstallLocation -and (Test-Path (Join-Path $_.InstallLocation 'MicrosoftGame.config')) } | ForEach-Object { $m = Get-AppxPackageManifest $_; $cfg = [xml](Get-Content (Join-Path $_.InstallLocation 'MicrosoftGame.config') -Raw); [pscustomobject]@{ pfn = $_.PackageFamilyName; app = @($m.Package.Applications.Application)[0].Id; name = $m.Package.Properties.DisplayName; alt = $cfg.Game.ShellVisuals.DefaultDisplayName; pkg = $_.Name; dir = $_.InstallLocation } } | ConvertTo-Json -Compress"#;

#[derive(serde::Deserialize)]
struct XboxRaw { pfn: String, app: Option<String>, name: Option<String>, alt: Option<String>, pkg: String, dir: String }

/// Nombre legible: el del manifiesto, si no es una referencia a recursos (ms-resource:...)
fn xbox_title(r: &XboxRaw) -> String {
    [&r.name, &r.alt]
        .into_iter()
        .flatten()
        .find(|n| !n.is_empty() && !n.starts_with("ms-resource"))
        .cloned()
        .unwrap_or_else(|| r.pkg.rsplit('.').next().unwrap_or(&r.pkg).to_string())
}

pub fn parse_xbox(json: &str) -> Vec<Game> {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return vec![];
    }
    // PowerShell devuelve un objeto suelto si solo hay uno
    let raws: Vec<XboxRaw> = if trimmed.starts_with('[') {
        serde_json::from_str(trimmed).unwrap_or_default()
    } else {
        serde_json::from_str::<XboxRaw>(trimmed).map(|r| vec![r]).unwrap_or_default()
    };
    raws.into_iter()
        .map(|r| Game {
            id: format!("xbox-{}", r.pfn),
            title: xbox_title(&r),
            store: "xbox".into(),
            install_dir: r.dir.clone(),
            cover: None,
            hero: None,
            icon: None,
            last_played: None,
            size_bytes: None,
            launch: format!("shell:AppsFolder\\{}!{}", r.pfn, r.app.as_deref().unwrap_or("Game")),
        })
        .collect()
}

fn xbox_games() -> Vec<Game> {
    use std::os::windows::process::CommandExt;
    let out = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", XBOX_PS])
        .creation_flags(0x0800_0000)
        .output();
    out.map(|o| parse_xbox(&String::from_utf8_lossy(&o.stdout))).unwrap_or_default()
}

// ---------- Comandos ----------

/// Solo los de tienda (Steam, Epic, GOG, Xbox)
pub fn detect_store() -> Vec<Game> {
    [steam_games(), epic_games(), gog_games(), xbox_games()].concat()
}

/// Todo el PC, repartido en juegos y aplicaciones, con iconos y las marcas del usuario
pub fn detect_everything_from(store: Vec<Game>) -> (Vec<Game>, Vec<crate::apps::App>) {
    let (mut games, apps) = crate::apps::split(store, crate::apps::detect(), &crate::apps::load_marks());
    crate::apps::icons::fill_games(&mut games);
    // Primero los jugados más recientemente, luego por nombre
    games.sort_by(|a, b| b.last_played.cmp(&a.last_played).then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase())));
    (games, apps)
}

pub fn detect_all() -> Vec<Game> {
    detect_everything_from(detect_store()).0
}

pub fn launch(g: &Game) -> Result<(), String> {
    Command::new("explorer").arg(&g.launch).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Lo que se puede abrir: juegos y aplicaciones. Lanzar solo acepta ids de aquí.
#[derive(Default)]
pub struct Launchables(pub Mutex<std::collections::HashMap<String, String>>);

fn remember(state: &Launchables, games: &[Game], apps: &[crate::apps::App], all_store: &[Game]) {
    let mut m = state.0.lock().unwrap();
    m.clear();
    for g in games.iter().chain(all_store) {
        m.insert(g.id.clone(), g.launch.clone());
    }
    for a in apps.iter().filter(|a| !a.app_id.is_empty()) {
        m.insert(a.id.clone(), crate::apps::launch_uri(&a.app_id));
    }
}

#[derive(Serialize)]
pub struct Everything {
    pub games: Vec<Game>,
    pub apps: Vec<crate::apps::App>,
}

#[tauri::command]
pub async fn list_everything(cache: tauri::State<'_, GameCache>, launch: tauri::State<'_, Launchables>) -> Result<Everything, String> {
    let (store, (games, apps)) = tauri::async_runtime::spawn_blocking(|| {
        let store = detect_store();
        (store.clone(), detect_everything_from(store))
    })
    .await
    .map_err(|e| e.to_string())?;
    remember(&launch, &games, &apps, &store);
    *cache.0.lock().unwrap() = games.clone();
    Ok(Everything { games, apps })
}

#[tauri::command(async)]
pub fn mark_as_game(id: String, game: bool) -> Result<(), String> {
    crate::apps::set_mark(&id, game)
}

#[tauri::command]
pub async fn list_games(cache: tauri::State<'_, GameCache>, launch: tauri::State<'_, Launchables>) -> Result<Vec<Game>, String> {
    Ok(list_everything(cache, launch).await?.games)
}

#[tauri::command(async)]
pub fn launch_game(id: String, cache: tauri::State<GameCache>, apps: tauri::State<Launchables>) -> Result<(), String> {
    let known = cache.0.lock().unwrap().iter().find(|g| g.id == id).map(|g| g.launch.clone());
    let launch = known.or_else(|| apps.0.lock().unwrap().get(&id).cloned()).ok_or("No encontrado")?;
    // explorer abre tanto URIs de tienda (steam://, com.epicgames...) como .exe
    Command::new("explorer").arg(&launch).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_valores_vdf() {
        let t = "\"AppState\"\n{\n\t\"appid\"\t\t\"620\"\n\t\"name\"\t\t\"Portal 2\"\n\t\"LastPlayed\"\t\t\"1700000000\"\n}";
        assert_eq!(vdf_value(t, "appid").as_deref(), Some("620"));
        assert_eq!(vdf_value(t, "name").as_deref(), Some("Portal 2"));
        assert_eq!(vdf_value(t, "lastplayed").as_deref(), Some("1700000000"));
        assert_eq!(vdf_value(t, "nada"), None);
    }

    /// Biblioteca de Steam falsa: dos juegos, una herramienta a ignorar y una portada.
    #[test]
    fn detecta_biblioteca_steam_de_prueba() {
        let root = std::env::temp_dir().join(format!("consola-steam-test-{}", std::process::id()));
        let apps = root.join("steamapps");
        fs::create_dir_all(&apps).unwrap();
        let acf = |id: &str, name: &str, last: &str| {
            format!("\"AppState\"\n{{\n\t\"appid\"\t\t\"{id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"installdir\"\t\t\"{name}\"\n\t\"LastPlayed\"\t\t\"{last}\"\n\t\"SizeOnDisk\"\t\t\"1000\"\n}}")
        };
        fs::write(apps.join("appmanifest_620.acf"), acf("620", "Portal 2", "1700000000")).unwrap();
        fs::write(apps.join("appmanifest_400.acf"), acf("400", "Portal", "1800000000")).unwrap();
        fs::write(apps.join("appmanifest_228980.acf"), acf("228980", "Steamworks Common Redistributables", "0")).unwrap();
        let cover_dir = root.join("appcache").join("librarycache").join("620").join("abc123");
        fs::create_dir_all(&cover_dir).unwrap();
        fs::write(cover_dir.join("library_600x900.jpg"), b"jpg").unwrap();

        unsafe { std::env::set_var("CONSOLA_STEAM_PATH", &root) };
        let mut games = steam_games();
        games.sort_by(|a, b| a.title.cmp(&b.title));
        let _ = fs::remove_dir_all(&root);

        assert_eq!(games.len(), 2, "la herramienta de Steam no debe salir");
        assert_eq!(games[0].title, "Portal");
        assert_eq!(games[1].id, "steam-620");
        assert_eq!(games[1].launch, "steam://rungameid/620");
        assert_eq!(games[1].last_played, Some(1700000000));
        assert!(games[1].cover.as_deref().unwrap().ends_with("library_600x900.jpg"));
        assert!(games[0].cover.is_none());
    }

    #[test]
    fn juegos_de_xbox() {
        let one = r#"{"pfn":"Microsoft.Forza_8wekyb3d8bbwe","app":"Game","name":"ms-resource:Title","alt":"Forza Horizon 5","pkg":"Microsoft.Forza","dir":"C:\\XboxGames\\Forza"}"#;
        let g = parse_xbox(one);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].title, "Forza Horizon 5");
        assert_eq!(g[0].launch, "shell:AppsFolder\\Microsoft.Forza_8wekyb3d8bbwe!Game");
        let many = format!("[{one},{}]", one.replace("Forza_8", "Halo_8"));
        assert_eq!(parse_xbox(&many).len(), 2);
        assert!(parse_xbox("").is_empty());
        // En este PC (sin Game Pass) no debe fallar
        let _ = xbox_games();
    }

    #[test]
    fn rutas_con_barras_dobles() {
        let t = "\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"";
        assert_eq!(vdf_values(t, "path"), vec!["D:\\SteamLibrary".to_string()]);
    }
}
