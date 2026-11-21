//! Claude dentro de la consola, con la cuenta del usuario (Claude Code CLI).
//! Claude solo puede usar las herramientas de la consola (servidor MCP propio) y los
//! conectores que el usuario añada: sin terminal, sin editar archivos, sin borrar ni apagar.
//!
//! Todo vive en %LOCALAPPDATA%\Consola\claude:
//! · conversaciones.json  lista de conversaciones (las comparten la ventana principal y la flotante)
//! · conectores.json      servidores MCP remotos que el usuario añadió
//! · mcp.json             configuración que se le pasa a Claude Code en cada pregunta
//! · acciones\*.json      canal del servidor MCP (otro proceso) hacia la consola: abrir pantallas,
//!                        webs o volver al Inicio. La consola lo vigila cada 300 ms.

use crate::i18n::{lookup, tr, trf};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager};

const NO_WINDOW: u32 = 0x0800_0000;

/// `__IDIOMA__` se cambia por el idioma de la consola (ver `system_prompt`)
const SYSTEM: &str = "Eres el asistente de una consola de juegos para Windows que se maneja con mando. \
Responde siempre en __IDIOMA__, en frases cortas y sin jerga técnica. Tus respuestas se pueden leer en voz alta: sé breve y escribe texto normal, sin tablas, listas ni formato markdown. \
Para actuar usa las herramientas de la consola (volumen, brillo, energía, juegos, aplicaciones, emuladores, archivos, \
abrir pantallas de la consola, abrir webs y buscar en YouTube). \
Solo puedes abrir juegos y aplicaciones de la lista de la consola. \
No puedes borrar archivos, apagar el equipo ni ejecutar comandos: si te lo piden, explica cómo hacerlo desde la consola. \
Nunca descargues ni ayudes a conseguir juegos, BIOS o claves que no sean del usuario.";

/// Instrucciones para Claude, pidiendo que responda en el idioma elegido en la consola
pub fn system_prompt() -> String {
    SYSTEM.replace("__IDIOMA__", crate::i18n::language_name())
}

/// Última conversación usada (la de la ventana flotante si no se elige otra)
#[derive(Default)]
pub struct ClaudeState(Mutex<Option<String>>);

/// Carpeta de trabajo de Claude en la consola
pub fn dir() -> PathBuf {
    crate::paths::data_dir().join("claude")
}

pub fn claude_exe() -> Option<PathBuf> {
    let local = PathBuf::from(std::env::var("USERPROFILE").ok()?).join(".local").join("bin").join("claude.exe");
    if local.exists() {
        return Some(local);
    }
    let out = Command::new("where").arg("claude").creation_flags(NO_WINDOW).output().ok()?;
    String::from_utf8_lossy(&out.stdout).lines().next().map(PathBuf::from)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Texto legible para "Claude está usando…"
pub fn tool_label(name: &str) -> String {
    match name.strip_prefix("mcp__") {
        Some(rest) => {
            let (server, tool) = rest.split_once("__").unwrap_or(("", rest));
            let t = tool.replace(['_', '-'], " ");
            if server == "consola" || server.is_empty() {
                // Herramientas propias: nombre traducido (claves tool.<nombre>)
                lookup(&format!("tool.{tool}")).map(String::from).unwrap_or(t)
            } else {
                format!("{t} ({server})")
            }
        }
        None => name.replace('_', " "),
    }
}

/// Convierte una línea del flujo JSON de Claude Code en eventos para la pantalla
pub fn parse_line(line: &str) -> (Vec<(&'static str, String)>, Option<String>) {
    let mut events = Vec::new();
    let Ok(v) = serde_json::from_str::<Value>(line) else { return (events, None) };
    let session = v.get("session_id").and_then(Value::as_str).map(String::from);
    match v.get("type").and_then(Value::as_str) {
        Some("assistant") => {
            for block in v.pointer("/message/content").and_then(Value::as_array).into_iter().flatten() {
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => events.push(("text", block["text"].as_str().unwrap_or("").to_string())),
                    Some("tool_use") => events.push(("tool", tool_label(block["name"].as_str().unwrap_or("")))),
                    _ => {}
                }
            }
        }
        Some("result") => {
            if v.get("is_error").and_then(Value::as_bool) == Some(true) {
                let raw = v.get("result").and_then(Value::as_str).unwrap_or("Error");
                events.push(("error", explain_error(raw)));
            }
            events.push(("done", String::new()));
        }
        _ => {}
    }
    (events, session)
}

/// Errores de Claude Code en palabras sencillas
pub fn explain_error(raw: &str) -> String {
    let l = raw.to_lowercase();
    if l.contains("login") || l.contains("api key") || l.contains("auth") || l.contains("401") {
        tr("claude.not_logged_in").into()
    } else if l.contains("usage limit") || l.contains("rate limit") || l.contains("limit reached") {
        tr("claude.usage_limit").into()
    } else if l.contains("network") || l.contains("enotfound") || l.contains("econn") || l.contains("fetch failed") {
        tr("claude.no_network").into()
    } else if l.contains("no conversation found") || l.contains("session") && l.contains("not found") {
        tr("claude.conv_gone").into()
    } else {
        trf("claude.problem", &[("err", raw.lines().next().unwrap_or(raw).trim())])
    }
}

// ---------------------------------------------------------------- Conversaciones

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChatMsg {
    /// user | claude | tool | error
    pub role: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    /// Id de sesión de Claude Code (sirve para --resume)
    pub id: String,
    pub title: String,
    pub updated_at: i64,
    #[serde(default)]
    pub messages: Vec<ChatMsg>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConvSummary {
    pub id: String,
    pub title: String,
    pub updated_at: i64,
    pub count: usize,
}

static STORE: Mutex<()> = Mutex::new(());
const MAX_MSGS: usize = 300;
const MAX_CONVS: usize = 200;

fn convs_path() -> PathBuf {
    dir().join("conversaciones.json")
}

pub fn parse_convs(text: &str) -> Vec<Conversation> {
    serde_json::from_str(text).unwrap_or_default()
}

fn load_convs() -> Vec<Conversation> {
    std::fs::read_to_string(convs_path()).map(|t| parse_convs(&t)).unwrap_or_default()
}

fn save_convs(list: &[Conversation]) -> Result<(), String> {
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    // Escribir aparte y cambiar de nombre: nunca queda un archivo a medias
    let tmp = dir().join("conversaciones.json.tmp");
    std::fs::write(&tmp, serde_json::to_string(list).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, convs_path()).map_err(|e| e.to_string())
}

/// Título = primera pregunta, en una línea y sin pasar de 60 letras
pub fn make_title(prompt: &str) -> String {
    let one = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.is_empty() {
        return tr("claude.conversation").into();
    }
    if one.chars().count() <= 60 {
        return one;
    }
    let cut: String = one.chars().take(59).collect();
    // Cortar en el último espacio si queda razonable
    let cut = match cut.rfind(' ') {
        Some(i) if i > 30 => cut[..i].to_string(),
        _ => cut,
    };
    format!("{}…", cut.trim_end_matches([',', '.', ';', ':', ' ']))
}

/// Añade el turno a su conversación (o crea una nueva). `old` es el id con el que se reanudó.
pub fn upsert(list: &mut Vec<Conversation>, old: Option<&str>, sid: &str, prompt: &str, turn: Vec<ChatMsg>, now: i64) {
    let pos = list.iter().position(|c| c.id == sid).or_else(|| old.and_then(|o| list.iter().position(|c| c.id == o)));
    let mut conv = match pos {
        Some(i) => list.remove(i),
        None => Conversation { id: sid.into(), title: make_title(prompt), updated_at: now, messages: vec![] },
    };
    conv.id = sid.into();
    conv.updated_at = now;
    conv.messages.extend(turn);
    if conv.messages.len() > MAX_MSGS {
        let extra = conv.messages.len() - MAX_MSGS;
        conv.messages.drain(..extra);
    }
    // La más reciente, la primera
    list.insert(0, conv);
    list.truncate(MAX_CONVS);
}

fn save_turn(old: Option<&str>, sid: &str, prompt: &str, turn: Vec<ChatMsg>) {
    let _g = STORE.lock().unwrap();
    let mut list = load_convs();
    upsert(&mut list, old, sid, prompt, turn, now_ms());
    if let Err(e) = save_convs(&list) {
        log::warn!("Conversaciones: {e}");
    }
}

#[tauri::command(async)]
pub fn claude_conversations() -> Vec<ConvSummary> {
    let _g = STORE.lock().unwrap();
    let mut list = load_convs();
    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    list.into_iter().map(|c| ConvSummary { count: c.messages.len(), id: c.id, title: c.title, updated_at: c.updated_at }).collect()
}

#[tauri::command(async)]
pub fn claude_conversation(id: String) -> Result<Conversation, String> {
    let _g = STORE.lock().unwrap();
    load_convs().into_iter().find(|c| c.id == id).ok_or_else(|| tr("claude.conv_not_found").into())
}

#[tauri::command(async)]
pub fn claude_conversation_delete(id: String) -> Result<(), String> {
    let _g = STORE.lock().unwrap();
    let mut list = load_convs();
    list.retain(|c| c.id != id);
    save_convs(&list)
}

// ---------------------------------------------------------------- Conectores

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Connector {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "yes")]
    pub enabled: bool,
}
fn yes() -> bool {
    true
}

#[derive(Serialize, Clone, Debug)]
pub struct CatalogItem {
    pub name: &'static str,
    pub title: &'static str,
    pub url: &'static str,
    pub description: &'static str,
}

/// Tienda: servidores MCP remotos sin cuenta. Todos respondieron a `initialize` (nov. 2026).
pub const CATALOG: &[CatalogItem] = &[
    CatalogItem { name: "deepwiki", title: "DeepWiki", url: "https://mcp.deepwiki.com/mcp", description: "Explica proyectos de GitHub y responde dudas sobre ellos." },
    CatalogItem { name: "context7", title: "Context7", url: "https://mcp.context7.com/mcp", description: "Documentación al día de miles de programas y librerías." },
    CatalogItem { name: "microsoft-learn", title: "Microsoft Learn", url: "https://learn.microsoft.com/api/mcp", description: "Ayuda oficial de Windows, Xbox, Office y más." },
    CatalogItem { name: "huggingface", title: "Hugging Face", url: "https://huggingface.co/mcp", description: "Busca modelos y demos de inteligencia artificial." },
    CatalogItem { name: "cloudflare-docs", title: "Cloudflare Docs", url: "https://docs.mcp.cloudflare.com/mcp", description: "Ayuda de Cloudflare: WARP, DNS y webs." },
    CatalogItem { name: "gitmcp", title: "GitMCP", url: "https://gitmcp.io/docs", description: "Lee la documentación de cualquier proyecto de GitHub." },
];

fn connectors_path() -> PathBuf {
    dir().join("conectores.json")
}

pub fn load_connectors() -> Vec<Connector> {
    std::fs::read_to_string(connectors_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save_connectors(list: &[Connector]) -> Result<(), String> {
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    std::fs::write(connectors_path(), serde_json::to_string_pretty(list).unwrap()).map_err(|e| e.to_string())
}

/// Nombre válido para Claude Code: minúsculas, números, - y _
pub fn safe_name(s: &str) -> String {
    let n: String = s
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            c if c.is_ascii_alphanumeric() || c == '_' || c == '-' => c,
            _ => '-',
        })
        .collect();
    let n = n.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    n.chars().take(40).collect()
}

/// Configuración MCP: la consola siempre, más los conectores encendidos.
/// Devuelve también la lista de herramientas permitidas.
pub fn build_mcp_config(me: &Path, list: &[Connector]) -> (Value, Vec<String>) {
    let mut servers = serde_json::Map::new();
    servers.insert("consola".into(), json!({"command": me, "args": ["--mcp"]}));
    let mut allowed = vec!["mcp__consola".to_string()];
    for c in list.iter().filter(|c| c.enabled) {
        let name = safe_name(&c.name);
        if name.is_empty() || name == "consola" || servers.contains_key(&name) {
            continue;
        }
        if !(c.url.starts_with("https://") || c.url.starts_with("http://")) {
            continue;
        }
        servers.insert(name.clone(), json!({"type": "http", "url": c.url}));
        allowed.push(format!("mcp__{name}"));
    }
    (json!({ "mcpServers": servers }), allowed)
}

#[derive(Serialize)]
pub struct ConnectorsView {
    mine: Vec<Connector>,
    catalog: Vec<CatalogItem>,
}

#[tauri::command(async)]
pub fn claude_connectors() -> ConnectorsView {
    // Descripciones de la tienda en el idioma de la consola (claves claude.cat.<nombre>)
    let catalog = CATALOG
        .iter()
        .map(|c| CatalogItem { description: lookup(&format!("claude.cat.{}", c.name)).unwrap_or(c.description), ..c.clone() })
        .collect();
    ConnectorsView { mine: load_connectors(), catalog }
}

#[tauri::command(async)]
pub fn claude_connector_add(name: String, url: String, description: Option<String>) -> Result<Connector, String> {
    let url = url.trim().to_string();
    let parsed: tauri::Url = url.parse().map_err(|_| tr("claude.bad_url"))?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return Err(tr("claude.bad_url").into());
    }
    let mut n = safe_name(&name);
    if n.is_empty() {
        n = safe_name(parsed.host_str().unwrap_or("conector"));
    }
    if n == "consola" {
        n = "consola-2".into();
    }
    let mut list = load_connectors();
    if list.iter().any(|c| c.url == url) {
        return Err(tr("claude.connector_exists").into());
    }
    let base = n.clone();
    let mut i = 2;
    while list.iter().any(|c| c.name == n) {
        n = format!("{base}-{i}");
        i += 1;
    }
    let c = Connector { name: n, url, description: description.unwrap_or_default(), enabled: true };
    list.push(c.clone());
    save_connectors(&list)?;
    Ok(c)
}

#[tauri::command(async)]
pub fn claude_connector_toggle(name: String, enabled: bool) -> Result<(), String> {
    let mut list = load_connectors();
    let c = list.iter_mut().find(|c| c.name == name).ok_or(tr("claude.connector_not_found"))?;
    c.enabled = enabled;
    save_connectors(&list)
}

#[tauri::command(async)]
pub fn claude_connector_remove(name: String) -> Result<(), String> {
    let mut list = load_connectors();
    list.retain(|c| c.name != name);
    save_connectors(&list)
}

// ---------------------------------------------------------------- Preguntar

#[derive(Serialize, Clone)]
struct Event {
    tag: String,
    kind: &'static str,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    installed: bool,
    logged_in: Option<bool>,
}

#[tauri::command(async)]
pub fn claude_available() -> bool {
    claude_exe().is_some()
}

/// ¿Está instalado y con la sesión iniciada? (sin leer ni guardar datos de la cuenta)
#[tauri::command(async)]
pub fn claude_status() -> Status {
    let Some(exe) = claude_exe() else { return Status { installed: false, logged_in: None } };
    let out = Command::new(exe).args(["auth", "status", "--json"]).stdin(Stdio::null()).creation_flags(NO_WINDOW).output();
    let logged_in = out.ok().and_then(|o| serde_json::from_slice::<Value>(&o.stdout).ok()).and_then(|v| v.get("loggedIn").and_then(Value::as_bool));
    Status { installed: true, logged_in }
}

/// Se mantiene por compatibilidad: ahora cada pantalla dice qué conversación sigue
#[tauri::command(async)]
pub fn claude_reset(state: tauri::State<ClaudeState>) {
    *state.0.lock().unwrap() = None;
}

/// Pregunta a Claude. `conversation`: id para seguir una conversación (None = nueva).
/// `tag`: lo elige la pantalla; va en cada evento "claude-event" para no mezclar ventanas.
#[tauri::command(async)]
pub fn claude_ask(app: tauri::AppHandle, prompt: String, conversation: Option<String>, tag: Option<String>) -> Result<(), String> {
    let tag = tag.unwrap_or_default();
    let exe = claude_exe().ok_or(tr("claude.not_installed"))?;
    let work = dir();
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let config = work.join("mcp.json");
    let (cfg, allowed) = build_mcp_config(&me, &load_connectors());
    std::fs::write(&config, cfg.to_string()).map_err(|e| e.to_string())?;

    let conversation = conversation.filter(|c| !c.trim().is_empty());
    let mut cmd = Command::new(exe);
    cmd.current_dir(&work)
        .args(["-p", &prompt, "--output-format", "stream-json", "--verbose"])
        .arg("--mcp-config")
        .arg(&config)
        .args(["--strict-mcp-config", "--tools", ""])
        .arg("--allowedTools")
        .arg(allowed.join(","))
        .arg("--append-system-prompt")
        .arg(system_prompt())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(NO_WINDOW);
    if let Some(id) = &conversation {
        cmd.args(["--resume", id]);
    }
    let mut child = cmd.spawn().map_err(|e| trf("claude.cant_start", &[("err", &e.to_string())]))?;
    let stdout = child.stdout.take().unwrap();

    std::thread::spawn(move || {
        let emit = |kind: &'static str, text: String| {
            let _ = app.emit("claude-event", Event { tag: tag.clone(), kind, text });
        };
        let mut turn = vec![ChatMsg { role: "user".into(), text: prompt.clone() }];
        let mut sid: Option<String> = None;
        let mut finished = false;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let (events, s) = parse_line(&line);
            if let Some(s) = s {
                if sid.as_deref() != Some(&s) {
                    *app.state::<ClaudeState>().0.lock().unwrap() = Some(s.clone());
                    emit("session", s.clone());
                    sid = Some(s);
                }
            }
            for (k, t) in events {
                finished |= k == "done";
                match k {
                    "text" if !t.trim().is_empty() => turn.push(ChatMsg { role: "claude".into(), text: t.clone() }),
                    "tool" => turn.push(ChatMsg { role: "tool".into(), text: t.clone() }),
                    "error" => turn.push(ChatMsg { role: "error".into(), text: t.clone() }),
                    _ => {}
                }
                if k != "done" {
                    emit(k, t);
                }
            }
            if finished {
                break;
            }
        }
        let _ = child.wait();
        if !finished {
            let mut err = String::new();
            if let Some(mut e) = child.stderr.take() {
                use std::io::Read;
                let _ = e.read_to_string(&mut err);
            }
            let msg = if err.trim().is_empty() { tr("claude.no_reply").to_string() } else { explain_error(&err) };
            turn.push(ChatMsg { role: "error".into(), text: msg.clone() });
            emit("error", msg);
        }
        // Guardar antes de avisar del final: así la lista ya está al día
        if let Some(s) = &sid {
            save_turn(conversation.as_deref(), s, &prompt, turn);
        }
        emit("done", String::new());
    });
    Ok(())
}

// ---------------------------------------------------------------- Extensiones (plugins de Claude Code)

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Plugins {
    installed: Vec<Value>,
    available: Vec<Value>,
}

fn run_claude(args: &[&str], timeout: Duration) -> Result<(bool, String, String), String> {
    let exe = claude_exe().ok_or(tr("claude.not_installed"))?;
    std::fs::create_dir_all(dir()).ok();
    let mut child = Command::new(exe)
        .args(args)
        .current_dir(dir())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(NO_WINDOW)
        .spawn()
        .map_err(|e| e.to_string())?;
    // Leer en hilos aparte para que una salida grande no bloquee
    let mut so = child.stdout.take().unwrap();
    let mut se = child.stderr.take().unwrap();
    let t1 = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = std::io::Read::read_to_string(&mut so, &mut s);
        s
    });
    let t2 = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = std::io::Read::read_to_string(&mut se, &mut s);
        s
    });
    let start = std::time::Instant::now();
    let ok = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st.success(),
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                return Err(tr("claude.timeout").into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => return Err(e.to_string()),
        }
    };
    Ok((ok, t1.join().unwrap_or_default(), t2.join().unwrap_or_default()))
}

/// Ordena y resume la lista de `claude plugin list --json --available`
pub fn parse_plugins(text: &str) -> Plugins {
    let Ok(v) = serde_json::from_str::<Value>(text) else { return Plugins::default() };
    let installed: Vec<Value> = v["installed"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            let scope = p["scope"].as_str().unwrap_or("");
            json!({
                "id": p["id"], "version": p["version"], "enabled": p["enabled"],
                // Las de la organización (synced/managed) no se pueden quitar desde aquí
                "managed": scope != "user" && scope != "project" && scope != "local",
            })
        })
        .collect();
    let have: std::collections::HashSet<&str> = installed.iter().filter_map(|p| p["id"].as_str()).collect();
    let mut available: Vec<&Value> = v["available"].as_array().into_iter().flatten().filter(|p| !have.contains(p["pluginId"].as_str().unwrap_or(""))).collect();
    available.sort_by_key(|p| std::cmp::Reverse(p["installCount"].as_u64().unwrap_or(0)));
    let available = available
        .into_iter()
        .take(60)
        .map(|p| json!({"id": p["pluginId"], "name": p["name"], "description": p["description"], "installs": p["installCount"]}))
        .collect();
    Plugins { installed, available }
}

fn valid_plugin_id(id: &str) -> bool {
    !id.is_empty() && id.len() < 120 && id.chars().all(|c| c.is_ascii_alphanumeric() || "@._-".contains(c)) && !id.starts_with('-')
}

#[tauri::command(async)]
pub fn claude_plugins() -> Result<Plugins, String> {
    let (_, out, err) = run_claude(&["plugin", "list", "--json", "--available"], Duration::from_secs(60))?;
    if out.trim().is_empty() {
        return Err(trf("claude.plugins_failed", &[("err", err.lines().last().unwrap_or(""))]).trim().to_string());
    }
    Ok(parse_plugins(&out))
}

fn plugin_result(ok: bool, out: &str, err: &str, done: &str) -> Result<String, String> {
    let msg = out
        .lines()
        .rev()
        .find_map(|l| serde_json::from_str::<Value>(l).ok())
        .and_then(|v| v.get("message").or(v.get("error")).and_then(Value::as_str).map(String::from));
    if ok {
        Ok(done.into())
    } else {
        Err(msg.or_else(|| err.lines().last().map(String::from)).unwrap_or_else(|| tr("claude.failed").into()))
    }
}

#[tauri::command(async)]
pub fn claude_plugin_install(id: String) -> Result<String, String> {
    if !valid_plugin_id(&id) {
        return Err(tr("claude.bad_plugin").into());
    }
    // Sin -y: si la extensión necesita ejecutar algo para instalarse, Claude Code se niega y lo decimos
    let (ok, out, err) = run_claude(&["plugin", "install", &id, "--json"], Duration::from_secs(180))?;
    plugin_result(ok, &out, &err, tr("claude.plugin_installed"))
}

#[tauri::command(async)]
pub fn claude_plugin_remove(id: String) -> Result<String, String> {
    if !valid_plugin_id(&id) {
        return Err(tr("claude.bad_plugin").into());
    }
    let (ok, out, err) = run_claude(&["plugin", "uninstall", &id, "--json"], Duration::from_secs(60))?;
    plugin_result(ok, &out, &err, tr("claude.plugin_removed"))
}

// ---------------------------------------------------------------- Acciones (MCP → consola)

/// Rutas de la consola que Claude puede abrir
pub const ROUTES: &[&str] = &[
    "/", "/biblioteca", "/aplicaciones", "/emuladores", "/archivos", "/navegador", "/nube", "/recibir", "/mando-movil",
    "/musica", "/claude", "/claude/conversaciones", "/claude/conectores", "/ajustes", "/ajustes/sonido", "/ajustes/pantalla",
    "/ajustes/energia", "/ajustes/red", "/ajustes/arranque", "/escritorio",
];
const ROUTE_PREFIXES: &[&str] = &["/juego/", "/emuladores/", "/aplicaciones/"];

pub fn valid_route(r: &str) -> bool {
    ROUTES.contains(&r)
        || ROUTE_PREFIXES.iter().any(|p| r.starts_with(p) && r.len() > p.len() && r[p.len()..].chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)))
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Action {
    /// ruta | web | inicio
    pub accion: String,
    #[serde(default)]
    pub valor: String,
    /// Momento en que se pidió (ms); las muy viejas se ignoran
    #[serde(default)]
    pub t: i64,
}

fn actions_dir() -> PathBuf {
    dir().join("acciones")
}

/// Lo llama el servidor MCP (otro proceso): deja el pedido en la carpeta de acciones
pub fn send_action(accion: &str, valor: &str) -> Result<(), String> {
    let d = actions_dir();
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let a = Action { accion: accion.into(), valor: valor.into(), t: now_ms() };
    let name = format!("{}-{}", a.t, std::process::id());
    let tmp = d.join(format!("{name}.tmp"));
    std::fs::write(&tmp, serde_json::to_string(&a).unwrap()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, d.join(format!("{name}.json"))).map_err(|e| e.to_string())
}

/// Recoge (y borra) los pedidos pendientes, en orden
pub fn take_actions(d: &Path, now: i64) -> Vec<Action> {
    let Ok(rd) = std::fs::read_dir(d) else { return vec![] };
    let mut files: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|e| e == "json")).collect();
    files.sort();
    let mut out = vec![];
    for f in files {
        let a = std::fs::read_to_string(&f).ok().and_then(|t| serde_json::from_str::<Action>(&t).ok());
        let _ = std::fs::remove_file(&f);
        if let Some(a) = a.filter(|a| now - a.t < 30_000) {
            out.push(a);
        }
    }
    out
}

fn run_action(app: &tauri::AppHandle, a: Action) {
    match a.accion.as_str() {
        "ruta" if valid_route(&a.valor) => {
            crate::browser::go_home(app);
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.emit("claude-navigate", a.valor);
            }
        }
        "web" if a.valor.starts_with("https://") || a.valor.starts_with("http://") => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = crate::browser::browser_open(app, a.valor).await {
                    log::warn!("Claude abrir web: {e}");
                }
            });
        }
        "inicio" => crate::browser::go_home(app),
        _ => log::warn!("Acción de Claude no válida: {a:?}"),
    }
}

/// Arranca la vigilancia de la carpeta de acciones (en el proceso principal)
pub fn start(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let d = actions_dir();
        let _ = std::fs::create_dir_all(&d);
        // Pedidos de una sesión anterior: fuera
        take_actions(&d, i64::MIN / 2);
        loop {
            std::thread::sleep(Duration::from_millis(300));
            for a in take_actions(&d, now_ms()) {
                run_action(&app, a);
            }
        }
    });
}

// ---------------------------------------------------------------- Asistente flotante

pub const ASSISTANT: &str = "asistente";
const ASSISTANT_W: f64 = 420.0;
const ASSISTANT_H: f64 = 640.0;

/// Abre o esconde el asistente flotante. Se puede llamar desde cualquier hilo.
pub fn toggle_assistant(app: &tauri::AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let visible = app.get_webview_window(ASSISTANT).and_then(|w| w.is_visible().ok()).unwrap_or(false);
        let r = if visible { hide_assistant(&app) } else { show_assistant(&app) };
        if let Err(e) = r {
            log::warn!("Asistente: {e}");
        }
    });
}

fn hide_assistant(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(ASSISTANT) {
        let _ = w.emit("assistant-hidden", ());
        w.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Monitor de la ventana que está delante (el juego o programa), o el principal
fn target_monitor(app: &tauri::AppHandle) -> Option<tauri::Monitor> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};
    let mut r = RECT::default();
    let ok = unsafe { GetWindowRect(GetForegroundWindow(), &mut r).is_ok() };
    if ok {
        let (x, y) = (f64::from(r.left + r.right) / 2.0, f64::from(r.top + r.bottom) / 2.0);
        if let Ok(Some(m)) = app.monitor_from_point(x, y) {
            return Some(m);
        }
    }
    app.primary_monitor().ok().flatten()
}

/// Esquina inferior derecha del área útil (sin la barra de tareas)
pub fn corner(area_x: i32, area_y: i32, area_w: u32, area_h: u32, scale: f64) -> (i32, i32, u32, u32) {
    let w = (ASSISTANT_W * scale).round() as u32;
    let h = ((ASSISTANT_H * scale).round() as u32).min(area_h.saturating_sub((32.0 * scale) as u32));
    let m = (24.0 * scale).round() as i32;
    (area_x + area_w as i32 - w as i32 - m, area_y + area_h as i32 - h as i32 - m, w, h)
}

fn show_assistant(app: &tauri::AppHandle) -> Result<(), String> {
    let w = match app.get_webview_window(ASSISTANT) {
        Some(w) => w,
        None => tauri::WebviewWindowBuilder::new(app, ASSISTANT, tauri::WebviewUrl::App("asistente-flotante".into()))
            .title(tr("claude.assistant_title"))
            .inner_size(ASSISTANT_W, ASSISTANT_H)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(true)
            .visible(false)
            .background_color(tauri::window::Color(251, 248, 242, 255))
            .build()
            .map_err(|e| e.to_string())?,
    };
    if let Some(m) = target_monitor(app) {
        let a = m.work_area();
        let (x, y, ww, hh) = corner(a.position.x, a.position.y, a.size.width, a.size.height, m.scale_factor());
        let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = w.set_size(tauri::PhysicalSize::new(ww, hh));
    }
    w.show().map_err(|e| e.to_string())?;
    let _ = w.set_always_on_top(true);
    let _ = w.set_focus();
    // Windows no deja quitar el foco a otro programa sin más: unirse un instante a su cola de teclado
    if let Ok(h) = w.hwnd() {
        let raw = h.0 as isize;
        let _ = app.run_on_main_thread(move || force_foreground(raw));
    }
    let _ = w.emit("assistant-shown", ());
    Ok(())
}

fn force_foreground(raw: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow};
    let hwnd = HWND(raw as *mut core::ffi::c_void);
    unsafe {
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, None);
        let me = GetCurrentThreadId();
        let attached = fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, true).as_bool();
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        if attached {
            let _ = AttachThreadInput(me, fg_thread, false);
        }
    }
}

#[tauri::command(async)]
pub fn assistant_toggle(app: tauri::AppHandle) {
    toggle_assistant(&app);
}

#[tauri::command(async)]
pub fn assistant_show(app: tauri::AppHandle) -> Result<(), String> {
    show_assistant(&app)
}

#[tauri::command(async)]
pub fn assistant_hide(app: tauri::AppHandle) -> Result<(), String> {
    hide_assistant(&app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpreta_el_flujo_de_claude() {
        let (_, sid) = parse_line(r#"{"type":"system","subtype":"init","session_id":"abc"}"#);
        assert_eq!(sid.as_deref(), Some("abc"));
        let (ev, _) = parse_line(r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"mcp__consola__poner_volumen","input":{"nivel":30}},{"type":"text","text":"Listo"}]}}"#);
        assert_eq!(ev, vec![("tool", "poner volumen".to_string()), ("text", "Listo".to_string())]);
        let (ev, _) = parse_line(r#"{"type":"result","is_error":false,"result":"Listo"}"#);
        assert_eq!(ev, vec![("done", String::new())]);
        let (ev, _) = parse_line(r#"{"type":"result","is_error":true,"result":"Invalid API key · Please run /login"}"#);
        assert!(ev[0].1.contains("sesión iniciada"));
        assert_eq!(tool_label("mcp__deepwiki__ask_question"), "ask question (deepwiki)");
    }

    #[test]
    fn titulos_de_conversacion() {
        assert_eq!(make_title("  ¿Qué   volumen\n tengo? "), "¿Qué volumen tengo?");
        assert_eq!(make_title("   "), "Conversación");
        let long = "Quiero que me digas qué juegos de carreras tengo instalados y cuál me recomiendas para jugar con mi hermano";
        let t = make_title(long);
        assert!(t.chars().count() <= 60, "{t}");
        assert!(t.ends_with('…'));
        assert!(!t.contains("  "));
    }

    #[test]
    fn conversaciones_se_guardan_y_leen() {
        let msg = |r: &str, t: &str| ChatMsg { role: r.into(), text: t.into() };
        let mut list = vec![];
        upsert(&mut list, None, "s1", "Hola", vec![msg("user", "Hola"), msg("claude", "¡Hola!")], 10);
        upsert(&mut list, None, "s2", "Otra", vec![msg("user", "Otra")], 20);
        // Seguir la primera: sube arriba y conserva el título
        upsert(&mut list, Some("s1"), "s1", "Y más", vec![msg("user", "Y más")], 30);
        assert_eq!(list[0].id, "s1");
        assert_eq!(list[0].title, "Hola");
        assert_eq!(list[0].messages.len(), 3);
        assert_eq!(list[0].updated_at, 30);
        // Si Claude Code cambia el id al reanudar, se sustituye
        upsert(&mut list, Some("s2"), "s3", "x", vec![msg("user", "x")], 40);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "s3");
        assert_eq!(list[0].title, "Otra");
        // Ida y vuelta por JSON (con updatedAt en camelCase)
        let text = serde_json::to_string(&list).unwrap();
        assert!(text.contains("\"updatedAt\":40"));
        assert_eq!(parse_convs(&text), list);
        assert!(parse_convs("basura").is_empty());
    }

    #[test]
    fn conectores_en_mcp_json() {
        let me = Path::new("C:\\Consola\\consola.exe");
        let list = vec![
            Connector { name: "deepwiki".into(), url: "https://mcp.deepwiki.com/mcp".into(), description: String::new(), enabled: true },
            Connector { name: "apagado".into(), url: "https://x.example/mcp".into(), description: String::new(), enabled: false },
            Connector { name: "consola".into(), url: "https://malo.example/mcp".into(), description: String::new(), enabled: true },
            Connector { name: "raro".into(), url: "file:///c:/x".into(), description: String::new(), enabled: true },
        ];
        let (cfg, allowed) = build_mcp_config(me, &list);
        assert_eq!(cfg["mcpServers"]["consola"]["args"][0], "--mcp");
        assert_eq!(cfg["mcpServers"]["deepwiki"], json!({"type": "http", "url": "https://mcp.deepwiki.com/mcp"}));
        assert!(cfg["mcpServers"].get("apagado").is_none());
        assert!(cfg["mcpServers"].get("raro").is_none());
        assert_eq!(cfg["mcpServers"]["consola"]["command"], "C:\\Consola\\consola.exe");
        assert_eq!(allowed, vec!["mcp__consola", "mcp__deepwiki"]);
        assert_eq!(safe_name(" Mi Música! "), "mi-musica");
    }

    #[test]
    fn rutas_y_acciones() {
        assert!(valid_route("/biblioteca"));
        assert!(valid_route("/ajustes/sonido"));
        assert!(valid_route("/juego/steam-570"));
        assert!(!valid_route("/juego/"));
        assert!(!valid_route("/juego/../../x"));
        assert!(!valid_route("https://malo"));
        let d = std::env::temp_dir().join(format!("consola-acciones-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("1.json"), r#"{"accion":"ruta","valor":"/biblioteca","t":5000}"#).unwrap();
        std::fs::write(d.join("2.json"), r#"{"accion":"inicio","t":1}"#).unwrap();
        std::fs::write(d.join("3.tmp"), "a medias").unwrap();
        let got = take_actions(&d, 31_000);
        assert_eq!(got, vec![Action { accion: "ruta".into(), valor: "/biblioteca".into(), t: 5000 }], "la vieja se descarta");
        let _ = std::fs::remove_file(d.join("3.tmp"));
        assert!(std::fs::read_dir(&d).unwrap().next().is_none(), "se borran todas");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn extensiones_y_esquina() {
        let p = parse_plugins(r#"{"installed":[{"id":"a@synced","version":"1","scope":"synced","enabled":true},{"id":"b@m","scope":"user","enabled":true}],
            "available":[{"pluginId":"b@m","name":"b","installCount":9},{"pluginId":"c@m","name":"c","description":"C","installCount":5},{"pluginId":"d@m","name":"d","installCount":50}]}"#);
        assert_eq!(p.installed[0]["managed"], true);
        assert_eq!(p.installed[1]["managed"], false);
        assert_eq!(p.available.iter().map(|v| v["id"].as_str().unwrap()).collect::<Vec<_>>(), vec!["d@m", "c@m"]);
        assert!(valid_plugin_id("figma@claude-plugins-official"));
        assert!(!valid_plugin_id("--yes"));
        assert!(!valid_plugin_id("a b"));
        // 1920x1040 de área útil a escala 1
        assert_eq!(corner(0, 0, 1920, 1040, 1.0), (1920 - 420 - 24, 1040 - 640 - 24, 420, 640));
    }
}
