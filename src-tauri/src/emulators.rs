//! Emuladores: catálogo, instalación desde la web oficial de cada proyecto,
//! actualización automática con vuelta atrás, guía de BIOS y "¿irá bien en mi PC?".
//! Nunca descarga juegos, BIOS ni claves.

use crate::hardware::{get_hardware, Hardware};
use crate::i18n::{lookup, tr, trf};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy)]
enum Source {
    Github { repo: &'static str, must: &'static [&'static str] },
    Forgejo { host: &'static str, repo: &'static str, must: &'static [&'static str] },
    Dolphin,
}

#[derive(Clone, Copy)]
struct Req { threads: u32, ram_gb: f64, gpu_tier: u8, vram_gb: f64, avx2: bool }

const fn req(threads: u32, ram_gb: f64, gpu_tier: u8, vram_gb: f64, avx2: bool) -> Req {
    Req { threads, ram_gb, gpu_tier, vram_gb, avx2 }
}

struct Emu {
    id: &'static str,
    name: &'static str,
    console: &'static str,
    trust: &'static str, // "fiable" | "experimental"
    source: Source,
    exe_hint: &'static str,
    min: Req,
    rec: Req,
    bios: &'static str,
    note: &'static str,
}

const PS5_MIN: Req = req(8, 16.0, 3, 8.0, true);
const PS5_REC: Req = req(12, 32.0, 4, 12.0, true);
const SW_MIN: Req = req(6, 8.0, 2, 4.0, false);
const SW_REC: Req = req(8, 16.0, 3, 6.0, true);
const SW2_MIN: Req = req(6, 16.0, 3, 8.0, true);
const SW2_REC: Req = req(8, 16.0, 4, 12.0, true);

const CATALOG: &[Emu] = &[
    Emu { id: "duckstation", name: "DuckStation", console: "PlayStation", trust: "fiable",
        source: Source::Github { repo: "stenzek/duckstation", must: &["windows", "x64"] }, exe_hint: "duckstation",
        min: req(2, 2.0, 0, 0.0, false), rec: req(4, 4.0, 1, 1.0, false),
        bios: "Necesita la BIOS de tu PS1 (por ejemplo scph5501.bin). Cópiala en la carpeta «bios» del emulador; el asistente del primer arranque la busca allí.",
        note: "" },
    Emu { id: "pcsx2", name: "PCSX2", console: "PlayStation 2", trust: "fiable",
        source: Source::Github { repo: "PCSX2/pcsx2", must: &["windows", "x64"] }, exe_hint: "pcsx2",
        min: req(4, 4.0, 1, 2.0, false), rec: req(6, 8.0, 2, 4.0, true),
        bios: "Necesita la BIOS de tu PS2 (archivo .bin sacado de tu consola). Ponla en la carpeta «bios»; PCSX2 la detecta en el asistente inicial.",
        note: "" },
    Emu { id: "rpcs3", name: "RPCS3", console: "PlayStation 3", trust: "fiable",
        source: Source::Github { repo: "RPCS3/rpcs3-binaries-win", must: &["win64"] }, exe_hint: "rpcs3",
        min: req(4, 8.0, 1, 2.0, false), rec: req(8, 16.0, 3, 6.0, true),
        bios: "Necesita el firmware oficial PS3UPDAT.PUP, gratis en la web de PlayStation (actualización del sistema de PS3). En RPCS3: Archivo → Instalar firmware.",
        note: "Tira mucho del procesador: los juegos ligeros van, los pesados no." },
    Emu { id: "shadps4", name: "shadPS4", console: "PlayStation 4", trust: "experimental",
        source: Source::Github { repo: "shadps4-emu/shadPS4", must: &["win64"] }, exe_hint: "shadps4",
        min: req(6, 8.0, 2, 4.0, true), rec: req(8, 16.0, 3, 8.0, true),
        bios: "No necesita BIOS. Opcional: módulos de sistema de tu PS4 en la carpeta «sys_modules».",
        note: "Pocos juegos jugables todavía." },
    Emu { id: "kytyps5", name: "KytyPS5", console: "PlayStation 5", trust: "experimental",
        source: Source::Github { repo: "KytyPS5/KytyPS5", must: &["win"] }, exe_hint: "kyty",
        min: PS5_MIN, rec: PS5_REC, bios: "No necesita BIOS ni firmware.", note: "" },
    Emu { id: "sharpemu", name: "SharpEmu", console: "PlayStation 5", trust: "experimental",
        source: Source::Github { repo: "sharpemu/sharpemu", must: &["win"] }, exe_hint: "sharpemu",
        min: PS5_MIN, rec: PS5_REC, bios: "No necesita BIOS. También abre algunos juegos de PS4.", note: "" },
    Emu { id: "anyps5", name: "AnyPS5", console: "PlayStation 5", trust: "experimental",
        source: Source::Github { repo: "boykopovar/AnyPS5", must: &["prx-windows-v"] }, exe_hint: "relinker",
        min: PS5_MIN, rec: PS5_REC, bios: "No necesita BIOS.",
        note: "No es un emulador: convierte el juego en un programa de Windows." },
    Emu { id: "ares", name: "ares", console: "Nintendo 64", trust: "fiable",
        source: Source::Github { repo: "ares-emulator/ares", must: &["windows"] }, exe_hint: "ares",
        min: req(2, 4.0, 0, 0.0, false), rec: req(4, 8.0, 1, 1.0, false),
        bios: "No necesita BIOS para Nintendo 64.", note: "" },
    Emu { id: "melonds", name: "melonDS", console: "Nintendo DS", trust: "fiable",
        source: Source::Github { repo: "melonDS-emu/melonDS", must: &["windows"] }, exe_hint: "melonds",
        min: req(2, 2.0, 0, 0.0, false), rec: req(4, 4.0, 0, 0.0, false),
        bios: "Opcional: bios7.bin, bios9.bin y firmware.bin de tu DS (Ajustes → Emulación). Sin ellos usa una alternativa libre.",
        note: "" },
    Emu { id: "dolphin", name: "Dolphin", console: "Wii y GameCube", trust: "fiable",
        source: Source::Dolphin, exe_hint: "dolphin",
        min: req(4, 4.0, 1, 1.0, false), rec: req(4, 8.0, 2, 2.0, false),
        bios: "No necesita BIOS.", note: "" },
    Emu { id: "eden", name: "Eden", console: "Nintendo Switch", trust: "fiable",
        source: Source::Forgejo { host: "git.eden-emu.dev", repo: "eden-emu/eden", must: &["windows"] }, exe_hint: "eden",
        min: SW_MIN, rec: SW_REC,
        bios: "Necesita prod.keys y el firmware de TU Switch. En Eden: Herramientas → Instalar claves / Instalar firmware.",
        note: "" },
    Emu { id: "novas", name: "NovaS", console: "Nintendo Switch 2", trust: "experimental",
        source: Source::Github { repo: "nova5emu/novas", must: &["portable"] }, exe_hint: "novas",
        min: SW2_MIN, rec: SW2_REC,
        bios: "Necesita prod.keys (y title.keys) y el firmware de TU Switch 2: botones «Claves» y «Firmware» del propio NovaS.",
        note: "Proyecto pequeño y nuevo: úsalo con cuidado." },
    Emu { id: "eden-nx2", name: "Eden NX2", console: "Nintendo Switch 2", trust: "experimental",
        source: Source::Forgejo { host: "git.eden-emu.dev", repo: "Takoroka/eden-nx2", must: &["windows"] }, exe_hint: "eden",
        min: SW2_MIN, rec: SW2_REC,
        bios: "Necesita prod.keys y el firmware de TU Switch 2 (Herramientas → Instalar claves / firmware).",
        note: "Rama experimental de Eden." },
    // Herramienta (no sale en la lista): convierte el mando en ratón y teclado (START + BACK)
    Emu { id: "xtend", name: "xTend", console: "Herramienta", trust: "fiable",
        source: Source::Github { repo: "kulishnik22/xTend", must: &["xtend"] }, exe_hint: "xtend",
        min: req(1, 1.0, 0, 0.0, false), rec: req(1, 1.0, 0, 0.0, false),
        bios: "", note: "Licencia GPL-3.0 · github.com/kulishnik22/xTend" },
    // Herramienta (no sale en la lista): jugar a tu PS5 / PS4 por la red (Remote Play libre, muy poca latencia)
    Emu { id: "chiaki-ng", name: "chiaki-ng", console: "PS5 Remote Play", trust: "fiable",
        source: Source::Github { repo: "chiaki-ng/chiaki-ng", must: &["win_x64", "portable"] }, exe_hint: "chiaki",
        min: req(2, 4.0, 1, 1.0, false), rec: req(4, 8.0, 2, 2.0, false),
        bios: "", note: "Licencia AGPL-3.0 · github.com/chiaki-ng/chiaki-ng" },
];

/// Herramientas que se usan desde su propia pantalla, no desde la lista de emuladores
const HIDDEN: &[&str] = &["xtend", "chiaki-ng"];

/// PS5 Remote Play: instala chiaki-ng si falta y lo abre
#[tauri::command(async)]
pub fn open_remote_play() -> Result<(), String> {
    if read_installed("chiaki-ng").is_none() {
        install("chiaki-ng")?;
    }
    launch_emulator("chiaki-ng".into())
}

/// Instala xTend si falta y lo deja funcionando en segundo plano
pub fn ensure_xtend_running() -> Result<(), String> {
    if read_installed("xtend").is_none() {
        install("xtend")?;
    }
    // Sin ventana: si no, al abrir la consola parpadea una terminal
    use std::os::windows::process::CommandExt;
    let out = Command::new("tasklist").args(["/FI", "IMAGENAME eq xtend.exe", "/NH"]).creation_flags(0x0800_0000).output().map_err(|e| e.to_string())?;
    if String::from_utf8_lossy(&out.stdout).to_lowercase().contains("xtend.exe") {
        return Ok(());
    }
    launch_emulator("xtend".into())
}

// ---------------- ¿Irá bien? ----------------

fn meets(h: &Hardware, r: &Req) -> bool {
    h.threads >= r.threads && h.ram_gb + 0.5 >= r.ram_gb && h.gpu_tier >= r.gpu_tier && h.vram_gb + 0.25 >= r.vram_gb && (!r.avx2 || h.avx2)
}

fn verdict(h: &Hardware, e: &Emu) -> &'static str {
    if meets(h, &e.rec) { "bien" } else if meets(h, &e.min) { "justo" } else { "no" }
}

// ---------------- Descarga de la última versión ----------------

#[derive(Debug, Clone, Serialize)]
pub struct Release {
    pub version: String,
    pub file: String,
    pub url: String,
    /// Programa suelto que va aparte del .zip (AnyPS5 publica su "relinker.exe" así)
    pub extra_exe: Option<(String, String)>,
}

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("Consola/0.1 (emulator updater)")
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("cliente http")
}

fn pick_asset<'a>(names: impl Iterator<Item = &'a str>, must: &[&str]) -> Option<&'a str> {
    const BAD: [&str; 9] = ["debug", "symbols", "pdb", "arm64", "aarch64", "linux", "macos", "android", "source"];
    names
        .filter(|n| {
            let l = n.to_lowercase();
            (l.ends_with(".zip") || l.ends_with(".7z"))
                && must.iter().all(|m| l.contains(m))
                && !BAD.iter().any(|b| l.contains(b))
        })
        .next()
}

fn latest(e: &Emu) -> Result<Release, String> {
    let http = client();
    let get = |url: &str| -> Result<reqwest::blocking::Response, String> {
        let r = http.get(url).send().map_err(|err| trf("emu.no_connection", &[("url", url), ("err", &err.to_string())]))?;
        if r.status().is_success() { Ok(r) } else { Err(trf("emu.http_status", &[("url", url), ("status", &r.status().to_string())])) }
    };
    match e.source {
        Source::Github { repo, must } => {
            // Sin API (tiene límite de uso): /releases/latest redirige a /tag/<versión>
            let r = get(&format!("https://github.com/{repo}/releases/latest"))?;
            let tag = r.url().path().rsplit("/tag/").next().filter(|t| !t.contains('/')).ok_or(tr("emu.no_releases"))?.to_string();
            let html = get(&format!("https://github.com/{repo}/releases/expanded_assets/{tag}"))?.text().map_err(|e| e.to_string())?;
            let prefix = format!("/{repo}/releases/download/{tag}/");
            let files: Vec<String> = html
                .split("href=\"")
                .skip(1)
                .filter_map(|s| s.split('"').next())
                .filter_map(|h| h.strip_prefix(&prefix).map(String::from))
                .collect();
            let file = pick_asset(files.iter().map(String::as_str), must).ok_or(tr("emu.no_windows"))?.to_string();
            let extra_exe = files
                .iter()
                .find(|f| {
                    // Un programa suelto, nunca un instalador ni la versión para otros procesadores
                    let l = f.to_lowercase();
                    f.starts_with(e.exe_hint) && l.ends_with(".exe")
                        && !["install", "setup", "arm64", "aarch64"].iter().any(|b| l.contains(b))
                })
                .map(|f| (format!("https://github.com{prefix}{f}"), f.clone()));
            // Versiones "rolling" (siempre se llaman igual): añadir la fecha para detectar cambios
            let version = if ["latest", "preview", "continuous", "nightly"].contains(&tag.as_str()) {
                let date = html.split("datetime=\"").nth(1).and_then(|s| s.get(..10)).unwrap_or("");
                format!("{tag}-{date}")
            } else {
                tag
            };
            Ok(Release { url: format!("https://github.com{prefix}{file}"), version, file, extra_exe })
        }
        Source::Forgejo { host, repo, must } => {
            #[derive(Deserialize)]
            struct Asset { name: String, browser_download_url: String }
            #[derive(Deserialize)]
            struct Rel { tag_name: String, assets: Vec<Asset> }
            let rels: Vec<Rel> = get(&format!("https://{host}/api/v1/repos/{repo}/releases?limit=5"))?.json().map_err(|e| e.to_string())?;
            for rel in rels {
                if let Some(name) = pick_asset(rel.assets.iter().map(|a| a.name.as_str()), must) {
                    let a = rel.assets.iter().find(|a| a.name == name).unwrap();
                    return Ok(Release { version: rel.tag_name.clone(), file: a.name.clone(), url: a.browser_download_url.clone(), extra_exe: None });
                }
            }
            Err(tr("emu.no_windows").into())
        }
        Source::Dolphin => {
            // La web de Dolphin bloquea programas; las versiones estables están como etiquetas en GitHub
            let html = get("https://github.com/dolphin-emu/dolphin/tags")?.text().map_err(|e| e.to_string())?;
            let tag = html
                .split("href=\"/dolphin-emu/dolphin/releases/tag/")
                .skip(1)
                .filter_map(|s| s.split('"').next())
                .find(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit()))
                .ok_or(tr("emu.no_dolphin"))?
                .to_string();
            let file = format!("dolphin-{tag}-x64.7z");
            Ok(Release { url: format!("https://dl.dolphin-emu.org/releases/{tag}/{file}"), version: tag, file, extra_exe: None })
        }
    }
}

// ---------------- Instalación ----------------

fn root() -> PathBuf {
    std::env::var("CONSOLA_EMU_DIR").map(PathBuf::from).unwrap_or_else(|_| {
        crate::paths::data_dir().join("emuladores")
    })
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Installed { version: String, exe: String, previous: Option<Box<Installed>> }

fn read_installed(id: &str) -> Option<Installed> {
    let mut i: Installed = serde_json::from_str(&fs::read_to_string(root().join(id).join("instalado.json")).ok()?).ok()?;
    rebase(&mut i, &root());
    Some(i)
}

/// Hasta la 1.1.0 los emuladores vivían en %LOCALAPPDATA%\Consola\emuladores: si la ruta guardada
/// ya no existe pero el mismo archivo está bajo la carpeta actual, se usa la nueva
fn rebase(i: &mut Installed, root: &Path) {
    if !Path::new(&i.exe).exists() {
        let lower = i.exe.to_lowercase().replace('/', "\\");
        if let Some(pos) = lower.find("\\emuladores\\") {
            let candidate = root.join(&i.exe[pos + "\\emuladores\\".len()..]);
            if candidate.exists() {
                i.exe = candidate.display().to_string();
            }
        }
    }
    if let Some(prev) = i.previous.as_mut() {
        rebase(prev, root);
    }
}

fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    if archive.extension().is_some_and(|e| e.eq_ignore_ascii_case("7z")) {
        sevenz_rust2::decompress_file(archive, dest).map_err(|e| e.to_string())
    } else {
        let f = fs::File::open(archive).map_err(|e| e.to_string())?;
        zip::ZipArchive::new(f).and_then(|mut z| z.extract(dest)).map_err(|e| e.to_string())
    }
}

fn find_exe(dir: &Path, hint: &str) -> Option<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")) {
                found.push(p);
            }
        }
    }
    let skip = ["update", "crash", "unins", "vc_redist", "setup", "dxsetup"];
    let ok = |p: &PathBuf| {
        let n = p.file_name().unwrap().to_string_lossy().to_lowercase();
        !skip.iter().any(|s| n.contains(s))
    };
    // Primero el que contiene el nombre del emulador; si no, el más corto
    found.iter().filter(|p| ok(p)).find(|p| p.file_name().unwrap().to_string_lossy().to_lowercase().contains(hint)).cloned()
        .or_else(|| found.into_iter().filter(ok).min_by_key(|p| p.as_os_str().len()))
}

fn install_release(e: &Emu, rel: &Release) -> Result<Installed, String> {
    let dir = root().join(e.id);
    let safe_version: String = rel.version.chars().map(|c| if c.is_ascii_alphanumeric() || ".-_".contains(c) { c } else { '_' }).collect();
    let target = dir.join(&safe_version);
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let archive = dir.join(&rel.file);
    let mut resp = client().get(&rel.url).send().map_err(|err| err.to_string())?;
    if !resp.status().is_success() {
        return Err(trf("emu.download_failed", &[("status", &resp.status().to_string())]));
    }
    let mut bytes = Vec::new();
    resp.read_to_end(&mut bytes).map_err(|err| err.to_string())?;
    fs::write(&archive, &bytes).map_err(|err| err.to_string())?;
    let _ = fs::remove_dir_all(&target);
    let result = extract(&archive, &target);
    let _ = fs::remove_file(&archive);
    result?;
    if let Some((url, file)) = &rel.extra_exe {
        let data = client().get(url).send().and_then(|r| r.bytes()).map_err(|err| err.to_string())?;
        fs::write(target.join(file), &data).map_err(|err| err.to_string())?;
    }
    // Modo portable cuando el emulador lo admite: la configuración queda junto al programa
    let exe = find_exe(&target, e.exe_hint).ok_or(tr("emu.no_exe"))?;
    if matches!(e.id, "duckstation" | "pcsx2" | "melonds") {
        let _ = fs::write(exe.parent().unwrap().join("portable.txt"), "");
    }
    // Tus datos (BIOS, partidas, ajustes, juegos instalados…) se mudan a la versión nueva
    if let Some(prev) = read_installed(e.id).filter(|p| p.version != rel.version) {
        if let (Some(from), Some(to)) = (Path::new(&prev.exe).parent(), exe.parent()) {
            carry_user_data(from, to);
        }
    }
    let previous = read_installed(e.id).filter(|p| p.version != rel.version).map(|mut p| {
        // Solo se guarda una versión anterior; las más viejas se borran
        if let Some(old) = p.previous.take() {
            if let Some(old_dir) = Path::new(&old.exe).ancestors().find(|a| a.parent() == Some(dir.as_path())) {
                let _ = fs::remove_dir_all(old_dir);
            }
        }
        Box::new(p)
    });
    let inst = Installed { version: rel.version.clone(), exe: exe.to_string_lossy().into_owned(), previous };
    fs::write(dir.join("instalado.json"), serde_json::to_string_pretty(&inst).unwrap()).map_err(|err| err.to_string())?;
    Ok(inst)
}

/// Mueve a la carpeta de la versión nueva todo lo que el usuario creó junto al programa
/// (lo que la descarga nueva no trae): carpetas `bios`, `memcards`, `dev_hdd0`, `user`, ajustes…
/// Se mueve (no se copia): es instantáneo aunque haya muchos GB de juegos instalados.
fn carry_user_data(from: &Path, to: &Path) {
    let Ok(entries) = fs::read_dir(from) else { return };
    for entry in entries.flatten() {
        let target = to.join(entry.file_name());
        if !target.exists() {
            let _ = fs::rename(entry.path(), &target);
        }
    }
}

// ---------------- Carpetas de cada emulador ----------------

#[derive(Serialize, Clone)]
pub struct Folder {
    /// "juegos" | "bios" | "datos"
    key: &'static str,
    label: &'static str,
    hint: &'static str,
    path: String,
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".into()))
}

/// Dónde va cada cosa. Los emuladores que guardan todo junto al programa (portables) usan su
/// carpeta; los demás, la suya de Windows. Los juegos, siempre en Juegos\<consola>.
fn folders(e: &Emu, exe_dir: Option<&Path>) -> Vec<Folder> {
    let console: String = e.console.chars().filter(|c| !"<>:\"/\\|?*".contains(*c)).collect();
    let games = home().join("Juegos").join(&console);
    let appdata = PathBuf::from(std::env::var("APPDATA").unwrap_or_default());
    let f = |key, label, hint, path: PathBuf| Folder { key, label, hint, path: path.to_string_lossy().into_owned() };
    let mut out = vec![f("juegos", tr("emu.folder.games"), tr("emu.folder.games_hint"), games.clone())];
    let exe_dir = exe_dir.map(Path::to_path_buf);
    let bios = match e.id {
        "duckstation" => exe_dir.as_ref().map(|d| (d.join("bios"), "emu.folder.bios_duckstation")),
        "pcsx2" => exe_dir.as_ref().map(|d| (d.join("bios"), "emu.folder.bios_pcsx2")),
        "melonds" => exe_dir.as_ref().map(|d| (d.join("bios"), "emu.folder.bios_melonds")),
        "rpcs3" => Some((games.join("Firmware"), "emu.folder.bios_rpcs3")),
        "shadps4" => exe_dir.as_ref().map(|d| (d.join("user").join("sys_modules"), "emu.folder.bios_shadps4")),
        "eden" | "eden-nx2" => Some((appdata.join("eden").join("keys"), "emu.folder.bios_eden")),
        _ => None,
    };
    if let Some((path, hint)) = bios {
        out.push(f("bios", tr("emu.folder.bios"), tr(hint), path));
    }
    let data = match e.id {
        "dolphin" => Some(home().join("Documents").join("Dolphin Emulator")),
        "eden" | "eden-nx2" => Some(appdata.join("eden")),
        _ => exe_dir.clone(),
    };
    if let Some(path) = data {
        out.push(f("datos", tr("emu.folder.data"), tr("emu.folder.data_hint"), path));
    }
    out
}

/// Abre (creándola si hace falta) una de las carpetas del emulador
#[tauri::command(async)]
pub fn emu_folder(id: String, key: String) -> Result<String, String> {
    let e = emu(&id)?;
    let inst = read_installed(e.id);
    let exe_dir = inst.as_ref().and_then(|i| Path::new(&i.exe).parent().map(Path::to_path_buf));
    let f = folders(e, exe_dir.as_deref()).into_iter().find(|f| f.key == key).ok_or(tr("transfer.unknown_folder"))?;
    fs::create_dir_all(&f.path).map_err(|err| err.to_string())?;
    Ok(f.path)
}

/// Una carpeta de un emulador instalado a la que se pueden enviar archivos desde otro dispositivo
pub struct Destination {
    /// "<emulador>.<carpeta>", p. ej. "pcsx2.bios"
    pub id: String,
    pub emu: &'static str,
    pub label: &'static str,
    pub path: PathBuf,
}

/// Todas las carpetas (juegos, BIOS, partidas) de los emuladores instalados
pub fn destinations() -> Vec<Destination> {
    CATALOG
        .iter()
        .filter(|e| !HIDDEN.contains(&e.id))
        .filter_map(|e| read_installed(e.id).map(|i| (e, i)))
        .flat_map(|(e, inst)| {
            let exe_dir = Path::new(&inst.exe).parent().map(Path::to_path_buf);
            folders(e, exe_dir.as_deref()).into_iter().map(move |f| Destination {
                id: format!("{}.{}", e.id, f.key),
                emu: e.name,
                label: f.label,
                path: PathBuf::from(f.path),
            })
        })
        .collect()
}

fn emu(id: &str) -> Result<&'static Emu, String> {
    CATALOG.iter().find(|e| e.id == id).ok_or_else(|| tr("emu.unknown").into())
}

// ---------------- Comandos ----------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmuInfo {
    id: &'static str,
    name: &'static str,
    console: &'static str,
    trust: &'static str,
    verdict: &'static str,
    bios: &'static str,
    note: &'static str,
    installed: Option<String>,
    can_rollback: bool,
    folder: Option<String>,
    /// Icono real del programa (PNG), cuando está instalado
    icon: Option<String>,
    folders: Vec<Folder>,
}

#[tauri::command(async)]
pub fn list_emulators() -> (Hardware, Vec<EmuInfo>) {
    let h = get_hardware();
    let shown: Vec<&Emu> = CATALOG.iter().filter(|e| !HIDDEN.contains(&e.id)).collect();
    let installed: Vec<Option<Installed>> = shown.iter().map(|e| read_installed(e.id)).collect();
    // Iconos reales de los .exe instalados (Windows los da en un hilo propio)
    let icons = crate::apps::icons::get_many(installed.iter().map(|i| i.as_ref().map(|i| i.exe.clone())).collect());
    let list = shown
        .into_iter()
        .zip(installed)
        .zip(icons)
        .map(|((e, inst), icon)| {
            let exe_dir = inst.as_ref().and_then(|i| Path::new(&i.exe).parent().map(Path::to_path_buf));
            EmuInfo {
                // Textos del catálogo en el idioma de la consola (claves emu.<id>.bios/note/console)
                id: e.id, name: e.name, trust: e.trust, verdict: verdict(&h, e),
                console: lookup(&format!("emu.{}.console", e.id)).unwrap_or(e.console),
                bios: lookup(&format!("emu.{}.bios", e.id)).unwrap_or(e.bios),
                note: lookup(&format!("emu.{}.note", e.id)).unwrap_or(e.note),
                can_rollback: inst.as_ref().is_some_and(|i| i.previous.is_some()),
                folder: exe_dir.as_ref().map(|p| p.to_string_lossy().into_owned()),
                folders: folders(e, exe_dir.as_deref()),
                icon,
                installed: inst.map(|i| i.version),
            }
        })
        .collect();
    (h, list)
}

/// Instala o actualiza a la última versión oficial
pub fn install(id: &str) -> Result<String, String> {
    let e = emu(id)?;
    let rel = latest(e)?;
    if read_installed(e.id).is_some_and(|i| i.version == rel.version) {
        return Ok(rel.version);
    }
    install_release(e, &rel).map(|i| i.version)
}

#[tauri::command]
pub async fn install_emulator(id: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || install(&id)).await.map_err(|e| e.to_string())?
}

#[tauri::command(async)]
pub fn rollback_emulator(id: String) -> Result<String, String> {
    let e = emu(&id)?;
    let cur = read_installed(e.id).ok_or(tr("emu.not_installed"))?;
    let prev = *cur.previous.clone().ok_or(tr("emu.no_previous"))?;
    // Tus datos vuelven con la versión anterior
    if let (Some(from), Some(to)) = (Path::new(&cur.exe).parent(), Path::new(&prev.exe).parent()) {
        carry_user_data(from, to);
    }
    let back = Installed { version: prev.version.clone(), exe: prev.exe.clone(), previous: Some(Box::new(Installed { previous: None, ..cur })) };
    fs::write(root().join(e.id).join("instalado.json"), serde_json::to_string_pretty(&back).unwrap()).map_err(|err| err.to_string())?;
    Ok(back.version)
}

/// Pide a Windows que use la gráfica potente (NVIDIA/AMD) con este programa
fn prefer_dedicated_gpu(exe: &str) {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    if let Ok((k, _)) = hkcu.create_subkey("Software\\Microsoft\\DirectX\\UserGpuPreferences") {
        let _ = k.set_value(exe, &"GpuPreference=2;");
    }
}

#[tauri::command(async)]
pub fn launch_emulator(id: String) -> Result<(), String> {
    let e = emu(&id)?;
    let inst = read_installed(e.id).ok_or(tr("emu.not_installed"))?;
    prefer_dedicated_gpu(&inst.exe);
    let exe = Path::new(&inst.exe);
    Command::new(exe).current_dir(exe.parent().unwrap()).spawn().map(|_| ()).map_err(|err| err.to_string())
}

/// Al arrancar: actualiza en segundo plano los emuladores ya instalados
pub fn update_all_installed() {
    for e in CATALOG {
        let Some(inst) = read_installed(e.id) else { continue };
        match latest(e) {
            Ok(rel) if rel.version != inst.version => match install_release(e, &rel) {
                Ok(_) => log::info!("{} actualizado a {}", e.name, rel.version),
                Err(err) => log::warn!("{}: no se pudo actualizar: {err}", e.name),
            },
            Ok(_) => {}
            Err(err) => log::warn!("{}: {err}", e.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textos_del_catalogo_traducidos() {
        for e in CATALOG.iter().filter(|e| !HIDDEN.contains(&e.id)) {
            if !e.bios.is_empty() {
                assert_eq!(lookup(&format!("emu.{}.bios", e.id)), Some(e.bios), "{}: falta o no coincide emu.{}.bios", e.id, e.id);
            }
            if !e.note.is_empty() {
                assert_eq!(lookup(&format!("emu.{}.note", e.id)), Some(e.note), "{}: falta o no coincide emu.{}.note", e.id, e.id);
            }
        }
        assert_eq!(lookup("emu.dolphin.console"), Some("Wii y GameCube"));
    }

    #[test]
    fn elige_el_archivo_de_windows() {
        let files = ["pcsx2-v2.4-linux-appimage-x64-Qt.AppImage", "pcsx2-v2.4-windows-x64-Qt-symbols.7z", "pcsx2-v2.4-windows-arm64-Qt.7z", "pcsx2-v2.4-windows-x64-Qt.7z"];
        assert_eq!(pick_asset(files.into_iter(), &["windows", "x64"]), Some("pcsx2-v2.4-windows-x64-Qt.7z"));
    }

    #[test]
    fn veredicto_para_este_portatil() {
        let h = Hardware { cpu: "i7-10710U".into(), threads: 12, avx2: true, ram_gb: 15.8, gpu: "GTX 1650 Max-Q".into(), vram_gb: 4.0, gpu_tier: 2, vulkan: true };
        let v = |id| verdict(&h, emu(id).unwrap());
        assert_eq!(v("duckstation"), "bien");
        assert_eq!(v("pcsx2"), "bien");
        assert_eq!(v("rpcs3"), "justo");
        assert_eq!(v("eden"), "justo");
        assert_eq!(v("kytyps5"), "no");
        assert_eq!(v("novas"), "no");
    }

    /// Consulta la web oficial de cada emulador (red real, no descarga nada)
    #[test]
    #[ignore]
    fn encuentra_version_windows_de_cada_emulador() {
        for e in CATALOG {
            match latest(e) {
                Ok(r) => println!("OK   {:<12} {:<22} {}", e.id, r.version, r.file),
                Err(err) => println!("FALLA {:<12} {err}", e.id),
            }
        }
    }

    /// Instalación real de melonDS (pequeño) en una carpeta temporal
    #[test]
    fn tus_datos_pasan_a_la_version_nueva() {
        let base = std::env::temp_dir().join(format!("consola-carry-{}", std::process::id()));
        let (old, new) = (base.join("v1"), base.join("v2"));
        for d in [&old, &new] { fs::create_dir_all(d).unwrap(); }
        fs::write(old.join("duckstation.exe"), b"viejo").unwrap();
        fs::write(new.join("duckstation.exe"), b"nuevo").unwrap();
        fs::create_dir_all(old.join("bios")).unwrap();
        fs::write(old.join("bios").join("scph5501.bin"), b"bios").unwrap();
        fs::write(old.join("settings.ini"), b"ajustes").unwrap();
        carry_user_data(&old, &new);
        assert_eq!(fs::read(new.join("bios").join("scph5501.bin")).unwrap(), b"bios");
        assert_eq!(fs::read(new.join("settings.ini")).unwrap(), b"ajustes");
        assert_eq!(fs::read(new.join("duckstation.exe")).unwrap(), b"nuevo", "el programa nuevo no se pisa");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn carpetas_de_cada_emulador() {
        let exe_dir = Path::new(r"C:\emus\duckstation\v1");
        let f = folders(emu("duckstation").unwrap(), Some(exe_dir));
        let keys: Vec<_> = f.iter().map(|f| f.key).collect();
        assert_eq!(keys, ["juegos", "bios", "datos"]);
        assert!(f[0].path.ends_with(r"Juegos\PlayStation"));
        assert_eq!(f[1].path, r"C:\emus\duckstation\v1\bios");
        // Sin instalar: solo la de juegos (y la de claves de Eden, que no depende del programa)
        assert_eq!(folders(emu("pcsx2").unwrap(), None).len(), 1);
        assert!(folders(emu("eden").unwrap(), None).iter().any(|f| f.path.ends_with(r"eden\keys")));
        assert!(folders(emu("dolphin").unwrap(), None).iter().any(|f| f.path.ends_with("Dolphin Emulator")));
    }

    #[test]
    #[ignore]
    fn instala_melonds_de_verdad() {
        let dir = std::env::temp_dir().join(format!("consola-emu-{}", std::process::id()));
        unsafe { std::env::set_var("CONSOLA_EMU_DIR", &dir) };
        for id in ["melonds", "pcsx2", "xtend", "chiaki-ng"] {
            // melonDS viene en .zip y PCSX2 en .7z
            let e = emu(id).unwrap();
            let rel = latest(e).unwrap();
            let inst = install_release(e, &rel).unwrap();
            println!("instalado {} {} en {}", id, inst.version, inst.exe);
            assert!(Path::new(&inst.exe).exists());
            assert_eq!(read_installed(id).unwrap().version, rel.version);
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
