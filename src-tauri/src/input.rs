//! Simula ratón y teclado de Windows (SendInput) para el mando móvil.

use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SetCursorPos, SM_CXSCREEN, SM_CYSCREEN};

fn send(inputs: &[INPUT]) {
    unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
}

fn mouse(flags: MOUSE_EVENT_FLAGS, data: i32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: data as u32, dwFlags: flags, time: 0, dwExtraInfo: 0 },
        },
    }
}

fn keybd(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    }
}

pub fn screen_size() -> (i32, i32) {
    unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
}

pub fn move_to(x: i32, y: i32) {
    unsafe {
        let _ = SetCursorPos(x, y);
    }
}

pub fn click(right: bool) {
    let (down, up) = if right {
        (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP)
    } else {
        (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP)
    };
    send(&[mouse(down, 0), mouse(up, 0)]);
}

/// `dy` positivo = rueda hacia abajo (contenido sube). 120 = un "clic" de rueda.
pub fn scroll(dx: i32, dy: i32) {
    if dy != 0 {
        send(&[mouse(MOUSEEVENTF_WHEEL, -dy)]);
    }
    if dx != 0 {
        send(&[mouse(MOUSEEVENTF_HWHEEL, dx)]);
    }
}

/// Escribe texto tal cual (acentos, ñ, emojis) con eventos Unicode
pub fn text(s: &str) {
    let mut inputs = Vec::new();
    for unit in s.encode_utf16() {
        inputs.push(keybd(VIRTUAL_KEY(0), unit, KEYEVENTF_UNICODE));
        inputs.push(keybd(VIRTUAL_KEY(0), unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
    }
    if !inputs.is_empty() {
        send(&inputs);
    }
}

/// Mueve el puntero relativo a donde está (como un ratón de verdad)
pub fn move_by(dx: i32, dy: i32) {
    if dx == 0 && dy == 0 {
        return;
    }
    let mut i = mouse(MOUSEEVENTF_MOVE, 0);
    i.Anonymous.mi.dx = dx;
    i.Anonymous.mi.dy = dy;
    send(&[i]);
}

/// Pulsar o soltar un botón del ratón (para arrastrar)
pub fn button(right: bool, down: bool) {
    let f = match (right, down) {
        (false, true) => MOUSEEVENTF_LEFTDOWN,
        (false, false) => MOUSEEVENTF_LEFTUP,
        (true, true) => MOUSEEVENTF_RIGHTDOWN,
        (true, false) => MOUSEEVENTF_RIGHTUP,
    };
    send(&[mouse(f, 0)]);
}

/// Atajo de teclado: pulsa las teclas en orden y las suelta al revés (p. ej. Alt + Izquierda)
pub fn combo(keys: &[VIRTUAL_KEY]) {
    let ext = |vk: VIRTUAL_KEY| {
        if [VK_UP, VK_DOWN, VK_LEFT, VK_RIGHT, VK_HOME, VK_LWIN].contains(&vk) { KEYEVENTF_EXTENDEDKEY } else { KEYBD_EVENT_FLAGS(0) }
    };
    let mut inputs: Vec<INPUT> = keys.iter().map(|&k| keybd(k, 0, ext(k))).collect();
    inputs.extend(keys.iter().rev().map(|&k| keybd(k, 0, ext(k) | KEYEVENTF_KEYUP)));
    send(&inputs);
}

pub fn key(name: &str) -> bool {
    let vk = match name {
        "enter" => VK_RETURN,
        "backspace" => VK_BACK,
        "esc" => VK_ESCAPE,
        "tab" => VK_TAB,
        "space" => VK_SPACE,
        "up" => VK_UP,
        "down" => VK_DOWN,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        "volup" => VK_VOLUME_UP,
        "voldown" => VK_VOLUME_DOWN,
        "home" => VK_HOME,
        _ => return false,
    };
    // Las flechas son teclas "extendidas"
    let ext = if matches!(name, "up" | "down" | "left" | "right" | "home") { KEYEVENTF_EXTENDEDKEY } else { KEYBD_EVENT_FLAGS(0) };
    send(&[keybd(vk, 0, ext), keybd(vk, 0, ext | KEYEVENTF_KEYUP)]);
    true
}
