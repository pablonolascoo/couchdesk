//! Recibir con LocalSend (app gratuita para móvil y PC: github.com/localsend/localsend).
//!
//! La consola aparece sola en la lista de LocalSend de cualquier dispositivo de casa:
//! - Descubrimiento por multidifusión UDP (224.0.0.167:53317), protocolo v2.
//! - Servidor en el puerto 53317: register, info, prepare-upload, upload y cancel.
//!   Atiende HTTPS (cifrado, lo normal en LocalSend) y HTTP en el mismo puerto: se mira el
//!   primer byte de cada conexión (0x16 = saludo TLS). El certificado es propio y se guarda
//!   en la carpeta de datos (%LOCALAPPDATA%\CouchDesk); su huella SHA-256 es el «fingerprint» que pide el protocolo.
//!
//! Seguridad:
//! - Solo se atiende a la red de casa (`receive::is_lan`).
//! - Cada envío se acepta solo si la pantalla «Recibir con LocalSend» está abierta,
//!   o si alguien pulsa «Aceptar» en la consola.
//! - Los archivos van al destino elegido (Recibidos por defecto) con nombres saneados
//!   y nunca se sobrescribe nada (`transfer::save`).

use crate::receive::{is_lan, random_password};
use axum::body::Body;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Emitter;
use tokio::sync::oneshot;

pub const PORT: u16 = 53317;
const MULTICAST: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 167);
/// Tiempo para pulsar «Aceptar» en la consola
const ASK_TIMEOUT: Duration = Duration::from_secs(60);
/// Un envío sin movimiento durante este tiempo se da por abandonado
const SESSION_IDLE: Duration = Duration::from_secs(120);
/// La pantalla avisa cada segundo; si deja de hacerlo, ya no se acepta solo
const SCREEN_ALIVE: Duration = Duration::from_secs(5);

type Resolver = Box<dyn Fn(&str) -> Option<PathBuf> + Send + Sync>;

pub struct LocalSend {
    alias: String,
    model: String,
    fingerprint: String,
    /// HTTPS; si no se pudo preparar el certificado, solo HTTP
    tls: Option<tokio_rustls::TlsAcceptor>,
    app: Option<tauri::AppHandle>,
    /// id de destino → carpeta real
    resolve: Resolver,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    dest: String,
    screen_seen: Option<Instant>,
    running: Option<Result<(), String>>,
    session: Option<Session>,
    pending: Option<Pending>,
    recent: Vec<Received>,
}

struct Session {
    id: String,
    ip: IpAddr,
    from: String,
    dir: PathBuf,
    files: HashMap<String, Incoming>,
    last: Instant,
}

struct Incoming {
    token: String,
    rel: PathBuf,
    size: u64,
    done: bool,
}

struct Pending {
    id: String,
    from: String,
    count: usize,
    size: u64,
    names: Vec<String>,
    answer: oneshot::Sender<bool>,
}

#[derive(Serialize, Clone)]
pub struct Received {
    name: String,
    path: String,
    from: String,
}

impl LocalSend {
    pub fn new(app: Option<tauri::AppHandle>) -> Arc<Self> {
        let resolve: Resolver = Box::new(|id| crate::transfer::destinations().into_iter().find(|d| d.id == id).map(|d| d.path));
        let identity = saved_identity(&crate::paths::data_dir());
        if let Err(e) = &identity {
            log::warn!("LocalSend: sin HTTPS ({e})");
        }
        Self::with_resolver(app, resolve, identity.ok())
    }

    fn with_resolver(app: Option<tauri::AppHandle>, resolve: Resolver, identity: Option<Identity>) -> Arc<Self> {
        let pc = std::env::var("COMPUTERNAME").unwrap_or_default();
        let tls = identity.as_ref().and_then(|id| match tls_acceptor(id) {
            Ok(a) => Some(a),
            Err(e) => {
                log::warn!("LocalSend: sin HTTPS ({e})");
                None
            }
        });
        let fingerprint = match (&tls, &identity) {
            (Some(_), Some(id)) => cert_fingerprint(&id.cert),
            // Sin certificado el protocolo pide una cadena aleatoria
            _ => format!("{}{}{}", random_password(), random_password(), random_password()),
        };
        Arc::new(Self {
            alias: if pc.is_empty() { "CouchDesk".into() } else { format!("CouchDesk ({pc})") },
            model: "Windows".into(),
            fingerprint,
            tls,
            app,
            resolve,
            inner: Mutex::new(Inner { dest: "recibidos".into(), ..Default::default() }),
        })
    }

    fn info(&self, announce: Option<bool>) -> serde_json::Value {
        let mut v = serde_json::json!({
            "alias": self.alias,
            "version": "2.0",
            "deviceModel": self.model,
            "deviceType": "desktop",
            "fingerprint": self.fingerprint,
            "port": PORT,
            "protocol": if self.tls.is_some() { "https" } else { "http" },
            "download": false,
        });
        if let Some(a) = announce {
            v["announce"] = a.into();
            v["announcement"] = a.into(); // v1
        }
        v
    }
}

// ---------------- Certificado (HTTPS) ----------------

/// Certificado propio y su clave, en DER
pub struct Identity {
    cert: Vec<u8>,
    key: Vec<u8>,
}

fn new_identity() -> Result<Identity, String> {
    let ck = rcgen::generate_simple_self_signed(vec!["localsend".into()]).map_err(|e| e.to_string())?;
    Ok(Identity { cert: ck.cert.der().to_vec(), key: ck.signing_key.serialize_der() })
}

/// Se crea una vez y se reutiliza: así la huella no cambia y LocalSend reconoce la consola
fn saved_identity(dir: &std::path::Path) -> Result<Identity, String> {
    let (cert, key) = (dir.join("localsend-cert.der"), dir.join("localsend-key.der"));
    if let (Ok(cert), Ok(key)) = (std::fs::read(&cert), std::fs::read(&key)) {
        let id = Identity { cert, key };
        if tls_acceptor(&id).is_ok() {
            return Ok(id);
        }
    }
    let id = new_identity()?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(&cert, &id.cert).map_err(|e| e.to_string())?;
    std::fs::write(&key, &id.key).map_err(|e| e.to_string())?;
    Ok(id)
}

fn tls_acceptor(id: &Identity) -> Result<tokio_rustls::TlsAcceptor, String> {
    use tokio_rustls::rustls::{self, pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer}};
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(id.cert.clone())],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(id.key.clone())),
        )
        .map_err(|e| e.to_string())?;
    Ok(tokio_rustls::TlsAcceptor::from(Arc::new(config)))
}

/// Huella del certificado como la calcula LocalSend: SHA-256 del DER, en hexadecimal
fn cert_fingerprint(der: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, der).as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------- HTTPS y HTTP en el mismo puerto ----------------

/// Conexión ya preparada: cifrada o tal cual
pub enum Conn {
    Plain(tokio::net::TcpStream),
    Tls(Box<tokio_rustls::server::TlsStream<tokio::net::TcpStream>>),
}

impl tokio::io::AsyncRead for Conn {
    fn poll_read(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>, buf: &mut tokio::io::ReadBuf<'_>) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Conn::Plain(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            Conn::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for Conn {
    fn poll_write(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>, buf: &[u8]) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Conn::Plain(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            Conn::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_write(cx, buf),
        }
    }
    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Conn::Plain(s) => std::pin::Pin::new(s).poll_flush(cx),
            Conn::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_flush(cx),
        }
    }
    fn poll_shutdown(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Conn::Plain(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            Conn::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_shutdown(cx),
        }
    }
}

/// Acepta conexiones y prepara cada una aparte (un saludo TLS lento no frena a las demás)
pub struct DualListener {
    rx: tokio::sync::mpsc::Receiver<(Conn, SocketAddr)>,
    local: SocketAddr,
}

impl DualListener {
    pub fn new(tcp: tokio::net::TcpListener, tls: Option<tokio_rustls::TlsAcceptor>) -> std::io::Result<Self> {
        let local = tcp.local_addr()?;
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        tokio::spawn(async move {
            loop {
                let (stream, addr) = match tcp.accept().await {
                    Ok(c) => c,
                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };
                let (conn_tx, tls) = (tx.clone(), tls.clone());
                tokio::spawn(async move {
                    let prepared = tokio::time::timeout(Duration::from_secs(10), async {
                        let mut first = [0u8; 1];
                        let n = stream.peek(&mut first).await.ok()?;
                        match tls {
                            Some(tls) if n == 1 && first[0] == 0x16 => tls.accept(stream).await.ok().map(|s| Conn::Tls(Box::new(s))),
                            _ => Some(Conn::Plain(stream)),
                        }
                    })
                    .await;
                    if let Ok(Some(conn)) = prepared {
                        let _ = conn_tx.send((conn, addr)).await;
                    }
                });
                if tx.is_closed() {
                    break;
                }
            }
        });
        Ok(Self { rx, local })
    }
}

impl axum::serve::Listener for DualListener {
    type Io = Conn;
    type Addr = SocketAddr;
    async fn accept(&mut self) -> (Conn, SocketAddr) {
        match self.rx.recv().await {
            Some(c) => c,
            None => std::future::pending().await,
        }
    }
    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok(self.local)
    }
}

/// Sirve el router en un puerto ya abierto, con HTTPS y HTTP a la vez
async fn serve_on(ls: Arc<LocalSend>, tcp: tokio::net::TcpListener) -> std::io::Result<()> {
    use axum::serve::ListenerExt;
    // tap_io: así ConnectInfo<SocketAddr> sirve con este listener propio
    let listener = DualListener::new(tcp, ls.tls.clone())?.tap_io(|_| {});
    let app = router(ls).into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app).await
}

// ---------------- HTTP ----------------

pub fn router(ls: Arc<LocalSend>) -> Router {
    Router::new()
        .route("/api/localsend/v2/register", post(register))
        .route("/api/localsend/v2/info", get(info))
        .route("/api/localsend/v1/info", get(info))
        .route("/api/localsend/v2/prepare-upload", post(prepare_upload))
        .route("/api/localsend/v2/upload", post(upload))
        .route("/api/localsend/v2/cancel", post(cancel))
        .layer(DefaultBodyLimit::disable())
        .layer(axum::middleware::from_fn(only_lan))
        .with_state(ls)
}

async fn only_lan(ConnectInfo(addr): ConnectInfo<SocketAddr>, req: Request, next: Next) -> Response {
    if !is_lan(addr.ip().to_canonical()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    next.run(req).await
}

async fn register(State(ls): State<Arc<LocalSend>>) -> Json<serde_json::Value> {
    Json(ls.info(None))
}

async fn info(State(ls): State<Arc<LocalSend>>) -> Json<serde_json::Value> {
    Json(ls.info(None))
}

#[derive(Deserialize)]
struct PrepareBody {
    info: PeerInfo,
    #[serde(default)]
    files: HashMap<String, FileMeta>,
}

#[derive(Deserialize)]
struct PeerInfo {
    alias: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileMeta {
    id: String,
    file_name: String,
    #[serde(default)]
    size: u64,
}

async fn prepare_upload(
    State(ls): State<Arc<LocalSend>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<PrepareBody>,
) -> Response {
    if body.files.is_empty() {
        return StatusCode::NO_CONTENT.into_response();
    }
    let from = body.info.alias.chars().take(60).collect::<String>();
    let ip = addr.ip().to_canonical();
    // Solo nombres seguros, siempre dentro del destino
    let files: Vec<(String, PathBuf, u64)> = body
        .files
        .values()
        .filter_map(|f| Some((f.id.clone(), crate::transfer::safe_relative(&f.file_name)?, f.size)))
        .collect();
    if files.is_empty() {
        return (StatusCode::BAD_REQUEST, crate::i18n::tr("localsend.bad_names")).into_response();
    }

    // ¿Aceptar? Solo, con la pantalla abierta; si no, se pregunta en la consola
    let rx = {
        let mut g = ls.inner.lock().unwrap();
        if g.session.as_ref().is_some_and(|s| s.last.elapsed() < SESSION_IDLE) || g.pending.is_some() {
            return StatusCode::CONFLICT.into_response();
        }
        g.session = None;
        if ls.auto_accept_locked(&g) {
            None
        } else {
            let (tx, rx) = oneshot::channel();
            let id = random_password();
            g.pending = Some(Pending {
                id: id.clone(),
                from: from.clone(),
                count: files.len(),
                size: files.iter().map(|f| f.2).sum(),
                names: files.iter().take(3).map(|f| f.1.to_string_lossy().into_owned()).collect(),
                answer: tx,
            });
            Some((id, rx))
        }
    };
    if let Some((id, rx)) = rx {
        if let Some(app) = &ls.app {
            let _ = app.emit("localsend-request", &id);
        }
        let ok = matches!(tokio::time::timeout(ASK_TIMEOUT, rx).await, Ok(Ok(true)));
        let mut g = ls.inner.lock().unwrap();
        if g.pending.as_ref().is_some_and(|p| p.id == id) {
            g.pending = None;
        }
        if !ok {
            return StatusCode::FORBIDDEN.into_response();
        }
    }

    let mut g = ls.inner.lock().unwrap();
    let Some(dir) = (ls.resolve)(&g.dest).or_else(|| (ls.resolve)("recibidos")) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let session = Session {
        id: random_password(),
        ip,
        from,
        dir,
        files: files
            .into_iter()
            .map(|(id, rel, size)| (id, Incoming { token: random_password(), rel, size, done: false }))
            .collect(),
        last: Instant::now(),
    };
    let tokens: HashMap<&String, &String> = session.files.iter().map(|(id, f)| (id, &f.token)).collect();
    let reply = serde_json::json!({ "sessionId": session.id, "files": tokens });
    g.session = Some(session);
    Json(reply).into_response()
}

impl LocalSend {
    fn auto_accept_locked(&self, g: &Inner) -> bool {
        g.screen_seen.is_some_and(|t| t.elapsed() < SCREEN_ALIVE)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadQuery {
    session_id: String,
    file_id: String,
    token: String,
}

async fn upload(
    State(ls): State<Arc<LocalSend>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(q): Query<UploadQuery>,
    body: Body,
) -> Response {
    let (dir, rel) = {
        let mut g = ls.inner.lock().unwrap();
        let Some(s) = g.session.as_mut().filter(|s| s.id == q.session_id) else {
            return StatusCode::CONFLICT.into_response();
        };
        if s.ip != addr.ip().to_canonical() {
            return StatusCode::FORBIDDEN.into_response();
        }
        let Some(f) = s.files.get(&q.file_id).filter(|f| f.token == q.token && !f.done) else {
            return StatusCode::FORBIDDEN.into_response();
        };
        s.last = Instant::now();
        (s.dir.clone(), f.rel.clone())
    };
    let saved = crate::transfer::save(&dir, &rel, body).await;
    let mut g = ls.inner.lock().unwrap();
    let Some(s) = g.session.as_mut().filter(|s| s.id == q.session_id) else {
        return StatusCode::CONFLICT.into_response();
    };
    s.last = Instant::now();
    match saved {
        Ok(path) => {
            if let Some(f) = s.files.get_mut(&q.file_id) {
                f.done = true;
            }
            let item = Received {
                name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                path: path.to_string_lossy().into_owned(),
                from: s.from.clone(),
            };
            let finished = s.files.values().all(|f| f.done);
            if finished {
                g.session = None;
            }
            g.recent.insert(0, item);
            g.recent.truncate(30);
            StatusCode::OK.into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CancelQuery {
    session_id: String,
}

async fn cancel(State(ls): State<Arc<LocalSend>>, Query(q): Query<CancelQuery>) -> StatusCode {
    let mut g = ls.inner.lock().unwrap();
    if g.session.as_ref().is_some_and(|s| s.id == q.session_id) {
        g.session = None;
    }
    StatusCode::OK
}

// ---------------- Descubrimiento (multidifusión) ----------------

async fn discovery(ls: Arc<LocalSend>) -> std::io::Result<()> {
    use socket2::{Domain, Protocol, SockRef, Socket, Type};
    let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    s.set_reuse_address(true)?;
    s.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, PORT)).into())?;
    s.set_nonblocking(true)?;
    s.set_multicast_loop_v4(false)?;
    // En las tarjetas de casa (así no se cuela una VPN)
    let ifaces: Vec<Ipv4Addr> = crate::receive::physical_lan_ips().iter().filter_map(|ip| ip.parse().ok()).collect();
    let ifaces = if ifaces.is_empty() { vec![Ipv4Addr::UNSPECIFIED] } else { ifaces };
    for ip in &ifaces {
        let _ = s.join_multicast_v4(&MULTICAST, ip);
    }
    let sock = tokio::net::UdpSocket::from_std(s.into())?;
    let target = SocketAddr::from((MULTICAST, PORT));
    let send_all = |msg: Vec<u8>| {
        let (sock, ifaces) = (&sock, &ifaces);
        async move {
            for ip in ifaces {
                let _ = SockRef::from(sock).set_multicast_if_v4(ip);
                let _ = sock.send_to(&msg, target).await;
            }
        }
    };
    // Al arrancar: «aquí estoy»
    send_all(ls.info(Some(true)).to_string().into_bytes()).await;

    let mut buf = vec![0u8; 8192];
    loop {
        let (n, from) = sock.recv_from(&mut buf).await?;
        if !is_lan(from.ip()) {
            continue;
        }
        let Ok(msg) = serde_json::from_slice::<serde_json::Value>(&buf[..n]) else { continue };
        if msg["fingerprint"].as_str() == Some(ls.fingerprint.as_str()) {
            continue;
        }
        let asks = msg["announce"].as_bool().or(msg["announcement"].as_bool()).unwrap_or(false);
        if asks {
            // Respuesta por UDP (vale para todas las versiones de LocalSend)
            send_all(ls.info(Some(false)).to_string().into_bytes()).await;
        }
    }
}

/// Arranca el servidor y el descubrimiento. Si el puerto está ocupado (LocalSend abierto
/// en este mismo PC), se apunta el error para la pantalla.
pub async fn serve(ls: Arc<LocalSend>) {
    let listener = match tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, PORT))).await {
        Ok(l) => l,
        Err(e) => {
            // Solo el detalle del sistema: el texto se monta al pedirlo, en el idioma de ese momento
            ls.inner.lock().unwrap().running = Some(Err(e.to_string()));
            return;
        }
    };
    ls.inner.lock().unwrap().running = Some(Ok(()));
    let d = ls.clone();
    tokio::spawn(async move {
        if let Err(e) = discovery(d).await {
            log::warn!("LocalSend (descubrimiento): {e}");
        }
    });
    if let Err(e) = serve_on(ls, listener).await {
        log::error!("LocalSend: {e}");
    }
}

// ---------------- Comandos para la interfaz ----------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    running: bool,
    error: Option<String>,
    alias: String,
    ip: Option<String>,
    port: u16,
    dest: String,
    dest_label: String,
    dest_path: String,
    active: Option<Active>,
    pending: Option<Ask>,
    recent: Vec<Received>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Active {
    from: String,
    done: usize,
    total: usize,
    size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ask {
    id: String,
    from: String,
    count: usize,
    size: u64,
    names: Vec<String>,
}

fn status_of(ls: &LocalSend, screen: bool) -> Status {
    let dests = crate::transfer::destinations();
    let mut g = ls.inner.lock().unwrap();
    if screen {
        g.screen_seen = Some(Instant::now());
    }
    let dest = dests.iter().find(|d| d.id == g.dest).or_else(|| dests.first());
    Status {
        running: matches!(g.running, Some(Ok(()))),
        error: g.running.as_ref().and_then(|r| r.clone().err()).map(|e| {
            crate::i18n::trf("localsend.port_busy", &[("port", &PORT.to_string()), ("err", &e)])
        }),
        alias: ls.alias.clone(),
        ip: crate::receive::lan_ip(),
        port: PORT,
        dest: g.dest.clone(),
        dest_label: dest.map(|d| if d.group == d.label { d.label.clone() } else { format!("{} · {}", d.group, d.label) }).unwrap_or_default(),
        dest_path: dest.map(|d| d.path.to_string_lossy().into_owned()).unwrap_or_default(),
        active: g.session.as_ref().filter(|s| s.last.elapsed() < SESSION_IDLE).map(|s| Active {
            from: s.from.clone(),
            done: s.files.values().filter(|f| f.done).count(),
            total: s.files.len(),
            size: s.files.values().map(|f| f.size).sum(),
        }),
        pending: g.pending.as_ref().map(|p| Ask { id: p.id.clone(), from: p.from.clone(), count: p.count, size: p.size, names: p.names.clone() }),
        recent: g.recent.clone(),
    }
}

/// `screen`: la pantalla de LocalSend está abierta (mientras lo esté, se acepta solo)
#[tauri::command(async)]
pub fn localsend_status(state: tauri::State<Arc<LocalSend>>, screen: Option<bool>) -> Status {
    status_of(&state, screen.unwrap_or(false))
}

/// Dónde se puede guardar: Recibidos y las carpetas de cada emulador
#[tauri::command(async)]
pub fn localsend_destinations() -> Vec<crate::transfer::Dest> {
    crate::transfer::destinations()
}

#[tauri::command(async)]
pub fn localsend_set_dest(state: tauri::State<Arc<LocalSend>>, id: String) -> Result<Status, String> {
    if !crate::transfer::destinations().iter().any(|d| d.id == id) {
        return Err(crate::i18n::tr("transfer.unknown_folder").into());
    }
    state.inner.lock().unwrap().dest = id;
    Ok(status_of(&state, true))
}

#[tauri::command(async)]
pub fn localsend_answer(state: tauri::State<Arc<LocalSend>>, id: String, accept: bool) {
    answer(&state, &id, accept);
}

fn answer(ls: &LocalSend, id: &str, accept: bool) {
    let mut g = ls.inner.lock().unwrap();
    if g.pending.as_ref().is_some_and(|p| p.id == id) {
        if let Some(p) = g.pending.take() {
            let _ = p.answer.send(accept);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn start(dir: &std::path::Path) -> (Arc<LocalSend>, String) {
        let root = dir.to_path_buf();
        let ls = LocalSend::with_resolver(None, Box::new(move |id| (id == "recibidos").then(|| root.clone())), Some(new_identity().unwrap()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        // Cifrado, como envía LocalSend por defecto
        let base = format!("https://{}/api/localsend/v2", listener.local_addr().unwrap());
        tokio::spawn(serve_on(ls.clone(), listener));
        (ls, base)
    }

    /// Cliente como el de LocalSend: acepta certificados propios (no hay autoridad en casa)
    fn client() -> reqwest::Client {
        reqwest::Client::builder().danger_accept_invalid_certs(true).build().unwrap()
    }

    #[tokio::test]
    async fn https_y_http_en_el_mismo_puerto() {
        let dir = std::env::temp_dir().join(format!("consola-localsend-dual-{}", std::process::id()));
        let (ls, base) = start(&dir).await;
        // La huella es el SHA-256 del certificado y se anuncia HTTPS
        assert_eq!(ls.fingerprint.len(), 64);
        assert!(ls.fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
        let me: serde_json::Value = client().get(format!("{base}/info")).send().await.unwrap().json().await.unwrap();
        assert_eq!(me["fingerprint"], ls.fingerprint.as_str());
        assert_eq!(ls.info(Some(true))["protocol"], "https");
        // Un emisor sin cifrado también entra por el mismo puerto
        let plain = base.replace("https://", "http://");
        let me: serde_json::Value = reqwest::Client::new().post(format!("{plain}/register")).json(&serde_json::json!({})).send().await.unwrap().json().await.unwrap();
        assert_eq!(me["version"], "2.0");
    }

    #[test]
    fn certificado_guardado_se_reutiliza() {
        let dir = std::env::temp_dir().join(format!("consola-localsend-cert-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = saved_identity(&dir).unwrap();
        let b = saved_identity(&dir).unwrap();
        assert_eq!(cert_fingerprint(&a.cert), cert_fingerprint(&b.cert));
        // Archivo roto: se crea otro
        std::fs::write(dir.join("localsend-key.der"), b"roto").unwrap();
        let c = saved_identity(&dir).unwrap();
        assert_ne!(cert_fingerprint(&a.cert), cert_fingerprint(&c.cert));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn prepare(files: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "info": { "alias": "Móvil de prueba", "version": "2.0", "deviceModel": "Pixel", "deviceType": "mobile",
                      "fingerprint": "abc", "port": 53317, "protocol": "https", "download": false },
            "files": files,
        })
    }

    #[tokio::test]
    async fn envio_completo_por_localsend() {
        let dir = std::env::temp_dir().join(format!("consola-localsend-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("juego.iso"), b"ya estaba").unwrap();
        let (ls, base) = start(&dir).await;
        let http = client();

        // Descubrimiento por HTTP
        let me: serde_json::Value = http.post(format!("{base}/register")).json(&serde_json::json!({})).send().await.unwrap().json().await.unwrap();
        assert_eq!(me["version"], "2.0");
        assert_eq!(me["fingerprint"], ls.fingerprint.as_str());

        // Sin la pantalla abierta y sin respuesta: se pregunta; «Rechazar» → 403
        let ls2 = ls.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(30)).await;
                let id = ls2.inner.lock().unwrap().pending.as_ref().map(|p| p.id.clone());
                if let Some(id) = id {
                    answer(&ls2, &id, false);
                    break;
                }
            }
        });
        let files = serde_json::json!({
            "f1": { "id": "f1", "fileName": "juego.iso", "size": 9, "fileType": "application/octet-stream" },
            "f2": { "id": "f2", "fileName": "Carpeta/../../fuera.bin", "size": 3, "fileType": "application/octet-stream" },
            "f3": { "id": "f3", "fileName": "Saves/partida.sav", "size": 4, "fileType": "application/octet-stream" },
        });
        let r = http.post(format!("{base}/prepare-upload")).json(&prepare(files.clone())).send().await.unwrap();
        assert_eq!(r.status(), 403);

        // Con la pantalla abierta se acepta solo
        ls.inner.lock().unwrap().screen_seen = Some(Instant::now());
        let r: serde_json::Value = http.post(format!("{base}/prepare-upload")).json(&prepare(files)).send().await.unwrap().json().await.unwrap();
        let sid = r["sessionId"].as_str().unwrap().to_string();
        // La ruta con «..» no recibe permiso
        assert!(r["files"].get("f2").is_none());
        let token = |id: &str| r["files"][id].as_str().unwrap().to_string();

        // Mientras dura, otro envío espera
        let busy = http.post(format!("{base}/prepare-upload")).json(&prepare(serde_json::json!({"x": {"id":"x","fileName":"x","size":1,"fileType":"a"}}))).send().await.unwrap();
        assert_eq!(busy.status(), 409);

        // Token falso → 403
        let bad = http.post(format!("{base}/upload?sessionId={sid}&fileId=f1&token=falso")).body("x").send().await.unwrap();
        assert_eq!(bad.status(), 403);

        for (id, data) in [("f1", &b"nuevo iso"[..]), ("f3", &b"save"[..])] {
            let up = http
                .post(format!("{base}/upload?sessionId={sid}&fileId={id}&token={}", token(id)))
                .body(data.to_vec())
                .send()
                .await
                .unwrap();
            assert_eq!(up.status(), 200, "{id}");
        }
        // Nada se sobrescribe; las subcarpetas se respetan
        assert_eq!(std::fs::read(dir.join("juego.iso")).unwrap(), b"ya estaba");
        assert_eq!(std::fs::read(dir.join("juego (2).iso")).unwrap(), b"nuevo iso");
        assert_eq!(std::fs::read(dir.join("Saves").join("partida.sav")).unwrap(), b"save");
        assert!(!dir.parent().unwrap().join("fuera.bin").exists());

        // Envío terminado: la sesión se cierra y queda en «recibidos»
        let g = ls.inner.lock().unwrap();
        assert!(g.session.is_none());
        assert_eq!(g.recent.len(), 2);
        assert_eq!(g.recent[1].from, "Móvil de prueba");
        drop(g);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
