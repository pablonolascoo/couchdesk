//! Vista previa de webs para los accesos directos del navegador: el icono más grande que ofrece
//! cada web (apple-touch-icon, icon…) y su imagen de compartir (og:image) para el fondo.
//! Se guardan en %LOCALAPPDATA%\Consola\webs y se renuevan como mucho una vez por semana.

use serde::Serialize;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Serialize, Clone, Debug, Default)]
pub struct Preview {
    pub url: String,
    /// Imagen grande de fondo (ruta local)
    pub image: Option<String>,
    /// Icono grande de la web (ruta local)
    pub icon: Option<String>,
}

fn dir() -> PathBuf {
    crate::paths::data_dir().join("webs")
}

/// Atributos de una etiqueta HTML: `<meta property="og:image" content="...">`
fn attrs(tag: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = tag;
    while let Some(eq) = rest.find('=') {
        let name = rest[..eq].rsplit(|c: char| c.is_whitespace() || c == '<').next().unwrap_or("").to_lowercase();
        let after = rest[eq + 1..].trim_start();
        let (value, next) = match after.chars().next() {
            Some(q @ ('"' | '\'')) => match after[1..].find(q) {
                Some(end) => (&after[1..1 + end], &after[2 + end..]),
                None => break,
            },
            _ => {
                let end = after.find(|c: char| c.is_whitespace() || c == '>').unwrap_or(after.len());
                (&after[..end], &after[end..])
            }
        };
        out.push((name, value.replace("&amp;", "&")));
        rest = next;
    }
    out
}

fn tags<'a>(html: &'a str, name: &str) -> Vec<&'a str> {
    let lower = html.to_lowercase();
    let open = format!("<{name}");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = lower[from..].find(&open) {
        let start = from + i;
        let Some(end) = lower[start..].find('>') else { break };
        out.push(&html[start..start + end + 1]);
        from = start + end + 1;
    }
    out
}

/// Busca la imagen de compartir y el icono más grande en el HTML de una web
pub fn parse(html: &str, base: &reqwest::Url) -> (Option<String>, Option<String>) {
    let get = |a: &[(String, String)], k: &str| a.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    let abs = |u: String| base.join(&u).ok().map(|u| u.to_string());
    let mut image = None;
    for t in tags(html, "meta") {
        let a = attrs(t);
        let key = get(&a, "property").or_else(|| get(&a, "name")).unwrap_or_default().to_lowercase();
        if (key == "og:image" || key == "twitter:image") && image.is_none() {
            image = get(&a, "content").and_then(abs);
        }
    }
    // El icono más grande: apple-touch-icon (180 px) o el "icon" con más tamaño
    let mut best: Option<(u32, String)> = None;
    for t in tags(html, "link") {
        let a = attrs(t);
        let rel = get(&a, "rel").unwrap_or_default().to_lowercase();
        if !rel.split_whitespace().any(|r| r == "icon" || r == "apple-touch-icon" || r == "apple-touch-icon-precomposed") {
            continue;
        }
        let Some(href) = get(&a, "href").and_then(abs) else { continue };
        let size = get(&a, "sizes")
            .and_then(|s| s.split('x').next().and_then(|n| n.trim().parse().ok()))
            .unwrap_or(if rel.contains("apple") { 180 } else if href.ends_with(".svg") { 512 } else { 32 });
        if best.as_ref().is_none_or(|(b, _)| size > *b) {
            best = Some((size, href));
        }
    }
    (image, best.map(|(_, h)| h))
}

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36 Edg/140.0")
        .build()
        .expect("cliente http")
}

/// Descarga una imagen (máx. 4 MB) a la carpeta de webs; devuelve su ruta
fn download(http: &reqwest::blocking::Client, url: &str, name: &str) -> Option<String> {
    let r = http.get(url).send().ok()?.error_for_status().ok()?;
    let kind = r.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_lowercase();
    let ext = if kind.contains("svg") { "svg" } else if kind.contains("png") { "png" } else if kind.contains("webp") { "webp" }
        else if kind.contains("icon") { "ico" } else if kind.contains("jpeg") || kind.contains("jpg") { "jpg" } else { return None };
    let bytes = r.bytes().ok()?;
    if bytes.len() > 4_000_000 || bytes.len() < 100 {
        return None;
    }
    let path = dir().join(format!("{name}.{ext}"));
    std::fs::write(&path, &bytes).ok()?;
    Some(path.to_string_lossy().into_owned())
}

/// Ya guardado y con menos de una semana
fn cached(prefix: &str) -> Option<String> {
    let entries = std::fs::read_dir(dir()).ok()?;
    entries.flatten().map(|e| e.path()).find(|p| {
        p.file_stem().is_some_and(|s| s.to_string_lossy() == prefix)
            && p.metadata().and_then(|m| m.modified()).ok()
                .and_then(|t| SystemTime::now().duration_since(t).ok())
                .is_some_and(|age| age < Duration::from_secs(7 * 86_400))
    }).map(|p| p.to_string_lossy().into_owned())
}

pub fn preview(url: &str) -> Preview {
    let id = crate::apps::stable_id(url);
    let (og, ic) = (format!("{id}-og"), format!("{id}-icon"));
    let mut p = Preview { url: url.into(), image: cached(&og), icon: cached(&ic) };
    // Marca de "ya mirado": evita volver a pedir la web cada vez si no tiene imagen
    let checked = dir().join(format!("{id}.checked"));
    let fresh = checked.metadata().and_then(|m| m.modified()).ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < Duration::from_secs(7 * 86_400));
    if fresh || (p.image.is_some() && p.icon.is_some()) {
        return p;
    }
    let _ = std::fs::create_dir_all(dir());
    let http = client();
    if let Ok(r) = http.get(url).send() {
        // La dirección final (tras redirecciones) es la base de las rutas relativas
        let page = r.url().clone();
        if let Ok(html) = r.text() {
            // Basta con el principio de la página (la cabecera)
            let (image, icon) = parse(&html[..html.floor_char_boundary(600_000)], &page);
            if p.image.is_none() {
                p.image = image.and_then(|u| download(&http, &u, &og));
            }
            if p.icon.is_none() {
                p.icon = icon.and_then(|u| download(&http, &u, &ic));
            }
        }
    }
    let _ = std::fs::write(&checked, b"");
    p
}

/// Vistas previas de varias webs a la vez (en paralelo)
#[tauri::command(async)]
pub fn site_previews(urls: Vec<String>) -> Vec<Preview> {
    let handles: Vec<_> = urls.into_iter().take(24).map(|u| std::thread::spawn(move || preview(&u))).collect();
    handles.into_iter().filter_map(|h| h.join().ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_imagen_e_icono_mas_grande() {
        let html = r#"<html><head>
          <link rel="icon" href="/favicon.ico">
          <link rel="icon" type="image/png" sizes="96x96" href="/i96.png">
          <link rel="apple-touch-icon" href="https://cdn.ejemplo.com/apple.png">
          <meta property='og:image' content='/og.jpg?a=1&amp;b=2'>
        </head></html>"#;
        let base = reqwest::Url::parse("https://www.ejemplo.com/tv").unwrap();
        let (image, icon) = parse(html, &base);
        assert_eq!(image.as_deref(), Some("https://www.ejemplo.com/og.jpg?a=1&b=2"));
        assert_eq!(icon.as_deref(), Some("https://cdn.ejemplo.com/apple.png"), "el de 180 px gana al de 96");
    }

    #[test]
    #[ignore]
    fn webs_de_verdad() {
        for u in ["https://www.netflix.com/es/", "https://www.primevideo.com/", "https://www.youtube.com/"] {
            println!("{:?}", preview(u));
        }
    }
}
