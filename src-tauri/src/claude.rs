//! Claude dentro de la consola, con la cuenta del usuario (Claude Code CLI).
//! Claude solo puede usar las herramientas de la consola (servidor MCP propio):
//! sin terminal, sin editar archivos, sin borrar ni apagar.

use serde::Serialize;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

const SYSTEM: &str = "Eres el asistente de una consola de juegos para Windows que se maneja con mando. \
Responde siempre en español, en frases cortas y sin jerga técnica. \
Para actuar usa las herramientas de la consola (volumen, brillo, energía, juegos, emuladores, archivos). \
No puedes borrar archivos, apagar el equipo ni ejecutar programas: si te lo piden, explica cómo hacerlo desde la consola. \
Nunca descargues ni ayudes a conseguir juegos, BIOS o claves que no sean del usuario.";

#[derive(Default)]
pub struct ClaudeState(Mutex<Option<String>>); // id de la conversación para seguir hablando

#[derive(Serialize, Clone)]
struct Event { kind: &'static str, text: String }

pub fn claude_exe() -> Option<PathBuf> {
    let local = PathBuf::from(std::env::var("USERPROFILE").ok()?).join(".local").join("bin").join("claude.exe");
    if local.exists() {
        return Some(local);
    }
    let out = Command::new("where").arg("claude").creation_flags(0x0800_0000).output().ok()?;
    String::from_utf8_lossy(&out.stdout).lines().next().map(PathBuf::from)
}

/// Texto legible para "Claude está usando…"
pub fn tool_label(name: &str) -> String {
    name.trim_start_matches("mcp__consola__").replace('_', " ")
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
                events.push(("error", v.get("result").and_then(Value::as_str).unwrap_or("Error").to_string()));
            }
            events.push(("done", String::new()));
        }
        _ => {}
    }
    (events, session)
}

#[tauri::command(async)]
pub fn claude_available() -> bool {
    claude_exe().is_some()
}

#[tauri::command(async)]
pub fn claude_reset(state: tauri::State<ClaudeState>) {
    *state.0.lock().unwrap() = None;
}

#[tauri::command(async)]
pub fn claude_ask(app: tauri::AppHandle, prompt: String) -> Result<(), String> {
    let exe = claude_exe().ok_or("Claude Code no está instalado")?;
    let data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let work = data.join("claude");
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let config = work.join("mcp.json");
    let cfg = serde_json::json!({"mcpServers": {"consola": {"command": me, "args": ["--mcp"]}}});
    std::fs::write(&config, cfg.to_string()).map_err(|e| e.to_string())?;

    let session = app.state::<ClaudeState>().0.lock().unwrap().clone();
    let mut cmd = Command::new(exe);
    cmd.current_dir(&work)
        .args(["-p", &prompt, "--output-format", "stream-json", "--verbose"])
        .arg("--mcp-config").arg(&config)
        .args(["--strict-mcp-config", "--tools", "", "--allowedTools", "mcp__consola"])
        .args(["--append-system-prompt", SYSTEM])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x0800_0000);
    if let Some(id) = session {
        cmd.args(["--resume", &id]);
    }
    let mut child = cmd.spawn().map_err(|e| format!("No se pudo abrir Claude: {e}"))?;
    let stdout = child.stdout.take().unwrap();

    std::thread::spawn(move || {
        let emit = |kind: &'static str, text: String| { let _ = app.emit("claude-event", Event { kind, text }); };
        let mut finished = false;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let (events, sid) = parse_line(&line);
            if let Some(sid) = sid {
                *app.state::<ClaudeState>().0.lock().unwrap() = Some(sid);
            }
            for (k, t) in events {
                finished |= k == "done";
                emit(k, t);
            }
        }
        let status = child.wait();
        if !finished {
            let mut err = String::new();
            if let Some(mut e) = child.stderr.take() {
                use std::io::Read;
                let _ = e.read_to_string(&mut err);
            }
            let msg = if err.contains("login") || err.contains("auth") { "Hay que iniciar sesión en Claude Code otra vez." } else { "Claude no respondió." };
            emit("error", format!("{msg} {}", err.lines().last().unwrap_or("")).trim().to_string());
            emit("done", String::new());
        }
        let _ = status;
    });
    Ok(())
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
    }
}
