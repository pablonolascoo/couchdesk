//! Navegador manejable con mando: una ventana web a pantalla completa (motor Chromium de
//! WebView2). El mando se lee de Windows (XInput), así funciona aunque la web tenga el foco.
//! Cruceta/stick izq.: saltar entre enlaces · stick der.: desplazar · A: abrir
//! B: atrás (o cerrar si no hay más) · Y: volver a la consola · Menú: abrir en el navegador de Windows

use crate::psmando;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use windows::Win32::UI::Input::XboxController::*;

const LABEL: &str = "navegador";
static RUNNING: AtomicBool = AtomicBool::new(false);

/// Se inyecta en cada página: resalta el elemento elegido y ofrece acciones al mando
const PAD_SCRIPT: &str = r#"
(() => {
  if (window.__pad) return;
  const SEL = 'a[href], button, input:not([type=hidden]), select, textarea, [role=button], [role=link], [tabindex]:not([tabindex="-1"]), summary';
  let cur = null;
  const style = document.createElement('style');
  style.textContent = '.__pad-focus{outline:3px solid #111 !important;outline-offset:3px !important;border-radius:4px}';
  const visible = (el) => { const r = el.getBoundingClientRect(); return r.width > 2 && r.height > 2 && r.bottom > 0 && r.top < innerHeight && getComputedStyle(el).visibility !== 'hidden'; };
  const set = (el) => {
    if (!el) return;
    (document.head || document.documentElement).appendChild(style);
    cur && cur.classList.remove('__pad-focus');
    cur = el; el.classList.add('__pad-focus');
    el.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'smooth' });
    el.focus({ preventScroll: true });
  };
  const center = (r) => [r.left + r.width / 2, r.top + r.height / 2];
  window.__pad = {
    move(dir) {
      const all = [...document.querySelectorAll(SEL)].filter(visible);
      if (!cur || !document.contains(cur) || !visible(cur)) return set(all[0]);
      const [ax, ay] = center(cur.getBoundingClientRect());
      let best = null, score = Infinity;
      for (const el of all) {
        if (el === cur) continue;
        const [bx, by] = center(el.getBoundingClientRect()), dx = bx - ax, dy = by - ay;
        const ok = (dir === 'right' && dx > 4) || (dir === 'left' && dx < -4) || (dir === 'down' && dy > 4) || (dir === 'up' && dy < -4);
        if (!ok) continue;
        const h = dir === 'left' || dir === 'right';
        const s = (h ? Math.abs(dx) : Math.abs(dy)) + (h ? Math.abs(dy) : Math.abs(dx)) * 2;
        if (s < score) { score = s; best = el; }
      }
      if (best) set(best); else window.scrollBy({ top: dir === 'down' ? 300 : dir === 'up' ? -300 : 0, behavior: 'smooth' });
    },
    click() { if (cur && document.contains(cur)) { cur.click(); if (/^(INPUT|TEXTAREA)$/.test(cur.tagName)) cur.focus(); } },
    scroll(dx, dy) { window.scrollBy(dx, dy); },
    back() { if (history.length > 1) { history.back(); return true; } return false; },
  };
})();
"#;

/// Barra de arriba: siempre hay una salida, también con ratón o teclado (navegador y nube).
/// Para hablar con la consola la página pone un instante un título especial (no se ve: la ventana
/// no tiene barra de título) y lo devuelve; así no se toca la dirección ni el historial de la web.
const BAR_SCRIPT: &str = r#"
(() => {
  if (window.__consolaBar) return;
  window.__consolaBar = true;
  const signal = (what) => {
    const prev = document.title;
    document.title = '__consola:' + what;
    setTimeout(() => { if (document.title.startsWith('__consola:')) document.title = prev; }, 80);
  };
  // Al escribir en una web, el móvil conectado como mando abre su teclado
  const editable = (el) => !!el && (el.isContentEditable ||
    (/^(INPUT|TEXTAREA)$/.test(el.tagName) && !/^(button|checkbox|radio|range|submit|reset|file|color|image|hidden)$/i.test(el.type || '')));
  addEventListener('focusin', (e) => { if (editable(e.composedPath()[0])) signal('kbd:1'); }, true);
  addEventListener('focusout', (e) => { if (editable(e.composedPath()[0])) signal('kbd:0'); }, true);
  const bar = () => {
    if (document.getElementById('__consola_bar')) return;
    const host = document.createElement('div');
    host.id = '__consola_bar';
    host.style.cssText = 'position:fixed;top:14px;left:50%;transform:translateX(-50%);z-index:2147483647;transition:opacity .35s cubic-bezier(.32,.72,0,1),translate .35s cubic-bezier(.32,.72,0,1)';
    const sh = host.attachShadow({ mode: 'open' });
    sh.innerHTML = `<style>
      .b{display:flex;gap:4px;align-items:center;padding:6px;border-radius:999px;background:rgba(251,248,242,.94);backdrop-filter:blur(16px);box-shadow:0 0 0 1px rgba(40,30,20,.12),0 18px 44px -18px rgba(40,30,20,.5);font:500 13px/1 "Segoe UI",system-ui,sans-serif;color:#1d1813}
      button{all:unset;cursor:pointer;padding:10px 15px;border-radius:999px;white-space:nowrap;transition:background .2s,color .2s}
      button:hover,button:focus-visible{background:#1d1813;color:#fbf8f2}
      .x{background:#1d1813;color:#fbf8f2}
      .u{max-width:30vw;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;opacity:.55;padding:0 10px}
      .h{opacity:.5;font-size:12px;padding:0 12px 0 6px;white-space:nowrap}
    </style><div class="b"><button data-a="back">← __T_BACK__</button><button data-a="reload">⟳</button><span class="u"></span><button data-a="fuera">__T_OUT__</button><button class="x" data-a="cerrar">✕ __T_HOME__</button><span class="h">__T_ESC__</span></div>`;
    sh.querySelector('.u').textContent = location.hostname;
    sh.addEventListener('click', (e) => {
      const a = e.composedPath().find((n) => n.dataset && n.dataset.a)?.dataset.a;
      if (a === 'back') { if (history.length > 1) history.back(); else signal('cerrar'); }
      else if (a === 'reload') location.reload();
      else if (a) signal(a);
    });
    document.documentElement.appendChild(host);
    let t = 0;
    const hide = () => { host.style.opacity = '0'; host.style.translate = '0 -12px'; host.style.pointerEvents = 'none'; };
    const show = () => { host.style.opacity = '1'; host.style.translate = '0 0'; host.style.pointerEvents = 'auto'; clearTimeout(t); t = setTimeout(hide, 3500); };
    addEventListener('mousemove', (e) => { if (e.clientY < 90) show(); }, { passive: true });
    // Esc enseña la barra (mantener Esc o pulsarlo 3 veces lo vigila la consola: ninguna web lo bloquea)
    addEventListener('keydown', (e) => { if (e.key === 'Escape') show(); }, true);
    if (window.__pad) window.__pad.bar = show;
    show();
  };
  if (document.readyState === 'loading') addEventListener('DOMContentLoaded', bar); else bar();
})();
"#;

/// BAR_SCRIPT con los textos en el idioma de la consola (se monta al abrir la ventana)
fn bar_script() -> String {
    use crate::i18n::tr;
    // Los textos van dentro de un `...` de JavaScript: sin comillas invertidas ni ${
    let safe = |k: &'static str| tr(k).replace('`', "'").replace("${", "$ {").replace('<', "&lt;");
    BAR_SCRIPT
        .replace("__T_BACK__", &safe("browser.back"))
        .replace("__T_OUT__", &safe("browser.open_windows"))
        .replace("__T_HOME__", &safe("browser.home"))
        .replace("__T_ESC__", &safe("browser.hold_esc"))
}

fn open_in_system_browser(url: &str) {
    let _ = std::process::Command::new("explorer").arg(url).spawn();
}

/// Mando de PlayStation traducido a botones de Xbox (✕=A, ○=B, □=X, △=Y, OPTIONS=Menú)
fn ps_as_xinput(s: &psmando::State) -> XINPUT_GAMEPAD {
    use psmando::btn::*;
    let map = [
        (CROSS, XINPUT_GAMEPAD_A), (CIRCLE, XINPUT_GAMEPAD_B), (SQUARE, XINPUT_GAMEPAD_X), (TRIANGLE, XINPUT_GAMEPAD_Y),
        (OPTIONS, XINPUT_GAMEPAD_START), (SHARE, XINPUT_GAMEPAD_BACK), (L1, XINPUT_GAMEPAD_LEFT_SHOULDER),
        (R1, XINPUT_GAMEPAD_RIGHT_SHOULDER), (UP, XINPUT_GAMEPAD_DPAD_UP), (DOWN, XINPUT_GAMEPAD_DPAD_DOWN),
        (LEFT, XINPUT_GAMEPAD_DPAD_LEFT), (RIGHT, XINPUT_GAMEPAD_DPAD_RIGHT),
    ];
    let b = map.iter().filter(|(p, _)| s.has(*p)).fold(0u16, |a, (_, x)| a | x.0);
    // 0..255 (arriba = 0) → −32768..32767 (arriba = positivo)
    let axis = |v: u8| ((i32::from(v) - 128) * 256).clamp(-32767, 32767) as i16;
    XINPUT_GAMEPAD {
        wButtons: XINPUT_GAMEPAD_BUTTON_FLAGS(b),
        sThumbLX: axis(s.lx),
        sThumbLY: -axis(s.ly),
        sThumbRX: axis(s.rx),
        sThumbRY: -axis(s.ry),
        ..Default::default()
    }
}

/// Lee el mando que haya: Xbox (XInput) o, si no, PlayStation (HID, se busca cada 2 s)
#[derive(Default)]
struct PadReader {
    ps: Option<psmando::Pad>,
    api: Option<hidapi::HidApi>,
    last_search: Option<Instant>,
}

impl PadReader {
    fn read(&mut self) -> Option<XINPUT_GAMEPAD> {
        let mut state = XINPUT_STATE::default();
        if (0..4).any(|i| unsafe { XInputGetState(i, &mut state) } == 0) {
            return Some(state.Gamepad);
        }
        if self.ps.is_none() && self.last_search.is_none_or(|t| t.elapsed() > Duration::from_secs(2)) {
            self.last_search = Some(Instant::now());
            match self.api.as_mut() {
                Some(a) => { let _ = a.refresh_devices(); }
                None => self.api = hidapi::HidApi::new().ok(),
            }
            self.ps = self.api.as_ref().and_then(psmando::Pad::open);
        }
        match self.ps.as_mut()?.poll(0) {
            Some(s) => Some(ps_as_xinput(&s)),
            None => { self.ps = None; None }
        }
    }
}

/// Señales de la página (título especial puesto un instante por BAR_SCRIPT)
fn on_title(win: tauri::WebviewWindow, title: String) {
    let Some(what) = title.strip_prefix("__consola:") else { return };
    match what {
        "cerrar" => go_home(win.app_handle()),
        "fuera" => {
            if let Ok(url) = win.url() {
                open_in_system_browser(url.as_str());
            }
            go_home(win.app_handle());
        }
        "kbd:1" => crate::remote::set_mode(win.app_handle(), None, Some(true)),
        "kbd:0" => crate::remote::set_mode(win.app_handle(), None, Some(false)),
        _ => {}
    }
}

/// Volver al Inicio de la consola desde donde sea: cierra navegador y nube, trae la consola
/// al frente y pone los móviles conectados en modo mando.
pub fn go_home(app: &tauri::AppHandle) {
    use tauri::Emitter;
    for label in [LABEL, CLOUD] {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.close();
        }
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
        let _ = main.emit("go-home", ());
    }
    crate::remote::set_mode(app, Some("gp"), Some(false));
}

/// Bucle del mando mientras el navegador está abierto (Xbox o PlayStation)
fn pad_loop(app: tauri::AppHandle) {
    let mut prev = 0u16;
    let mut held_since = Instant::now();
    let mut last_repeat = Instant::now();
    let mut reader = PadReader::default();
    while RUNNING.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(16));
        let Some(win) = app.get_webview_window(LABEL) else { break };
        let Some(g) = reader.read() else { continue };
        let mut b = g.wButtons.0;
        // Stick izquierdo como cruceta
        const DEAD: i16 = 16000;
        if g.sThumbLY > DEAD { b |= XINPUT_GAMEPAD_DPAD_UP.0 }
        if g.sThumbLY < -DEAD { b |= XINPUT_GAMEPAD_DPAD_DOWN.0 }
        if g.sThumbLX < -DEAD { b |= XINPUT_GAMEPAD_DPAD_LEFT.0 }
        if g.sThumbLX > DEAD { b |= XINPUT_GAMEPAD_DPAD_RIGHT.0 }
        let pressed = b & !prev;
        if b != prev {
            held_since = Instant::now();
        }
        prev = b;

        let dirs = [
            (XINPUT_GAMEPAD_DPAD_UP.0, "up"), (XINPUT_GAMEPAD_DPAD_DOWN.0, "down"),
            (XINPUT_GAMEPAD_DPAD_LEFT.0, "left"), (XINPUT_GAMEPAD_DPAD_RIGHT.0, "right"),
        ];
        // Repetición al mantener la dirección
        let repeat = held_since.elapsed() > Duration::from_millis(350) && last_repeat.elapsed() > Duration::from_millis(110);
        for (mask, dir) in dirs {
            if pressed & mask != 0 || (repeat && b & mask != 0) {
                let _ = win.eval(format!("window.__pad && __pad.move('{dir}')"));
                last_repeat = Instant::now();
            }
        }
        if pressed & XINPUT_GAMEPAD_A.0 != 0 {
            let _ = win.eval("window.__pad && __pad.click()");
        }
        if pressed & XINPUT_GAMEPAD_B.0 != 0 {
            // Atrás; si no hay página anterior, cerrar el navegador
            let _ = win.eval("window.__pad && !__pad.back() && (window.location.href = 'about:blank#__consola_cerrar')");
        }
        if pressed & XINPUT_GAMEPAD_Y.0 != 0 {
            let _ = win.close();
            break;
        }
        if pressed & XINPUT_GAMEPAD_START.0 != 0 {
            if let Ok(url) = win.url() {
                open_in_system_browser(url.as_str());
            }
        }
        // Stick derecho: desplazar la página
        let (rx, ry) = (g.sThumbRX, g.sThumbRY);
        if rx.unsigned_abs() > 8000 || ry.unsigned_abs() > 8000 {
            let sx = i32::from(rx) / 1500;
            let sy = -i32::from(ry) / 1500;
            let _ = win.eval(format!("window.__pad && __pad.scroll({sx},{sy})"));
        }
    }
    RUNNING.store(false, Ordering::Relaxed);
    // Volver a la consola (y los móviles, a modo mando)
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
    }
    crate::remote::set_mode(&app, Some("gp"), Some(false));
}

/// Solo http(s); lo demás se trata como búsqueda
pub fn normalize(input: &str) -> String {
    let t = input.trim();
    if t.starts_with("https://") || t.starts_with("http://") {
        t.to_string()
    } else if t.contains('.') && !t.contains(' ') {
        format!("https://{t}")
    } else {
        format!("https://duckduckgo.com/?q={}", t.split_whitespace().collect::<Vec<_>>().join("+"))
    }
}

/// `async` a propósito: crear una ventana desde una orden normal bloquea la app en Windows
#[tauri::command]
pub async fn browser_open(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let url = normalize(&url);
    let parsed: tauri::Url = url.parse().map_err(|_| crate::i18n::tr("browser.bad_url"))?;
    if let Some(w) = app.get_webview_window(LABEL) {
        w.navigate(parsed).map_err(|e| e.to_string())?;
        let _ = w.set_focus();
    } else {
        WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::External(parsed))
            .title("Navegador")
            .fullscreen(true)
            .decorations(false)
            .initialization_script(PAD_SCRIPT)
            .initialization_script(bar_script())
            .on_document_title_changed(on_title)
            .build()
            .map_err(|e| e.to_string())?;
    }
    // En una web se usa sobre todo el puntero: los móviles pasan a touchpad
    crate::remote::set_mode(&app, Some("mouse"), Some(false));
    if !RUNNING.swap(true, Ordering::Relaxed) {
        std::thread::spawn(move || pad_loop(app));
    }
    Ok(())
}

// ---------------- Xbox Cloud Gaming ----------------

const CLOUD: &str = "nube";
/// Xbox Cloud Gaming en el idioma de la consola
fn cloud_url() -> String {
    let region = match crate::i18n::current() {
        crate::i18n::Lang::Es => "es-ES",
        crate::i18n::Lang::En => "en-US",
        crate::i18n::Lang::PtBr => "pt-BR",
        crate::i18n::Lang::Fr => "fr-FR",
        crate::i18n::Lang::De => "de-DE",
        crate::i18n::Lang::It => "it-IT",
    };
    format!("https://www.xbox.com/{region}/play")
}
static CLOUD_RUNNING: AtomicBool = AtomicBool::new(false);

/// En la nube el mando es del juego: la consola solo vigila la combinación para salir
/// (LB + RB + View, o L1 + R1 + SHARE, mantenidos 1,5 s) y la barra de arriba.
fn cloud_loop(app: tauri::AppHandle) {
    let mut reader = PadReader::default();
    let mut held: Option<Instant> = None;
    let exit = XINPUT_GAMEPAD_LEFT_SHOULDER.0 | XINPUT_GAMEPAD_RIGHT_SHOULDER.0 | XINPUT_GAMEPAD_BACK.0;
    while CLOUD_RUNNING.load(Ordering::Relaxed) {
        // 30 veces por segundo basta para vigilar; el juego lee el mando por su cuenta
        std::thread::sleep(Duration::from_millis(33));
        if app.get_webview_window(CLOUD).is_none() {
            break;
        }
        let on = reader.read().is_some_and(|g| g.wButtons.0 & exit == exit);
        match (on, held) {
            (false, _) => held = None,
            (true, None) => held = Some(Instant::now()),
            (true, Some(t)) if t.elapsed() > Duration::from_millis(1500) => {
                go_home(&app);
                break;
            }
            _ => {}
        }
    }
    CLOUD_RUNNING.store(false, Ordering::Relaxed);
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
    }
}

/// Abre Xbox Cloud Gaming a pantalla completa. La sesión de Microsoft se guarda: solo
/// hay que iniciarla la primera vez.
#[tauri::command]
pub async fn cloud_open(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(CLOUD) {
        let _ = w.set_focus();
    } else {
        WebviewWindowBuilder::new(&app, CLOUD, WebviewUrl::External(cloud_url().parse().unwrap()))
            .title("Xbox Cloud Gaming")
            .fullscreen(true)
            .decorations(false)
            .background_color(tauri::window::Color(0, 0, 0, 255))
            .initialization_script(bar_script())
            .on_document_title_changed(on_title)
            .build()
            .map_err(|e| e.to_string())?;
    }
    // En la nube se juega: los móviles pasan a mando
    crate::remote::set_mode(&app, Some("gp"), Some(false));
    if !CLOUD_RUNNING.swap(true, Ordering::Relaxed) {
        std::thread::spawn(move || cloud_loop(app));
    }
    Ok(())
}

#[tauri::command(async)]
pub fn browser_in_system(url: String) {
    open_in_system_browser(&normalize(&url));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direcciones_y_busquedas() {
        assert_eq!(normalize("youtube.com"), "https://youtube.com");
        assert_eq!(normalize("https://twitch.tv"), "https://twitch.tv");
        assert_eq!(normalize("trucos mario 64"), "https://duckduckgo.com/?q=trucos+mario+64");
        assert!(normalize("javascript:alert(1)").starts_with("https://duckduckgo.com"));
    }
}
