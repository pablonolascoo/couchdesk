//! "Recibir archivos": servidor FTP/FTPS solo para la red de casa.
//! Usuario fijo "consola", contraseña aleatoria nueva cada vez que se enciende.

use async_trait::async_trait;
use serde::Serialize;
use std::net::{IpAddr, UdpSocket};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tokio::sync::oneshot;
use unftp_core::auth::{AuthenticationError, Authenticator, Credentials, Principal};

pub const PORT: u16 = 2121;
const USER: &str = "consola";

#[derive(Debug)]
struct OneUser { password: String }

#[async_trait]
impl Authenticator for OneUser {
    async fn authenticate(&self, username: &str, creds: &Credentials) -> Result<Principal, AuthenticationError> {
        // Solo direcciones de red local
        if !is_lan(creds.source_ip) {
            return Err(AuthenticationError::BadUser);
        }
        if username == USER && creds.password.as_deref() == Some(self.password.as_str()) {
            Ok(Principal { username: username.into() })
        } else {
            Err(AuthenticationError::BadPassword)
        }
    }
}

pub fn is_lan(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || (v6.segments()[0] & 0xfe00) == 0xfc00 || (v6.segments()[0] & 0xffc0) == 0xfe80,
    }
}

/// IP de este PC en la red de casa.
/// Se busca en las tarjetas físicas (cable o Wi-Fi) con puerta de enlace: así no se cuela
/// la IP de una VPN (Cloudflare WARP, Hyper-V…), que el móvil no puede alcanzar.
pub fn lan_ip() -> Option<String> {
    physical_lan_ips().into_iter().next().or_else(|| {
        // Último recurso: la ruta que usaría el sistema para salir (no envía nada)
        let s = UdpSocket::bind("0.0.0.0:0").ok()?;
        s.connect("8.8.8.8:80").ok()?;
        Some(s.local_addr().ok()?.ip().to_string())
    })
}

/// IPs privadas de tarjetas de red reales y activas; primero el cable, luego el Wi-Fi
pub fn physical_lan_ips() -> Vec<String> {
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
        GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};

    const ETHERNET: u32 = 6;
    const WIFI: u32 = 71;
    let flags = GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size: u32 = 32 * 1024;
    let mut buf: Vec<u64> = Vec::new(); // u64 para que quede bien alineado
    for _ in 0..3 {
        buf = vec![0u64; size as usize / 8 + 1];
        let r = unsafe {
            GetAdaptersAddresses(u32::from(AF_INET.0), flags, None, Some(buf.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>()), &mut size)
        };
        if r == 0 { break }
        if r != 111 { return vec![] } // 111 = ERROR_BUFFER_OVERFLOW: repetir con más sitio
    }

    let mut found: Vec<(u32, String)> = Vec::new();
    let mut p = buf.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
    while !p.is_null() {
        let a = unsafe { &*p };
        let name = unsafe { a.FriendlyName.to_string() }.unwrap_or_default().to_lowercase();
        let virtual_name = ["warp", "vpn", "vethernet", "virtual", "wsl", "tailscale", "zerotier", "hamachi"]
            .iter()
            .any(|v| name.contains(v));
        if a.OperStatus == IfOperStatusUp
            && !a.FirstGatewayAddress.is_null()
            && (a.IfType == ETHERNET || a.IfType == WIFI)
            && !virtual_name
        {
            let mut u = a.FirstUnicastAddress;
            while !u.is_null() {
                let ua = unsafe { &*u };
                let sa = ua.Address.lpSockaddr;
                if !sa.is_null() && unsafe { (*sa).sa_family } == AF_INET {
                    let sin = unsafe { &*sa.cast::<SOCKADDR_IN>() };
                    let ip = std::net::Ipv4Addr::from(u32::from_be(unsafe { sin.sin_addr.S_un.S_addr }));
                    if ip.is_private() {
                        found.push((a.IfType, ip.to_string()));
                    }
                }
                u = ua.Next;
            }
        }
        p = a.Next;
    }
    found.sort_by_key(|(t, _)| u32::from(*t != ETHERNET)); // cable primero
    found.into_iter().map(|(_, ip)| ip).collect()
}

pub fn random_password() -> String {
    // 10 caracteres sin letras que se confundan (0/O, 1/l)
    const ABC: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
    let mut bytes = [0u8; 10];
    getrandom(&mut bytes);
    bytes.iter().map(|b| ABC[*b as usize % ABC.len()] as char).collect()
}

fn getrandom(buf: &mut [u8]) {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    for chunk in buf.chunks_mut(8) {
        let mut h = RandomState::new().build_hasher();
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let v = h.finish().to_le_bytes();
        chunk.copy_from_slice(&v[..chunk.len()]);
    }
}

pub fn received_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("C:\\"));
    home.join("Recibidos")
}

/// Certificado propio para FTPS (cifrado). Se crea una vez y se reutiliza.
fn ensure_cert(dir: &std::path::Path) -> Result<(PathBuf, PathBuf), String> {
    let (cert, key) = (dir.join("ftps-cert.pem"), dir.join("ftps-key.pem"));
    if !cert.exists() || !key.exists() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let ck = rcgen::generate_simple_self_signed(vec!["consola.local".into()]).map_err(|e| e.to_string())?;
        std::fs::write(&cert, ck.cert.pem()).map_err(|e| e.to_string())?;
        std::fs::write(&key, ck.signing_key.serialize_pem()).map_err(|e| e.to_string())?;
    }
    Ok((cert, key))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReceiveStatus {
    running: bool,
    ip: Option<String>,
    port: u16,
    user: String,
    password: Option<String>,
    folder: String,
    url: Option<String>,
}

#[derive(Default)]
pub struct ReceiveState(Mutex<Option<(oneshot::Sender<()>, String)>>);

fn status(state: &ReceiveState) -> ReceiveStatus {
    let guard = state.0.lock().unwrap();
    let password = guard.as_ref().map(|(_, p)| p.clone());
    let ip = lan_ip();
    ReceiveStatus {
        running: guard.is_some(),
        url: match (&ip, &password) {
            (Some(ip), Some(p)) => Some(format!("ftp://{USER}:{p}@{ip}:{PORT}/")),
            _ => None,
        },
        ip,
        port: PORT,
        user: USER.into(),
        password,
        folder: received_dir().to_string_lossy().into_owned(),
    }
}

/// Lo que llega a «PCSX2 - BIOS/…» se coloca solo en la carpeta real del emulador
#[derive(Debug)]
struct PlaceUploads { root: PathBuf, map: Vec<(String, PathBuf)> }

#[async_trait]
impl libunftp::notification::DataListener for PlaceUploads {
    async fn receive_data_event(&self, e: libunftp::notification::DataEvent, _m: libunftp::notification::EventMeta) {
        if let libunftp::notification::DataEvent::Put { path, .. } = e {
            let (root, map) = (self.root.clone(), self.map.clone());
            let _ = tokio::task::spawn_blocking(move || crate::transfer::route_ftp_upload(&root, &map, &path)).await;
        }
    }
}

fn build_server(
    folder: PathBuf,
    map: Vec<(String, PathBuf)>,
    password: String,
    cert: PathBuf,
    key: PathBuf,
    stop: oneshot::Receiver<()>,
) -> Result<libunftp::Server<unftp_sbe_fs::Filesystem, unftp_core::auth::DefaultUser>, String> {
    let listener = PlaceUploads { root: folder.clone(), map };
    libunftp::ServerBuilder::with_authenticator(
        Box::new(move || unftp_sbe_fs::Filesystem::new(folder.clone()).unwrap()),
        Arc::new(OneUser { password }),
    )
    .notify_data(listener)
    .greeting(crate::i18n::tr("receive.greeting"))
    .passive_ports(50000..=50100)
    .ftps(cert, key)
    .shutdown_indicator(async move {
        let _ = stop.await;
        libunftp::options::Shutdown::new().grace_period(std::time::Duration::from_secs(2))
    })
    .build()
    .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn receive_status(state: tauri::State<ReceiveState>) -> ReceiveStatus {
    status(&state)
}

#[tauri::command]
pub async fn receive_start(app: tauri::AppHandle, state: tauri::State<'_, ReceiveState>) -> Result<ReceiveStatus, String> {
    if state.0.lock().unwrap().is_some() {
        return Ok(status(&state));
    }
    let folder = received_dir();
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let (cert, key) = ensure_cert(&data_dir)?;
    let password = random_password();
    let (tx, rx) = oneshot::channel::<()>();
    let map = crate::transfer::prepare_ftp_root(&folder);
    let server = build_server(folder, map, password.clone(), cert, key, rx)?;

    // Comprobar que el puerto está libre antes de anunciarlo
    std::net::TcpListener::bind(("0.0.0.0", PORT)).map_err(|_| crate::i18n::trf("receive.port_busy", &[("port", &PORT.to_string())]))?;
    tauri::async_runtime::spawn(async move {
        if let Err(e) = server.listen(format!("0.0.0.0:{PORT}")).await {
            log::error!("FTP: {e}");
        }
    });
    *state.0.lock().unwrap() = Some((tx, password));
    Ok(status(&state))
}

#[tauri::command(async)]
pub fn receive_stop(state: tauri::State<ReceiveState>) -> ReceiveStatus {
    if let Some((tx, _)) = state.0.lock().unwrap().take() {
        let _ = tx.send(());
    }
    status(&state)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La IP del QR es la de la red de casa, nunca la de una VPN como Cloudflare WARP
    #[test]
    fn ip_de_la_red_de_casa() {
        let ip = lan_ip().unwrap();
        println!("IP del QR: {ip}");
        assert!(!physical_lan_ips().is_empty(), "debe encontrar una tarjeta de red real");
        assert_eq!(ip, physical_lan_ips()[0]);
        assert!(!ip.starts_with("172.16.0."), "no debe usar la IP de WARP");
    }

    #[test]
    fn solo_red_local() {
        assert!(is_lan("192.168.1.20".parse().unwrap()));
        assert!(is_lan("10.0.0.5".parse().unwrap()));
        assert!(is_lan("127.0.0.1".parse().unwrap()));
        assert!(!is_lan("8.8.8.8".parse().unwrap()));
        assert!(!is_lan("2a00:1450::1".parse().unwrap()));
    }

    /// Servidor real en 127.0.0.1 + curl.exe de Windows como cliente
    #[test]
    fn subir_archivo_por_ftp_y_ftps() {
        let base = std::env::temp_dir().join(format!("consola-ftp-{}", std::process::id()));
        let inbox = base.join("Recibidos");
        std::fs::create_dir_all(&inbox).unwrap();
        let (cert, key) = ensure_cert(&base).unwrap();
        let src = base.join("hola.txt");
        std::fs::write(&src, "hola desde el movil").unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (tx, rx) = oneshot::channel();
        let bios = base.join("emu").join("bios");
        std::fs::create_dir_all(inbox.join("PCSX2 - BIOS")).unwrap();
        let map = vec![("PCSX2 - BIOS".to_string(), bios.clone())];
        let server = build_server(inbox.clone(), map, "clave123".into(), cert, key, rx).unwrap();
        rt.spawn(async move { server.listen("127.0.0.1:2122").await.unwrap() });
        std::thread::sleep(std::time::Duration::from_millis(800));

        let curl = |args: &[&str]| std::process::Command::new("curl.exe").args(args).output().unwrap().status.success();
        let s = src.to_string_lossy().into_owned();
        // FTP normal
        assert!(curl(&["-s", "-T", &s, "ftp://consola:clave123@127.0.0.1:2122/"]));
        assert_eq!(std::fs::read_to_string(inbox.join("hola.txt")).unwrap(), "hola desde el movil");
        // FTPS (cifrado; -k porque el certificado es propio)
        assert!(curl(&["-s", "-k", "--ssl-reqd", "-T", &s, "ftp://consola:clave123@127.0.0.1:2122/cifrado.txt"]));
        assert!(inbox.join("cifrado.txt").exists());
        // A la carpeta de un emulador: se coloca sola en su sitio real
        assert!(curl(&["-s", "-T", &s, "ftp://consola:clave123@127.0.0.1:2122/PCSX2%20-%20BIOS/scph.bin"]));
        let placed = bios.join("scph.bin");
        for _ in 0..50 { if placed.exists() { break } std::thread::sleep(std::time::Duration::from_millis(50)); }
        assert_eq!(std::fs::read_to_string(&placed).unwrap(), "hola desde el movil");
        assert!(!inbox.join("PCSX2 - BIOS").join("scph.bin").exists());
        // Contraseña mala: rechazado
        assert!(!curl(&["-s", "-T", &s, "ftp://consola:mala@127.0.0.1:2122/malo.txt"]));
        assert!(!inbox.join("malo.txt").exists());

        let _ = tx.send(());
        drop(rt);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn contrasenas_distintas_y_legibles() {
        let (a, b) = (random_password(), random_password());
        assert_eq!(a.len(), 10);
        assert_ne!(a, b);
        assert!(!a.contains('0') && !a.contains('l'));
    }
}
