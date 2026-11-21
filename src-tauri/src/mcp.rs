//! Servidor MCP (stdio) con las herramientas de la consola para Claude.
//! Claude Code lo arranca como `consola.exe --mcp`. Solo acciones seguras:
//! nada de borrar, apagar ni ejecutar programas arbitrarios.
//! Para manejar la consola (abrir pantallas, webs, volver al Inicio) deja un pedido en
//! %LOCALAPPDATA%\Consola\claude\acciones (ver claude.rs): la consola lo recoge al momento.

use crate::{apps, browser, claude, emulators, files, games, hardware, settings};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

fn tools() -> Value {
    let int = |desc: &str| json!({"type": "integer", "minimum": 0, "maximum": 100, "description": desc});
    let s = |desc: &str| json!({"type": "string", "description": desc});
    let tool = |name: &str, desc: &str, props: Value, req: &[&str]| {
        json!({"name": name, "description": desc, "inputSchema": {"type": "object", "properties": props, "required": req}})
    };
    json!([
        tool("ver_volumen", "Volumen actual (0-100) y si está silenciado.", json!({}), &[]),
        tool("poner_volumen", "Cambia el volumen general del PC.", json!({"nivel": int("Volumen 0-100")}), &["nivel"]),
        tool("silenciar", "Silencia o quita el silencio.", json!({"silencio": {"type": "boolean"}}), &["silencio"]),
        tool("ver_brillo", "Brillo actual de la pantalla del portátil (0-100).", json!({}), &[]),
        tool("poner_brillo", "Cambia el brillo de la pantalla del portátil.", json!({"nivel": int("Brillo 0-100")}), &["nivel"]),
        tool("ver_bateria", "Porcentaje de batería y si está enchufado.", json!({}), &[]),
        tool("ver_modos_energia", "Lista los planes de energía y cuál está activo.", json!({}), &[]),
        tool("poner_modo_energia", "Activa un plan de energía por su GUID.", json!({"guid": s("GUID del plan")}), &["guid"]),
        tool("ver_wifi", "Estado del Wi-Fi y redes guardadas.", json!({}), &[]),
        tool("ver_hardware", "Procesador, gráfica y memoria del PC.", json!({}), &[]),
        tool("listar_juegos", "Juegos instalados (Steam, Epic, GOG) con su id.", json!({}), &[]),
        tool("abrir_juego", "Lanza un juego por su id (de listar_juegos).", json!({"id": s("id del juego")}), &["id"]),
        tool("listar_emuladores", "Emuladores disponibles, si están instalados y si irán bien en este PC.", json!({}), &[]),
        tool("instalar_emulador", "Instala o actualiza un emulador oficial por su id.", json!({"id": s("id del emulador")}), &["id"]),
        tool("abrir_emulador", "Abre un emulador instalado por su id.", json!({"id": s("id del emulador")}), &["id"]),
        tool("listar_carpeta", "Lista archivos y carpetas de una ruta de Windows.", json!({"ruta": s("Ruta, p. ej. C:\\Users\\CASA\\Downloads")}), &["ruta"]),
        tool("lugares", "Carpetas personales y discos del PC.", json!({}), &[]),
        tool("abrir_archivo", "Abre un documento, imagen, vídeo o carpeta con su programa. No abre programas (.exe, .bat...).", json!({"ruta": s("Ruta del archivo")}), &["ruta"]),
        tool(
            "abrir_pantalla",
            &format!("Lleva la consola a una de sus pantallas. Rutas: {}. También /juego/<id>, /emuladores/<id>, /aplicaciones/<id>.", claude::ROUTES.join(", ")),
            json!({"ruta": s("Ruta de la pantalla, p. ej. /biblioteca o /ajustes/sonido")}),
            &["ruta"],
        ),
        tool("abrir_web", "Abre una web dentro del navegador de la consola (a pantalla completa, manejable con mando). Si no es una dirección, la busca.", json!({"url": s("Dirección, p. ej. https://twitch.tv, o texto a buscar")}), &["url"]),
        tool("buscar_en_youtube", "Busca vídeos en YouTube y abre los resultados en el navegador de la consola.", json!({"texto": s("Qué buscar")}), &["texto"]),
        tool("listar_aplicaciones", "Aplicaciones instaladas que la consola puede abrir, con su id.", json!({}), &[]),
        tool("abrir_aplicacion", "Abre una aplicación por su id (de listar_aplicaciones). Sirve también para juegos.", json!({"id": s("id de la aplicación")}), &["id"]),
        tool("volver_al_inicio", "Cierra el navegador o la nube y vuelve a la pantalla de Inicio de la consola.", json!({}), &[]),
        tool("musica_que_suena", "Qué canción suena ahora (Spotify u otro reproductor): título, artista, si está en pausa.", json!({}), &[]),
        tool("musica_control", "Controla la música: reproducir, pausa, alternar, siguiente o anterior.", json!({"accion": {"type": "string", "enum": ["play", "pause", "toggle", "next", "previous"]}}), &["accion"]),
        tool("musica_ver_volumen", "Volumen del reproductor de música (0-100).", json!({}), &[]),
        tool("musica_poner_volumen", "Cambia el volumen solo del reproductor de música, sin tocar el del PC.", json!({"nivel": int("Volumen 0-100")}), &["nivel"]),
        tool("abrir_spotify", "Abre la app de Spotify. Si no está instalada, abre Spotify en el navegador de la consola.", json!({}), &[]),
    ])
}

/// Dirección de búsqueda de YouTube
pub fn youtube_url(q: &str) -> String {
    let q = q.split_whitespace().collect::<Vec<_>>().join(" ");
    tauri::Url::parse_with_params("https://www.youtube.com/results", &[("search_query", q.as_str())]).map(|u| u.to_string()).unwrap_or_else(|_| "https://www.youtube.com".into())
}

/// Programas que Claude no debe abrir aunque salgan en el menú Inicio (terminales, registro…)
pub fn blocked_app(name: &str, app_id: &str) -> bool {
    let n = format!("{} {}", name.to_lowercase(), app_id.to_lowercase());
    ["powershell", "cmd.exe", "símbolo del sistema", "simbolo del sistema", "command prompt", "terminal", "wt.exe", "regedit", "registro", "registry", "windows tools", "herramientas de windows", "mmc", "wsl", "ubuntu", "bash", "git bash", "visual studio code", "code.exe", "devenv"]
        .iter()
        .any(|b| n.contains(b))
}

fn arg_str<'a>(a: &'a Value, k: &str) -> Result<&'a str, String> {
    a.get(k).and_then(Value::as_str).ok_or(format!("Falta «{k}»"))
}
fn arg_u8(a: &Value, k: &str) -> Result<u8, String> {
    a.get(k).and_then(Value::as_u64).map(|v| v.min(100) as u8).ok_or(format!("Falta «{k}»"))
}

fn call(name: &str, a: &Value) -> Result<Value, String> {
    let ok = |v: &str| Ok(json!(v));
    match name {
        "musica_que_suena" => crate::media::now_playing().map(|n| json!(n)),
        "musica_control" => crate::media::control(arg_str(a, "accion")?).and(ok("hecho")),
        "musica_ver_volumen" => crate::media::app_volume_get().map(|v| json!(v)),
        "musica_poner_volumen" => crate::media::app_volume_set(arg_u8(a, "nivel")?).map(|v| json!(v)),
        "abrir_spotify" => match crate::media::open_spotify() {
            Ok(()) => ok("abriendo Spotify"),
            Err(_) => call("abrir_web", &json!({"url": "https://open.spotify.com/"})),
        },
        "ver_volumen" => settings::audio::get().map(|(l, m)| json!({"nivel": l, "silencio": m})).map_err(|e| e.to_string()),
        "poner_volumen" => settings::audio::set_level(arg_u8(a, "nivel")?).map_err(|e| e.to_string()).and(ok("hecho")),
        "silenciar" => settings::audio::set_mute(a.get("silencio").and_then(Value::as_bool).unwrap_or(true)).map_err(|e| e.to_string()).and(ok("hecho")),
        "ver_brillo" => settings::brightness_get().map(|b| json!(b)),
        "poner_brillo" => settings::brightness_set(arg_u8(a, "nivel")?).and(ok("hecho")),
        "ver_bateria" => settings::get_battery().map(|b| json!(b)),
        "ver_modos_energia" => settings::power_plans().map(|p| json!(p)),
        "poner_modo_energia" => settings::plan_set(arg_str(a, "guid")?.to_string()).and(ok("hecho")),
        "ver_wifi" => settings::wifi_status().map(|w| json!(w)),
        "ver_hardware" => Ok(json!(hardware::get_hardware())),
        "listar_juegos" => Ok(json!(games::detect_all().iter().map(|g| json!({"id": g.id, "titulo": g.title, "tienda": g.store})).collect::<Vec<_>>())),
        "abrir_juego" => {
            let id = arg_str(a, "id")?;
            let all = games::detect_all();
            let g = all.iter().find(|g| g.id == id).ok_or("No hay ningún juego con ese id")?;
            games::launch(g).and(ok("abriendo"))
        }
        "listar_emuladores" => {
            let (_, list) = emulators::list_emulators();
            Ok(json!(list))
        }
        "instalar_emulador" => emulators::install(arg_str(a, "id")?).map(|v| json!({"version": v})),
        "abrir_emulador" => emulators::launch_emulator(arg_str(a, "id")?.into()).and(ok("abriendo")),
        "listar_carpeta" => files::list_dir(arg_str(a, "ruta")?.into()).map(|l| json!(l)),
        "lugares" => Ok(json!(files::list_places())),
        "abrir_archivo" => {
            let ruta = arg_str(a, "ruta")?;
            let ext = std::path::Path::new(ruta).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            const EXEC: [&str; 12] = ["exe", "bat", "cmd", "com", "ps1", "vbs", "js", "msi", "lnk", "scr", "reg", "hta"];
            if EXEC.contains(&ext.as_str()) {
                return Err("Por seguridad no abro programas ni scripts".into());
            }
            files::open_path(ruta.into()).and(ok("abriendo"))
        }
        "abrir_pantalla" => {
            let ruta = arg_str(a, "ruta")?.trim();
            if !claude::valid_route(ruta) {
                return Err(format!("Esa pantalla no existe. Usa una de: {}", claude::ROUTES.join(", ")));
            }
            claude::send_action("ruta", ruta).and(ok("pantalla abierta en la consola"))
        }
        "abrir_web" => claude::send_action("web", &browser::normalize(arg_str(a, "url")?)).and(ok("abriendo en el navegador de la consola")),
        "buscar_en_youtube" => claude::send_action("web", &youtube_url(arg_str(a, "texto")?)).and(ok("abriendo YouTube en el navegador de la consola")),
        "volver_al_inicio" => claude::send_action("inicio", "").and(ok("hecho")),
        "listar_aplicaciones" => {
            let (_, list) = games::detect_everything_from(games::detect_store());
            Ok(json!(list.iter().filter(|x| !blocked_app(&x.name, &x.app_id)).map(|x| json!({"id": x.id, "nombre": x.name})).collect::<Vec<_>>()))
        }
        "abrir_aplicacion" => {
            let id = arg_str(a, "id")?;
            let (all_games, list) = games::detect_everything_from(games::detect_store());
            if let Some(x) = list.iter().find(|x| x.id == id) {
                if blocked_app(&x.name, &x.app_id) {
                    return Err("Por seguridad no abro terminales ni herramientas del sistema".into());
                }
                let target = if x.app_id.is_empty() {
                    all_games.iter().find(|g| g.id == id).map(|g| g.launch.clone()).ok_or("No sé cómo abrir esa aplicación")?
                } else {
                    apps::launch_uri(&x.app_id)
                };
                return std::process::Command::new("explorer").arg(target).spawn().map_err(|e| e.to_string()).and(ok("abriendo"));
            }
            let g = all_games.iter().find(|g| g.id == id).ok_or("No hay ninguna aplicación con ese id")?;
            games::launch(g).and(ok("abriendo"))
        }
        _ => Err(format!("Herramienta desconocida: {name}")),
    }
}

/// Atiende una petición JSON-RPC. Devuelve None para notificaciones.
pub fn handle(req: &Value) -> Option<Value> {
    let id = req.get("id")?.clone();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "initialize" => json!({
            "protocolVersion": req.pointer("/params/protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "consola", "version": env!("CARGO_PKG_VERSION")}
        }),
        "ping" => json!({}),
        "tools/list" => json!({"tools": tools()}),
        "tools/call" => {
            let name = req.pointer("/params/name").and_then(Value::as_str).unwrap_or("");
            let args = req.pointer("/params/arguments").cloned().unwrap_or(json!({}));
            match call(name, &args) {
                Ok(v) => json!({"content": [{"type": "text", "text": v.to_string()}]}),
                Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
            }
        }
        _ => return Some(json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "Método no encontrado"}})),
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

/// Bucle principal del modo `--mcp`: una línea JSON por mensaje
pub fn serve_stdio() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(req) = serde_json::from_str::<Value>(line.trim_start_matches('\u{feff}')) else { continue };
        if let Some(resp) = handle(&req) {
            let _ = writeln!(out, "{resp}");
            let _ = out.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocolo_mcp_basico() {
        let init = handle(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}})).unwrap();
        assert_eq!(init["result"]["serverInfo"]["name"], "consola");
        assert!(handle(&json!({"jsonrpc":"2.0","method":"notifications/initialized"})).is_none());
        let list = handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).unwrap();
        assert!(list["result"]["tools"].as_array().unwrap().len() > 10);
        let vol = handle(&json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ver_volumen","arguments":{}}})).unwrap();
        assert!(vol["result"]["content"][0]["text"].as_str().unwrap().contains("nivel"));
        let bad = handle(&json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"abrir_archivo","arguments":{"ruta":"C:\\malo.exe"}}})).unwrap();
        assert_eq!(bad["result"]["isError"], true);
    }

    /// Cada herramienta tiene su nombre traducido para "Claude está usando…"
    #[test]
    fn herramientas_con_nombre_traducido() {
        for t in tools().as_array().unwrap() {
            let name = t["name"].as_str().unwrap();
            assert!(crate::i18n::lookup(&format!("tool.{name}")).is_some(), "falta tool.{name} en i18n.rs");
        }
    }

    #[test]
    fn herramientas_nuevas() {
        let list = handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).unwrap();
        let names: Vec<&str> = list["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        for n in ["abrir_pantalla", "abrir_web", "buscar_en_youtube", "listar_aplicaciones", "abrir_aplicacion", "volver_al_inicio"] {
            assert!(names.contains(&n), "falta {n}");
        }
        // Ninguna herramienta peligrosa
        assert!(!names.iter().any(|n| n.contains("borrar") || n.contains("apagar") || n.contains("ejecutar")));
        let bad = handle(&json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"abrir_pantalla","arguments":{"ruta":"/no-existe"}}})).unwrap();
        assert_eq!(bad["result"]["isError"], true);
        assert_eq!(youtube_url("  mario   kart 8 "), "https://www.youtube.com/results?search_query=mario+kart+8");
        assert_eq!(youtube_url("¿qué & más?"), "https://www.youtube.com/results?search_query=%C2%BFqu%C3%A9+%26+m%C3%A1s%3F");
        assert!(blocked_app("Windows PowerShell", ""));
        assert!(blocked_app("Terminal", "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App"));
        assert!(!blocked_app("Spotify", "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"));
    }
}
