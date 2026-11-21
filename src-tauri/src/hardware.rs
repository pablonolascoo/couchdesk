//! Lee el hardware del PC para decir si cada emulador irá bien.

use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Hardware {
    pub cpu: String,
    pub threads: u32,
    pub avx2: bool,
    pub ram_gb: f64,
    pub gpu: String,
    pub vram_gb: f64,
    /// 0 integrada · 1 básica · 2 media (GTX 1650) · 3 buena (RTX 3060) · 4 alta
    pub gpu_tier: u8,
    pub vulkan: bool,
}

/// Clasifica una gráfica por su nombre (aproximado, suficiente para "Bien / Justo / No").
pub fn gpu_tier(name: &str) -> u8 {
    let n = name.to_lowercase();
    let num = |prefix: &str| -> Option<u32> {
        let i = n.find(prefix)? + prefix.len();
        n[i..].trim_start().chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok()
    };
    if n.contains("nvidia") || n.contains("geforce") {
        if let Some(m) = num("rtx") {
            return match m {
                m if m >= 4070 || (m >= 3070 && m < 4000) => 4,
                m if m % 1000 >= 50 || m >= 4000 => 3,
                _ => 2,
            };
        }
        if let Some(m) = num("gtx") {
            return if m >= 1650 || (1060..1100).contains(&m) { 2 } else { 1 };
        }
        return 1; // MX, GT...
    }
    if n.contains("radeon") {
        if let Some(m) = num("rx") {
            return match m {
                m if m >= 6800 => 4,
                m if m >= 6600 => 3,
                m if m >= 570 => 2,
                _ => 1,
            };
        }
        return 0; // Radeon integrada (Vega, 780M...)
    }
    if n.contains("arc a7") || n.contains("arc b5") { 3 } else if n.contains("arc") { 2 } else { 0 }
}

fn cpu_name() -> String {
    let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    hklm.open_subkey("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0")
        .and_then(|k| k.get_value::<String, _>("ProcessorNameString"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "Procesador".into())
}

fn ram_gb() -> f64 {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut m = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    unsafe { GlobalMemoryStatusEx(&mut m) }.map(|_| m.ullTotalPhys as f64 / 1_073_741_824.0).unwrap_or(0.0)
}

/// La gráfica más potente instalada (en portátiles hay integrada + dedicada)
fn best_gpu() -> (String, f64) {
    let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    let Ok(class) = hklm.open_subkey("SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}") else {
        return ("Desconocida".into(), 0.0);
    };
    let mut best = ("Desconocida".to_string(), 0.0, 0u8);
    for sub in class.enum_keys().flatten() {
        let Ok(k) = class.open_subkey(&sub) else { continue };
        let Ok(name) = k.get_value::<String, _>("DriverDesc") else { continue };
        let vram = k.get_value::<u64, _>("HardwareInformation.qwMemorySize").unwrap_or(0) as f64 / 1_073_741_824.0;
        let tier = gpu_tier(&name);
        if (tier, vram) > (best.2, best.1) {
            best = (name, vram, tier);
        }
    }
    (best.0, best.1)
}

#[tauri::command(async)]
pub fn get_hardware() -> Hardware {
    let (gpu, vram_gb) = best_gpu();
    Hardware {
        cpu: cpu_name(),
        threads: std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1),
        avx2: std::arch::is_x86_feature_detected!("avx2"),
        ram_gb: (ram_gb() * 10.0).round() / 10.0,
        gpu_tier: gpu_tier(&gpu),
        vram_gb: (vram_gb * 10.0).round() / 10.0,
        gpu,
        vulkan: std::path::Path::new(r"C:\Windows\System32\vulkan-1.dll").exists(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clasifica_graficas() {
        assert_eq!(gpu_tier("NVIDIA GeForce GTX 1650 with Max-Q Design"), 2);
        assert_eq!(gpu_tier("NVIDIA GeForce RTX 3060 Laptop GPU"), 3);
        assert_eq!(gpu_tier("NVIDIA GeForce RTX 4080"), 4);
        assert_eq!(gpu_tier("NVIDIA GeForce MX350"), 1);
        assert_eq!(gpu_tier("Intel(R) UHD Graphics"), 0);
        assert_eq!(gpu_tier("AMD Radeon RX 6600"), 3);
        assert_eq!(gpu_tier("AMD Radeon(TM) Graphics"), 0);
    }

    #[test]
    fn hardware_real_de_este_pc() {
        let h = get_hardware();
        println!("{h:?}");
        assert!(h.threads >= 1 && h.ram_gb > 1.0);
    }
}
