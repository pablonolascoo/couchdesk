//! Lista de mandos conectados para Ajustes → Mandos.
//! · Xbox (y compatibles): huecos XInput 0-3, con batería.
//! · PlayStation (PS4 / PS5): buscados por HID (Sony), por cable o Bluetooth.
//! Solo se enumera: no se abre ningún mando (el proceso de fondo de psmando sigue a lo suyo).

use serde::Serialize;

/// Un mando conectado. JSON: { kind, name, slot, connection, battery, source }
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PadInfo {
    /// "xbox" | "ds4" | "ds5" | "ds5edge"
    pub kind: &'static str,
    /// Nombre para enseñar: "Mando de Xbox", "DualShock 4", "DualSense"…
    pub name: &'static str,
    /// Hueco XInput (0-3). Solo en mandos de Xbox.
    pub slot: Option<u32>,
    /// "usb" | "bluetooth" | "wireless" | "unknown"
    pub connection: &'static str,
    /// "empty" | "low" | "medium" | "full" | "wired" | "unknown"; null si no se sabe leer
    pub battery: Option<&'static str>,
    /// "xinput" | "hid"
    pub source: &'static str,
    /// Id del producto USB (0 en XInput)
    pub product_id: u16,
}

const SONY: u16 = 0x054C;

/// Qué mando de PlayStation es, por su id de producto
pub fn sony_kind(pid: u16) -> Option<(&'static str, &'static str)> {
    match pid {
        0x05C4 | 0x09CC => Some(("ds4", "DualShock 4")),
        0x0BA0 => Some(("ds4", crate::i18n::tr("pads.ds4_adapter"))),
        0x0CE6 => Some(("ds5", "DualSense")),
        0x0DF2 => Some(("ds5edge", "DualSense Edge")),
        _ => None,
    }
}

/// Batería de XInput (tipo + nivel) en texto
pub fn battery_label(kind: u8, level: u8) -> Option<&'static str> {
    match kind {
        0 => None,           // desconectado
        1 => Some("wired"),  // por cable: no tiene batería que mirar
        0xFF => Some("unknown"),
        _ => Some(match level {
            0 => "empty",
            1 => "low",
            2 => "medium",
            3 => "full",
            _ => "unknown",
        }),
    }
}

/// Conexión de un mando de Xbox según su batería: con cable = USB; si no, inalámbrico
pub fn xinput_connection(kind: u8) -> &'static str {
    if kind == 1 { "usb" } else { "wireless" }
}

fn bus_label(bus: hidapi::BusType) -> &'static str {
    match bus {
        hidapi::BusType::Usb => "usb",
        hidapi::BusType::Bluetooth => "bluetooth",
        _ => "unknown",
    }
}

fn xinput_pads() -> Vec<PadInfo> {
    use windows::Win32::UI::Input::XboxController::*;
    let mut out = Vec::new();
    for slot in 0..4u32 {
        let mut state = XINPUT_STATE::default();
        // 0 = ERROR_SUCCESS: hay un mando en este hueco
        if unsafe { XInputGetState(slot, &mut state) } != 0 {
            continue;
        }
        let mut bat = XINPUT_BATTERY_INFORMATION::default();
        let ok = unsafe { XInputGetBatteryInformation(slot, BATTERY_DEVTYPE_GAMEPAD, &mut bat) } == 0;
        let (kind, level) = if ok { (bat.BatteryType.0, bat.BatteryLevel.0) } else { (0xFF, 0) };
        out.push(PadInfo {
            kind: "xbox",
            name: crate::i18n::tr("pads.xbox"),
            slot: Some(slot),
            connection: if ok { xinput_connection(kind) } else { "unknown" },
            battery: battery_label(kind, level),
            source: "xinput",
            product_id: 0,
        });
    }
    out
}

fn sony_pads() -> Vec<PadInfo> {
    let Ok(api) = hidapi::HidApi::new() else { return Vec::new() };
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for d in api.device_list().filter(|d| d.vendor_id() == SONY) {
        let Some((kind, name)) = sony_kind(d.product_id()) else { continue };
        // Un mando puede salir varias veces (una por colección HID): solo cuenta la de "mando"
        if d.usage_page() != 0 && !(d.usage_page() == 1 && d.usage() == 5) {
            continue;
        }
        let key = match d.serial_number() {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => d.path().to_string_lossy().into_owned(),
        };
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let connection = if d.product_id() == 0x0BA0 { "wireless" } else { bus_label(d.bus_type()) };
        out.push(PadInfo {
            kind,
            name,
            slot: None,
            connection,
            battery: None,
            source: "hid",
            product_id: d.product_id(),
        });
    }
    out
}

/// Todos los mandos conectados ahora mismo
#[tauri::command(async)]
pub fn pads_list() -> Vec<PadInfo> {
    let mut list = xinput_pads();
    list.extend(sony_pads());
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombres_de_playstation() {
        assert_eq!(sony_kind(0x05C4), Some(("ds4", "DualShock 4")));
        assert_eq!(sony_kind(0x09CC).unwrap().0, "ds4");
        assert_eq!(sony_kind(0x0CE6), Some(("ds5", "DualSense")));
        assert_eq!(sony_kind(0x0DF2).unwrap().0, "ds5edge");
        assert_eq!(sony_kind(0x1234), None);
    }

    #[test]
    fn bateria_xinput() {
        assert_eq!(battery_label(0, 3), None);
        assert_eq!(battery_label(1, 0), Some("wired"));
        assert_eq!(battery_label(2, 0), Some("empty"));
        assert_eq!(battery_label(3, 1), Some("low"));
        assert_eq!(battery_label(2, 2), Some("medium"));
        assert_eq!(battery_label(3, 3), Some("full"));
        assert_eq!(battery_label(0xFF, 3), Some("unknown"));
        assert_eq!(battery_label(2, 9), Some("unknown"));
    }

    #[test]
    fn conexion_xinput() {
        assert_eq!(xinput_connection(1), "usb");
        assert_eq!(xinput_connection(2), "wireless");
        assert_eq!(xinput_connection(0xFF), "wireless");
    }

    #[test]
    fn json_en_camel_case() {
        let p = PadInfo { kind: "ds5", name: "DualSense", slot: None, connection: "bluetooth", battery: None, source: "hid", product_id: 0x0CE6 };
        let j = serde_json::to_string(&p).unwrap();
        assert!(j.contains("\"productId\":3302"));
        assert!(j.contains("\"slot\":null"));
    }
}
