//! Música: lo que suena en el PC (Spotify o cualquier reproductor) y sus controles.
//! Usa los controles multimedia de Windows (los mismos de las teclas de música del teclado),
//! así funciona con Spotify, Apple Music, YouTube en Edge/Chrome, VLC…
//! El volumen se cambia solo para la app que suena (mezclador de volumen de Windows).

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};

const NO_WINDOW: u32 = 0x0800_0000;
const SPOTIFY_STORE: &str = "SpotifyAB.SpotifyMusic_zpdnekdrzrea0";

/// Lo que suena ahora mismo
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    /// Nombre corto de la app: "Spotify", "Chrome", "Apple Music"…
    pub app: String,
    /// Identificador de Windows de la app (AUMID)
    pub app_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    /// Segundos (si la app lo informa)
    pub position: Option<f64>,
    pub duration: Option<f64>,
    pub can_next: bool,
    pub can_prev: bool,
    /// Ruta de la carátula guardada en %LOCALAPPDATA%\Consola\musica
    pub cover: Option<String>,
}

/// Volumen cambiado: el de la app que suena ("app") o el general del PC ("system")
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct AppVolume {
    pub level: u8,
    pub scope: &'static str,
    /// App cuyo volumen se ha leído o cambiado (si es "app")
    pub app: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action { Play, Pause, Toggle, Next, Previous }

/// Acepta inglés y español: "play", "pausa", "siguiente", "anterior"…
pub fn parse_action(action: &str) -> Option<Action> {
    Some(match action.trim().to_lowercase().as_str() {
        "play" | "reproducir" | "resume" | "seguir" => Action::Play,
        "pause" | "pausa" | "pausar" | "stop" | "parar" => Action::Pause,
        "toggle" | "playpause" | "play_pause" | "alternar" => Action::Toggle,
        "next" | "siguiente" | "skip" => Action::Next,
        "previous" | "prev" | "anterior" | "back" => Action::Previous,
        _ => return None,
    })
}

/// Nombre corto y legible a partir del identificador de la app (AUMID)
pub fn friendly_app_name(aumid: &str) -> String {
    let id = aumid.trim();
    let low = id.to_lowercase();
    let known: &[(&str, &str)] = &[
        ("spotify", "Spotify"),
        ("applemusic", "Apple Music"),
        ("itunes", "iTunes"),
        ("zunemusic", crate::i18n::tr("media.player")),
        ("msedge", "Edge"),
        ("microsoftedge", "Edge"),
        ("chrome", "Chrome"),
        ("308046b0af4a39cb", "Firefox"), // así se identifica Firefox
        ("firefox", "Firefox"),
        ("opera", "Opera"),
        ("brave", "Brave"),
        ("vlc", "VLC"),
        ("tidal", "TIDAL"),
        ("deezer", "Deezer"),
        ("amazonmusic", "Amazon Music"),
        ("youtube", "YouTube"),
    ];
    if let Some((_, name)) = known.iter().find(|(k, _)| low.contains(k)) {
        return (*name).to_string();
    }
    // "Paquete.Nombre_hash!App" → la parte tras "!" (salvo "App") o el último trozo del paquete
    let (pkg, entry) = id.split_once('!').unwrap_or((id, ""));
    let raw = if !entry.is_empty() && !entry.eq_ignore_ascii_case("app") {
        entry.rsplit('.').next().unwrap_or(entry)
    } else {
        let pkg = pkg.split('_').next().unwrap_or(pkg);
        let pkg = pkg.rsplit(['\\', '/']).next().unwrap_or(pkg);
        let pkg = pkg.strip_suffix(".exe").or_else(|| pkg.strip_suffix(".EXE")).unwrap_or(pkg);
        pkg.rsplit('.').next().unwrap_or(pkg)
    };
    let mut c = raw.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => crate::i18n::tr("media.player_short").into(),
    }
}

/// Programa (.exe) que hace sonar esa app, para encontrar su volumen
pub fn exe_for(aumid: &str) -> String {
    // El nombre del reproductor de Windows cambia con el idioma: se mira el identificador
    if aumid.to_lowercase().contains("zunemusic") {
        return "microsoft.media.player.exe".into();
    }
    match friendly_app_name(aumid).as_str() {
        "Spotify" => "spotify.exe".into(),
        "Edge" => "msedge.exe".into(),
        "Chrome" => "chrome.exe".into(),
        "Firefox" => "firefox.exe".into(),
        "Apple Music" => "applemusic.exe".into(),
        other => format!("{}.exe", other.to_lowercase().replace(' ', "")),
    }
}

// ---------- Controles multimedia de Windows ----------

fn com_init() {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    // Si el hilo ya tiene COM iniciado en otro modo, se puede usar igual
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
}

fn err(e: windows::core::Error) -> String {
    e.message().to_string()
}

/// Sesión elegida: Spotify si está; si no, la que Windows da como actual; si no, la primera
fn session() -> Result<Option<Session>, String> {
    com_init();
    let mgr = Manager::RequestAsync().and_then(|op| op.join()).map_err(err)?;
    let sessions: Vec<Session> = mgr.GetSessions().map(|v| v.into_iter().collect()).unwrap_or_default();
    let spotify = sessions
        .iter()
        .find(|s| s.SourceAppUserModelId().map(|id| friendly_app_name(&id.to_string()) == "Spotify").unwrap_or(false));
    if let Some(s) = spotify {
        return Ok(Some(s.clone()));
    }
    if let Ok(s) = mgr.GetCurrentSession() {
        return Ok(Some(s));
    }
    Ok(sessions.into_iter().next())
}

fn covers_dir() -> PathBuf {
    crate::paths::data_dir().join("musica")
}

fn hash(text: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Guarda la carátula una vez por canción (si ya está, no se vuelve a leer)
fn save_cover(props: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties, key: &str) -> Option<String> {
    use windows::Storage::Streams::{Buffer, DataReader, InputStreamOptions};
    let dir = covers_dir();
    let name = format!("cover-{}", hash(key));
    for ext in ["jpg", "png"] {
        let p = dir.join(format!("{name}.{ext}"));
        if p.exists() {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    let stream = props.Thumbnail().ok()?.OpenReadAsync().ok()?.join().ok()?;
    let size = stream.Size().ok()?.min(8 * 1024 * 1024) as u32;
    if size == 0 {
        return None;
    }
    let ext = if stream.ContentType().map(|t| t.to_string().contains("png")).unwrap_or(false) { "png" } else { "jpg" };
    let buffer = Buffer::Create(size).ok()?;
    let read = stream.ReadAsync(&buffer, size, InputStreamOptions::None).ok()?.join().ok()?;
    let len = read.Length().ok()? as usize;
    let mut bytes = vec![0u8; len];
    DataReader::FromBuffer(&read).ok()?.ReadBytes(&mut bytes).ok()?;
    // Solo se guarda la última: la carpeta no crece
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            if e.file_name().to_string_lossy().starts_with("cover-") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let path = dir.join(format!("{name}.{ext}"));
    std::fs::write(&path, &bytes).ok()?;
    Some(path.to_string_lossy().into_owned())
}

/// 100 ns desde 1601 (formato de fecha de Windows) de este momento
fn now_ticks() -> i64 {
    let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    (unix.as_nanos() / 100) as i64 + 116_444_736_000_000_000
}

/// Posición real: la app la informa de vez en cuando; si suena, se suma el tiempo pasado
pub fn live_position(position: f64, duration: Option<f64>, elapsed: f64, playing: bool) -> f64 {
    let p = if playing { position + elapsed.max(0.0) } else { position };
    match duration {
        Some(d) if d > 0.0 => p.clamp(0.0, d),
        _ => p.max(0.0),
    }
}

fn read(s: &Session) -> Result<NowPlaying, String> {
    let app_id = s.SourceAppUserModelId().map(|h| h.to_string()).unwrap_or_default();
    let props = s.TryGetMediaPropertiesAsync().and_then(|op| op.join()).map_err(err)?;
    let title = props.Title().map(|h| h.to_string()).unwrap_or_default();
    let artist = props.Artist().map(|h| h.to_string()).unwrap_or_default();
    let album = props.AlbumTitle().map(|h| h.to_string()).unwrap_or_default();
    let info = s.GetPlaybackInfo().map_err(err)?;
    let playing = info.PlaybackStatus().map(|st| st == Status::Playing).unwrap_or(false);
    let controls = info.Controls().ok();
    let can_next = controls.as_ref().and_then(|c| c.IsNextEnabled().ok()).unwrap_or(false);
    let can_prev = controls.as_ref().and_then(|c| c.IsPreviousEnabled().ok()).unwrap_or(false);

    let (mut position, mut duration) = (None, None);
    if let Ok(tl) = s.GetTimelineProperties() {
        let secs = |ticks: i64| ticks as f64 / 10_000_000.0;
        let end = tl.EndTime().map(|t| secs(t.Duration)).unwrap_or(0.0);
        let start = tl.StartTime().map(|t| secs(t.Duration)).unwrap_or(0.0);
        if end > start {
            let d = end - start;
            let pos = tl.Position().map(|t| secs(t.Duration)).unwrap_or(0.0) - start;
            let elapsed = tl.LastUpdatedTime().map(|t| secs(now_ticks() - t.UniversalTime)).unwrap_or(0.0);
            duration = Some(d);
            position = Some(live_position(pos, Some(d), elapsed, playing));
        }
    }
    let cover = save_cover(&props, &format!("{app_id}|{title}|{artist}|{album}"));
    Ok(NowPlaying { app: friendly_app_name(&app_id), app_id, title, artist, album, playing, position, duration, can_next, can_prev, cover })
}

fn current() -> Result<Option<NowPlaying>, String> {
    match session()? {
        Some(s) => {
            let np = read(&s)?;
            // Una sesión sin título no es "algo sonando" (p. ej. una pestaña muda)
            Ok((!np.title.is_empty()).then_some(np))
        }
        None => Ok(None),
    }
}

/// Lo que suena ahora (para Claude). Error si no suena nada.
pub fn now_playing() -> Result<NowPlaying, String> {
    current()?.ok_or_else(|| crate::i18n::tr("media.nothing_playing").to_string())
}

/// play, pause, toggle, next, previous (también en español)
pub fn control(action: &str) -> Result<(), String> {
    let a = parse_action(action).ok_or(crate::i18n::tr("media.bad_action"))?;
    let s = session()?.ok_or(crate::i18n::tr("media.no_app"))?;
    let op = match a {
        Action::Play => s.TryPlayAsync(),
        Action::Pause => s.TryPauseAsync(),
        Action::Toggle => s.TryTogglePlayPauseAsync(),
        Action::Next => s.TrySkipNextAsync(),
        Action::Previous => s.TrySkipPreviousAsync(),
    };
    let ok = op.and_then(|o| o.join()).map_err(err)?;
    if ok { Ok(()) } else { Err(crate::i18n::tr("media.refused").into()) }
}

// ---------- Volumen de la app (mezclador de Windows) ----------

mod mixer {
    use windows::core::{Interface, PWSTR};
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume,
        MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};

    /// Nombre del programa (en minúsculas) de un proceso
    fn exe_name(pid: u32) -> Option<String> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = [0u16; 520];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
            let _ = CloseHandle(h);
            if !ok {
                return None;
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            path.rsplit('\\').next().map(|s| s.to_lowercase())
        }
    }

    /// Volúmenes de todas las sesiones de sonido de ese programa
    pub fn volumes(exe: &str) -> windows::core::Result<Vec<ISimpleAudioVolume>> {
        super::com_init();
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let manager = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)?;
            let list = manager.GetSessionEnumerator()?;
            let mut out = Vec::new();
            for i in 0..list.GetCount()? {
                let Ok(ctl) = list.GetSession(i) else { continue };
                let Ok(ctl2) = ctl.cast::<IAudioSessionControl2>() else { continue };
                let Ok(pid) = ctl2.GetProcessId() else { continue };
                if pid != 0 && exe_name(pid).as_deref() == Some(exe) {
                    if let Ok(v) = ctl.cast::<ISimpleAudioVolume>() {
                        out.push(v);
                    }
                }
            }
            Ok(out)
        }
    }
}

/// Programa y nombre de la app que suena (Spotify si no suena nada)
fn target() -> (String, String) {
    match session().ok().flatten().and_then(|s| s.SourceAppUserModelId().ok()) {
        Some(id) => {
            let id = id.to_string();
            (exe_for(&id), friendly_app_name(&id))
        }
        None => ("spotify.exe".into(), "Spotify".into()),
    }
}

/// Volumen de la app que suena; si no tiene sonido propio, el general del PC
pub fn app_volume_get() -> Result<AppVolume, String> {
    let (exe, app) = target();
    if let Some(v) = mixer::volumes(&exe).ok().and_then(|v| v.into_iter().next()) {
        if let Ok(level) = unsafe { v.GetMasterVolume() } {
            return Ok(AppVolume { level: (level * 100.0).round() as u8, scope: "app", app: Some(app) });
        }
    }
    let (level, _) = crate::settings::audio::get().map_err(err)?;
    Ok(AppVolume { level, scope: "system", app: None })
}

/// Cambia el volumen de la app que suena (0-100); si no se encuentra, el general del PC
pub fn app_volume_set(level: u8) -> Result<AppVolume, String> {
    let level = level.min(100);
    let (exe, app) = target();
    let vols = mixer::volumes(&exe).unwrap_or_default();
    let mut changed = false;
    for v in &vols {
        changed |= unsafe { v.SetMasterVolume(f32::from(level) / 100.0, std::ptr::null()) }.is_ok();
    }
    if changed {
        return Ok(AppVolume { level, scope: "app", app: Some(app) });
    }
    crate::settings::audio::set_level(level).map_err(err)?;
    Ok(AppVolume { level, scope: "system", app: None })
}

/// Abre la app de escritorio de Spotify (normal o de Microsoft Store).
/// Error si no está instalada: entonces se usa el reproductor web.
pub fn open_spotify() -> Result<(), String> {
    let env = |k: &str| std::env::var(k).map(PathBuf::from).ok();
    if let Some(exe) = env("APPDATA").map(|p| p.join("Spotify").join("Spotify.exe")).filter(|p| p.exists()) {
        return Command::new(exe).creation_flags(NO_WINDOW).spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    let store = env("LOCALAPPDATA").map(|p| p.join("Packages").join(SPOTIFY_STORE)).is_some_and(|p| p.exists());
    if store {
        return Command::new("explorer.exe")
            .arg(format!("shell:AppsFolder\\{SPOTIFY_STORE}!Spotify"))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string());
    }
    Err(crate::i18n::tr("media.no_spotify").into())
}

// ---------- Comandos para la pantalla ----------

#[tauri::command(async)]
pub fn media_now_playing() -> Result<Option<NowPlaying>, String> {
    current()
}

#[tauri::command(async)]
pub fn media_control(action: String) -> Result<(), String> {
    control(&action)
}

#[tauri::command(async)]
pub fn media_volume_get() -> Result<AppVolume, String> {
    app_volume_get()
}

#[tauri::command(async)]
pub fn media_volume_set(level: u8) -> Result<AppVolume, String> {
    app_volume_set(level)
}

#[tauri::command(async)]
pub fn media_open_spotify() -> Result<(), String> {
    open_spotify()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombres_de_apps() {
        assert_eq!(friendly_app_name("Spotify.exe"), "Spotify");
        assert_eq!(friendly_app_name("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"), "Spotify");
        assert_eq!(friendly_app_name("Chrome"), "Chrome");
        assert_eq!(friendly_app_name("MSEdge"), "Edge");
        assert_eq!(friendly_app_name("308046B0AF4A39CB"), "Firefox");
        assert_eq!(friendly_app_name("AppleInc.AppleMusicWin_nzyj5cx40ttqa!App"), "Apple Music");
        assert_eq!(friendly_app_name("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"), "Reproductor multimedia");
        assert_eq!(friendly_app_name("foobar2000.exe"), "Foobar2000");
        assert_eq!(friendly_app_name("Contoso.Player_abc123!App"), "Player");
        assert_eq!(friendly_app_name(""), "Reproductor");
    }

    #[test]
    fn programas_de_cada_app() {
        assert_eq!(exe_for("Spotify.exe"), "spotify.exe");
        assert_eq!(exe_for("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"), "spotify.exe");
        assert_eq!(exe_for("Chrome"), "chrome.exe");
        assert_eq!(exe_for("MSEdge"), "msedge.exe");
        assert_eq!(exe_for("vlc.exe"), "vlc.exe");
    }

    #[test]
    fn acciones() {
        assert_eq!(parse_action("play"), Some(Action::Play));
        assert_eq!(parse_action(" Pausa "), Some(Action::Pause));
        assert_eq!(parse_action("toggle"), Some(Action::Toggle));
        assert_eq!(parse_action("siguiente"), Some(Action::Next));
        assert_eq!(parse_action("prev"), Some(Action::Previous));
        assert_eq!(parse_action("previous"), Some(Action::Previous));
        assert_eq!(parse_action("borrar"), None);
    }

    #[test]
    fn posicion_en_vivo() {
        assert_eq!(live_position(10.0, Some(200.0), 5.0, true), 15.0);
        assert_eq!(live_position(10.0, Some(200.0), 5.0, false), 10.0);
        assert_eq!(live_position(198.0, Some(200.0), 5.0, true), 200.0);
        assert_eq!(live_position(10.0, None, -3.0, true), 10.0);
    }

    /// Lectura real del PC (solo lee). Ejecutar a mano: cargo test --lib media -- --ignored --nocapture
    #[test]
    #[ignore]
    fn lectura_real() {
        println!("suena: {:?}", current());
        println!("volumen: {:?}", app_volume_get());
    }
}
