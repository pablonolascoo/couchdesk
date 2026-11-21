//! Mando móvil: servidor web en la red local. El móvil abre la página (por QR)
//! y manda ratón, teclado y botones por WebSocket.
//! Varios móviles a la vez: cada uno tiene su cursor; el último que se mueve mueve el puntero real.

use crate::input;
use crate::receive::{is_lan, lan_ip, random_password};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, put};
use axum::Router;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Emitter;

pub const PORT: u16 = 7777;
const PAGE: &str = include_str!("../remote/index.html");
const COLORS: [&str; 6] = ["#1f6c9f", "#9f2f2d", "#346538", "#956400", "#6b4fa0", "#2f3437"];

#[derive(Clone, Serialize)]
pub struct Cursor { id: u32, name: String, color: String, x: f64, y: f64 }

pub struct Hub {
    token: String,
    clients: Mutex<HashMap<u32, Cursor>>,
    /// Órdenes de la consola a todos los móviles (modo mando/touchpad, teclado)
    tx: tokio::sync::broadcast::Sender<String>,
    /// Último modo pedido: un móvil que se conecta tarde lo recibe al entrar
    mode: Mutex<(String, bool)>,
    next_id: AtomicU32,
    last_emit: Mutex<Instant>,
    app: Option<tauri::AppHandle>,
}

impl Hub {
    pub fn new(app: Option<tauri::AppHandle>) -> Arc<Self> {
        Arc::new(Hub {
            token: saved_token(),
            clients: Mutex::new(HashMap::new()),
            tx: tokio::sync::broadcast::channel(32).0,
            mode: Mutex::new(("gp".into(), false)),
            next_id: AtomicU32::new(1),
            last_emit: Mutex::new(Instant::now()),
            app,
        })
    }

    fn mode_msg(&self) -> String {
        let (tab, kbd) = self.mode.lock().unwrap().clone();
        serde_json::json!({ "t": "mode", "tab": tab, "kbd": kbd }).to_string()
    }

    /// Cambia lo que enseñan los móviles: "gp" (mando) o "mouse" (touchpad y teclado), y el teclado
    pub fn set_mode(&self, tab: Option<&str>, kbd: Option<bool>) {
        {
            let mut m = self.mode.lock().unwrap();
            if let Some(t) = tab { m.0 = t.into(); }
            if let Some(k) = kbd { m.1 = k; }
        }
        let _ = self.tx.send(self.mode_msg());
    }

    pub fn url(&self) -> Option<String> {
        lan_ip().map(|ip| format!("http://{ip}:{PORT}/?t={}", self.token))
    }

    /// Página para enviar archivos desde el navegador de otro dispositivo
    pub fn upload_url(&self) -> Option<String> {
        lan_ip().map(|ip| format!("http://{ip}:{PORT}/subir?t={}", self.token))
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// Solo la red de casa y con el código del QR
    pub fn allowed(&self, addr: SocketAddr, t: Option<&str>) -> bool {
        is_lan(addr.ip()) && t == Some(self.token.as_str())
    }

    fn emit(&self, force: bool) {
        // Como mucho ~60 avisos por segundo a la pantalla
        let mut last = self.last_emit.lock().unwrap();
        if !force && last.elapsed() < Duration::from_millis(16) {
            return;
        }
        *last = Instant::now();
        if let Some(app) = &self.app {
            let list: Vec<Cursor> = self.clients.lock().unwrap().values().cloned().collect();
            let _ = app.emit("remote-cursors", list);
        }
    }
}

/// Código del QR guardado en disco: así el acceso guardado en el móvil sigue valiendo tras reiniciar
fn saved_token() -> String {
    let Ok(base) = std::env::var("LOCALAPPDATA") else { return random_password() };
    let file = std::path::Path::new(&base).join("Consola").join("mando.txt");
    if let Ok(t) = std::fs::read_to_string(&file) {
        let t = t.trim().to_string();
        if t.len() >= 10 && t.chars().all(|c| c.is_ascii_alphanumeric()) {
            return t;
        }
    }
    let t = random_password();
    let _ = std::fs::create_dir_all(file.parent().unwrap());
    let _ = std::fs::write(&file, &t);
    t
}

#[derive(Deserialize)]
struct TokenQuery { t: Option<String> }

#[derive(Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
enum Msg {
    Hello { name: String },
    Click { right: Option<bool> },
    Scroll { dx: Option<i32>, dy: i32 },
    Text { s: String },
    Key { k: String },
}

async fn page(State(hub): State<Arc<Hub>>, ConnectInfo(addr): ConnectInfo<SocketAddr>, Query(q): Query<TokenQuery>) -> Response {
    if !is_lan(addr.ip()) || q.t.as_deref() != Some(hub.token.as_str()) {
        return (StatusCode::FORBIDDEN, crate::i18n::tr("remote.scan_qr")).into_response();
    }
    (
        [(header::CACHE_CONTROL, "no-store"), (header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(PAGE.replace("__TOKEN__", &hub.token)),
    )
        .into_response()
}

/// Para "Añadir a pantalla de inicio": abre a pantalla completa, en horizontal y con el código ya puesto
async fn manifest(State(hub): State<Arc<Hub>>, ConnectInfo(addr): ConnectInfo<SocketAddr>, Query(q): Query<TokenQuery>) -> Response {
    if !is_lan(addr.ip()) || q.t.as_deref() != Some(hub.token.as_str()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let json_str = |k: &'static str| serde_json::to_string(crate::i18n::tr(k)).unwrap_or_default();
    let body = format!(
        r##"{{"name":{},"short_name":{},"display":"fullscreen","orientation":"landscape","background_color":"#fbf8f2","theme_color":"#fbf8f2","start_url":"/?t={}","icons":[{{"src":"/icon.svg?t={}","sizes":"any","type":"image/svg+xml"}}]}}"##,
        json_str("remote.app_name"), json_str("remote.app_short"), hub.token, hub.token
    );
    ([(header::CONTENT_TYPE, "application/manifest+json")], body).into_response()
}

const ICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><rect width="512" height="512" rx="112" fill="#fbf8f2"/><circle cx="256" cy="256" r="150" fill="none" stroke="#1c1712" stroke-width="18"/><circle cx="256" cy="256" r="58" fill="#1c1712"/></svg>"##;

async fn icon() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/svg+xml"), (header::CACHE_CONTROL, "max-age=86400")], ICON)
}

async fn ws(
    State(hub): State<Arc<Hub>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(q): Query<TokenQuery>,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !is_lan(addr.ip()) || q.t.as_deref() != Some(hub.token.as_str()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    upgrade.on_upgrade(move |socket| session(hub, socket))
}

async fn session(hub: Arc<Hub>, mut socket: WebSocket) {
    let id = hub.next_id.fetch_add(1, Ordering::Relaxed);
    let (w, h) = input::screen_size();
    {
        let mut clients = hub.clients.lock().unwrap();
        clients.insert(id, Cursor {
            id,
            name: crate::i18n::trf("remote.phone", &[("n", &id.to_string())]),
            color: COLORS[(id as usize - 1) % COLORS.len()].into(),
            x: f64::from(w) / 2.0,
            y: f64::from(h) / 2.0,
        });
    }
    hub.emit(true);
    // Al entrar, el móvil se pone en el modo que toca ahora mismo
    let _ = socket.send(Message::Text(hub.mode_msg().into())).await;
    let mut orders = hub.tx.subscribe();

    loop {
        let msg = tokio::select! {
            m = socket.next() => match m { Some(Ok(m)) => m, _ => break },
            o = orders.recv() => {
                if let Ok(o) = o {
                    if socket.send(Message::Text(o.into())).await.is_err() { break }
                }
                continue;
            }
        };
        match msg {
            // Movimiento: 4 bytes = dx, dy como i16 (formato binario, lo más ligero)
            Message::Binary(b) if b.len() == 4 => {
                let dx = f64::from(i16::from_le_bytes([b[0], b[1]]));
                let dy = f64::from(i16::from_le_bytes([b[2], b[3]]));
                let pos = {
                    let mut clients = hub.clients.lock().unwrap();
                    clients.get_mut(&id).map(|c| {
                        c.x = (c.x + dx).clamp(0.0, f64::from(w - 1));
                        c.y = (c.y + dy).clamp(0.0, f64::from(h - 1));
                        (c.x as i32, c.y as i32)
                    })
                };
                if let Some((x, y)) = pos {
                    input::move_to(x, y); // el último que se mueve lleva el puntero real
                }
                hub.emit(false);
            }
            Message::Text(t) => {
                let Ok(m) = serde_json::from_str::<Msg>(&t) else { continue };
                match m {
                    Msg::Hello { name } => {
                        let name: String = name.trim().chars().take(20).collect();
                        if !name.is_empty() {
                            if let Some(c) = hub.clients.lock().unwrap().get_mut(&id) {
                                c.name = name;
                            }
                        }
                        hub.emit(true);
                    }
                    Msg::Click { right } => {
                        // Clic donde está el cursor de ESTE móvil
                        let pos = hub.clients.lock().unwrap().get(&id).map(|c| (c.x as i32, c.y as i32));
                        if let Some((x, y)) = pos {
                            input::move_to(x, y);
                        }
                        input::click(right.unwrap_or(false));
                    }
                    Msg::Scroll { dx, dy } => input::scroll(dx.unwrap_or(0).clamp(-1200, 1200), dy.clamp(-1200, 1200)),
                    Msg::Text { s } => input::text(&s.chars().take(500).collect::<String>()),
                    // Inicio del móvil: la misma salida segura que el botón Xbox (cierra webs y nube)
                    Msg::Key { k } if k == "home" => match &hub.app {
                        Some(app) => crate::browser::go_home(app),
                        None => { input::key("home"); }
                    },
                    Msg::Key { k } => {
                        input::key(&k);
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    hub.clients.lock().unwrap().remove(&id);
    hub.emit(true);
}

pub fn router(hub: Arc<Hub>) -> Router {
    Router::new()
        .route("/", get(page))
        .route("/manifest.json", get(manifest))
        .route("/icon.svg", get(icon))
        .route("/ws", get(ws))
        .route("/subir", get(crate::transfer::page))
        .route("/api/destinos", get(crate::transfer::list))
        // Archivos de varios GB: sin límite de tamaño, se escriben a disco mientras llegan
        .route("/api/subir", put(crate::transfer::upload).layer(DefaultBodyLimit::disable()))
        .with_state(hub)
}

pub async fn serve(hub: Arc<Hub>, port: u16) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    axum::serve(listener, router(hub).into_make_service_with_connect_info::<SocketAddr>()).await
}

/// Cambia el modo de los móviles conectados desde cualquier parte de la consola
pub fn set_mode(app: &tauri::AppHandle, tab: Option<&str>, kbd: Option<bool>) {
    use tauri::Manager;
    if let Some(hub) = app.try_state::<Arc<Hub>>() {
        hub.set_mode(tab, kbd);
    }
}

/// Para la propia interfaz: al escribir en un campo, los móviles abren el teclado
#[tauri::command(async)]
pub fn remote_mode(hub: tauri::State<Arc<Hub>>, tab: Option<String>, kbd: Option<bool>) {
    hub.set_mode(tab.as_deref(), kbd);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStatus { url: Option<String>, upload_url: Option<String>, clients: Vec<Cursor> }

#[tauri::command(async)]
pub fn remote_status(hub: tauri::State<Arc<Hub>>) -> RemoteStatus {
    RemoteStatus { url: hub.url(), upload_url: hub.upload_url(), clients: hub.clients.lock().unwrap().values().cloned().collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un "móvil" de verdad (Node + WebSocket) mueve el puntero real del PC
    #[test]
    fn movil_mueve_el_raton_real() {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let rt = tokio::runtime::Runtime::new().unwrap();
        let hub = Hub::new(None);
        let url = format!("ws://127.0.0.1:7779/ws?t={}", hub.token);
        rt.spawn(serve(hub.clone(), 7779));
        std::thread::sleep(Duration::from_millis(500));

        let script = format!(
            "const ws=new WebSocket('{url}');ws.binaryType='arraybuffer';\
             ws.onopen=()=>{{ws.send(JSON.stringify({{t:'hello',name:'Prueba'}}));ws.send(new Int16Array([40,25]).buffer);setTimeout(()=>process.exit(0),400)}};\
             ws.onerror=()=>process.exit(2);"
        );
        let ok = std::process::Command::new("node").args(["-e", &script]).status().unwrap().success();
        assert!(ok, "el cliente WebSocket no pudo conectar");

        let (w, h) = input::screen_size();
        let mut p = POINT::default();
        unsafe { GetCursorPos(&mut p).unwrap() };
        assert_eq!((p.x, p.y), (w / 2 + 40, h / 2 + 25), "el puntero debe moverse 40,25 desde el centro");
    }

    #[test]
    fn pagina_protegida_por_token() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let hub = Hub::new(None);
        let token = hub.token.clone();
        rt.spawn(serve(hub, 7790));
        std::thread::sleep(Duration::from_millis(500));
        let code = |url: &str| {
            let out = std::process::Command::new("curl.exe").args(["-s", "-o", "NUL", "-w", "%{http_code}", "--max-time", "5", url]).output().unwrap();
            String::from_utf8_lossy(&out.stdout).into_owned()
        };
        assert_eq!(code("http://127.0.0.1:7790/?t=mal"), "403");
        assert_eq!(code(&format!("http://127.0.0.1:7790/?t={token}")), "200");
        // Petición WebSocket de verdad: con código malo se rechaza, con el bueno se acepta (101)
        let ws_code = |url: &str| {
            let out = std::process::Command::new("curl.exe")
                .args(["-s", "-o", "NUL", "-w", "%{http_code}", "--max-time", "2",
                    "-H", "Connection: Upgrade", "-H", "Upgrade: websocket",
                    "-H", "Sec-WebSocket-Version: 13", "-H", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", url])
                .output().unwrap();
            String::from_utf8_lossy(&out.stdout).into_owned()
        };
        assert_eq!(ws_code("http://127.0.0.1:7790/ws?t=mal"), "403");
        assert_eq!(ws_code(&format!("http://127.0.0.1:7790/ws?t={token}")), "101");
    }

    /// Subir desde el navegador: un archivo grande de verdad, sin cargarlo en memoria, y nada fuera de su carpeta
    #[test]
    fn subir_archivos_desde_el_navegador() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let hub = Hub::new(None);
        let token = hub.token.clone();
        rt.spawn(serve(hub, 7791));
        std::thread::sleep(Duration::from_millis(500));

        let base = std::env::temp_dir().join(format!("consola-subir-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let src = base.join("grande.bin");
        let data: Vec<u8> = (0..200u32 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
        std::fs::write(&src, &data).unwrap();
        let name = format!("prueba-consola-{}.bin", std::process::id());
        let put = |q: &str| {
            let out = std::process::Command::new("curl.exe")
                .args(["-s", "-o", "NUL", "-w", "%{http_code} %{speed_upload}", "-T", &src.to_string_lossy(), &format!("http://127.0.0.1:7791/api/subir?{q}")])
                .output().unwrap();
            String::from_utf8_lossy(&out.stdout).into_owned()
        };
        // Sin código: rechazado
        assert!(put(&format!("t=mal&d=recibidos&n={name}")).starts_with("403"));
        // Intento de salir de la carpeta: rechazado
        assert!(put(&format!("t={token}&d=recibidos&n=..%2F..%2Fmalo.bin")).starts_with("400"));
        // Carpeta inventada: rechazada
        assert!(put(&format!("t={token}&d=no.existe&n={name}")).starts_with("404"));
        // 200 MB a Recibidos
        let r = put(&format!("t={token}&d=recibidos&n={name}"));
        assert!(r.starts_with("200"), "{r}");
        let saved = crate::receive::received_dir().join(&name);
        assert_eq!(std::fs::read(&saved).unwrap(), data);
        let mb_s = r.split(' ').nth(1).unwrap().parse::<f64>().unwrap() / 1e6;
        println!("Velocidad de subida en local: {mb_s:.0} MB/s");
        assert!(mb_s > 100.0, "demasiado lento: {mb_s} MB/s");
        // Lista de carpetas: Recibidos siempre está
        let out = std::process::Command::new("curl.exe").args(["-s", &format!("http://127.0.0.1:7791/api/destinos?t={token}")]).output().unwrap();
        assert!(String::from_utf8_lossy(&out.stdout).contains(r#""id":"recibidos""#));

        let _ = std::fs::remove_file(saved);
        let _ = std::fs::remove_dir_all(&base);
    }
}
