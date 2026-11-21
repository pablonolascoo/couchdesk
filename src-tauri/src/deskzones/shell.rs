//! Los iconos del escritorio de Windows, con la API oficial del shell (nada de inyectar en explorer):
//! IShellWindows::FindWindowSW(escritorio) → IServiceProvider(SID_STopLevelBrowser) → IShellBrowser
//! → vista activa → IFolderView2 (leer iconos y posiciones, colocarlos, tamaño y opciones de la vista).
//! Todo aquí debe llamarse desde un hilo STA (CoInitializeEx APARTMENTTHREADED).

use super::classify::DeskItem;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, IPersistFile, IServiceProvider, CLSCTX_ALL, CLSCTX_INPROC_SERVER, STGM_READ};
use windows::Win32::System::Variant::{VARIANT, VT_I4};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::FindWindowExW;
use windows::core::{Interface, HSTRING, PWSTR, w};

pub struct View {
    fv: IFolderView2,
    /// Lista de iconos (SysListView32): sus coordenadas son las que usa IFolderView
    list: HWND,
}

/// Un icono del escritorio tal como está ahora
#[derive(Clone, Debug)]
pub struct Icon {
    pub item: DeskItem,
    /// Posición en coordenadas de pantalla
    pub x: i32,
    pub y: i32,
}

fn take(p: PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    let s = unsafe { p.to_string() }.unwrap_or_default();
    unsafe { CoTaskMemFree(Some(p.0 as _)) };
    s
}

fn name_of(item: &IShellItem, how: SIGDN) -> Option<String> {
    unsafe { item.GetDisplayName(how) }.ok().map(take).filter(|s| !s.is_empty())
}

/// Hijo de Windows que libera el pidl al soltarse
struct Pidl(*mut ITEMIDLIST);
impl Drop for Pidl {
    fn drop(&mut self) {
        unsafe { CoTaskMemFree(Some(self.0 as _)) };
    }
}

/// Destino de un acceso directo: .lnk (IShellLinkW) o .url (línea URL=)
pub fn shortcut_target(path: &str) -> Option<String> {
    let lower = path.to_lowercase();
    if lower.ends_with(".url") {
        let text = std::fs::read(path).ok()?;
        let text = String::from_utf8_lossy(&text);
        return text.lines().find_map(|l| l.trim().strip_prefix("URL=").map(|u| u.trim().to_string()));
    }
    if !lower.ends_with(".lnk") {
        return None;
    }
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        link.cast::<IPersistFile>().ok()?.Load(&HSTRING::from(path), STGM_READ).ok()?;
        let mut buf = [0u16; 1024];
        let mut fd = WIN32_FIND_DATAW::default();
        if link.GetPath(&mut buf, &mut fd, SLGP_RAWPATH.0 as u32).is_ok() {
            let n = buf.iter().position(|&c| c == 0).unwrap_or(0);
            if n > 0 {
                return Some(String::from_utf16_lossy(&buf[..n]));
            }
        }
        // Accesos a apps de la tienda o URIs: sin ruta, pero con argumentos
        let mut args = [0u16; 1024];
        link.GetArguments(&mut args).ok()?;
        let n = args.iter().position(|&c| c == 0).unwrap_or(0);
        (n > 0).then(|| String::from_utf16_lossy(&args[..n]))
    }
}

/// Completa una entrada de la lista con lo que sabe Windows del disco
pub fn describe(name: String, parsing: String, path: Option<String>) -> DeskItem {
    let is_dir = path.as_deref().is_some_and(|p| std::path::Path::new(p).is_dir());
    let target = path.as_deref().filter(|_| !is_dir).and_then(shortcut_target);
    DeskItem { name, parsing, path, is_dir, target }
}

impl View {
    /// La vista del escritorio de explorer, si está abierto
    pub fn open() -> Result<View, String> {
        unsafe {
            let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL).map_err(|e| format!("ShellWindows: {e}"))?;
            let mut loc = VARIANT::default();
            (*loc.Anonymous.Anonymous).vt = VT_I4;
            (*loc.Anonymous.Anonymous).Anonymous.lVal = CSIDL_DESKTOP as i32;
            let root = VARIANT::default();
            let mut hwnd = 0i32;
            let disp = windows
                .FindWindowSW(&loc, &root, SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH)
                .map_err(|e| format!("FindWindowSW: {e}"))?;
            let sp: IServiceProvider = disp.cast().map_err(|e| e.to_string())?;
            let browser: IShellBrowser = sp.QueryService(&SID_STopLevelBrowser).map_err(|e| format!("QueryService: {e}"))?;
            let view = browser.QueryActiveShellView().map_err(|e| format!("QueryActiveShellView: {e}"))?;
            let defview = view.GetWindow().map_err(|e| e.to_string())?;
            let list = FindWindowExW(Some(defview), None, w!("SysListView32"), None).unwrap_or(defview);
            let fv: IFolderView2 = view.cast().map_err(|e| format!("IFolderView2: {e}"))?;
            Ok(View { fv, list })
        }
    }

    fn to_screen(&self, p: POINT) -> (i32, i32) {
        let mut p = p;
        unsafe {
            let _ = ClientToScreen(self.list, &mut p);
        }
        (p.x, p.y)
    }

    fn to_client(&self, x: i32, y: i32) -> POINT {
        let mut p = POINT { x, y };
        unsafe {
            let _ = ScreenToClient(self.list, &mut p);
        }
        p
    }

    fn count(&self) -> i32 {
        unsafe { self.fv.ItemCount(SVGIO_ALLVIEW) }.unwrap_or(0)
    }

    /// Todos los iconos con su nombre, ruta, destino y posición (en pantalla)
    pub fn icons(&self) -> Vec<Icon> {
        let mut out = Vec::new();
        for i in 0..self.count() {
            unsafe {
                let Ok(item) = self.fv.GetItem::<IShellItem>(i) else { continue };
                let Ok(pidl) = self.fv.Item(i) else { continue };
                let pidl = Pidl(pidl);
                let pos = self.fv.GetItemPosition(pidl.0).unwrap_or_default();
                let parsing = name_of(&item, SIGDN_DESKTOPABSOLUTEPARSING).unwrap_or_default();
                let name = name_of(&item, SIGDN_NORMALDISPLAY).unwrap_or_else(|| parsing.clone());
                let path = name_of(&item, SIGDN_FILESYSPATH);
                let (x, y) = self.to_screen(pos);
                out.push(Icon { item: describe(name, parsing, path), x, y });
            }
        }
        out
    }

    /// Posición actual (en pantalla) de un icono por su nombre completo
    pub fn position_of(&self, parsing: &str) -> Option<(i32, i32)> {
        for i in 0..self.count() {
            unsafe {
                let Ok(item) = self.fv.GetItem::<IShellItem>(i) else { continue };
                if !name_of(&item, SIGDN_DESKTOPABSOLUTEPARSING).is_some_and(|p| p.eq_ignore_ascii_case(parsing)) {
                    continue;
                }
                let pidl = Pidl(self.fv.Item(i).ok()?);
                return self.fv.GetItemPosition(pidl.0).ok().map(|p| self.to_screen(p));
            }
        }
        None
    }

    /// Hueco de cada icono (incluye la separación), en píxeles
    pub fn spacing(&self) -> (i32, i32) {
        let mut p = POINT::default();
        unsafe {
            let _ = self.fv.GetSpacing(&mut p);
        }
        (p.x, p.y)
    }

    pub fn flags(&self) -> u32 {
        unsafe { self.fv.GetCurrentFolderFlags() }.unwrap_or(0)
    }

    pub fn set_flags(&self, mask: u32, value: u32) -> Result<(), String> {
        unsafe { self.fv.SetCurrentFolderFlags(mask, value) }.map_err(|e| e.to_string())
    }

    /// (modo de vista, tamaño de icono)
    pub fn view_mode(&self) -> (i32, i32) {
        let (mut mode, mut size) = (FOLDERVIEWMODE(0), 0i32);
        unsafe {
            let _ = self.fv.GetViewModeAndIconSize(&mut mode, &mut size);
        }
        (mode.0, size)
    }

    pub fn set_view_mode(&self, mode: i32, size: i32) -> Result<(), String> {
        unsafe { self.fv.SetViewModeAndIconSize(FOLDERVIEWMODE(mode), size) }.map_err(|e| e.to_string())
    }

    /// Coloca iconos (por su nombre completo) en posiciones de pantalla. Devuelve cuántos movió.
    pub fn place(&self, moves: &[(String, i32, i32)]) -> usize {
        let mut pidls = Vec::new();
        let mut pts = Vec::new();
        for i in 0..self.count() {
            unsafe {
                let Ok(item) = self.fv.GetItem::<IShellItem>(i) else { continue };
                let Some(parsing) = name_of(&item, SIGDN_DESKTOPABSOLUTEPARSING) else { continue };
                let Some((_, x, y)) = moves.iter().find(|(p, _, _)| p.eq_ignore_ascii_case(&parsing)) else { continue };
                let Ok(pidl) = self.fv.Item(i) else { continue };
                pidls.push(Pidl(pidl));
                pts.push(self.to_client(*x, *y));
            }
        }
        if pidls.is_empty() {
            return 0;
        }
        let raw: Vec<*const ITEMIDLIST> = pidls.iter().map(|p| p.0 as *const _).collect();
        let ok = unsafe { self.fv.SelectAndPositionItems(raw.len() as u32, raw.as_ptr(), Some(pts.as_ptr()), SVSI_POSITIONITEM.0 as u32) };
        if ok.is_err() {
            // Uno a uno por si alguno falla
            let mut n = 0;
            for (p, pt) in raw.iter().zip(&pts) {
                if unsafe { self.fv.SelectAndPositionItems(1, p, Some(pt), SVSI_POSITIONITEM.0 as u32) }.is_ok() {
                    n += 1;
                }
            }
            return n;
        }
        raw.len()
    }
}

/// Carpetas que forman el escritorio: la del usuario (aunque OneDrive la haya movido) y la pública
pub fn desktop_dirs() -> Vec<std::path::PathBuf> {
    [FOLDERID_Desktop, FOLDERID_PublicDesktop]
        .iter()
        .filter_map(|id| unsafe { SHGetKnownFolderPath(id, KNOWN_FOLDER_FLAG(0), None) }.ok().map(take))
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .collect()
}

/// Lo que hay en el escritorio leído del disco (para la vista previa sin el escritorio de Windows)
pub fn items_from_disk() -> Vec<DeskItem> {
    let mut out = Vec::new();
    for dir in desktop_dirs() {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let lower = name.to_lowercase();
            if lower == "desktop.ini" || lower.starts_with("~$") || lower.starts_with('.') {
                continue;
            }
            if let Ok(md) = e.metadata() {
                use std::os::windows::fs::MetadataExt;
                if md.file_attributes() & 0x6 != 0 {
                    continue; // oculto o de sistema: Windows tampoco lo enseña
                }
            }
            let path = e.path().display().to_string();
            // Windows esconde la extensión de los accesos directos
            let shown = if lower.ends_with(".lnk") || lower.ends_with(".url") { name.rsplit_once('.').map(|(s, _)| s.to_string()).unwrap_or(name) } else { name };
            out.push(describe(shown, path.clone(), Some(path)));
        }
    }
    out
}
