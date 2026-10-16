use std::collections::BTreeMap;

use async_trait::async_trait;
use fanforge_core::{
    Capability, CapabilityStatus, DiscoveryAdapter, DiscoveryReport, EvidenceLevel, FanForgeError,
    FanInfo, HardwareIdentity, SensorInfo, SensorKind,
};
use sha2::{Digest, Sha256};

#[derive(Default)]
pub struct WindowsDiscoveryAdapter;

fn windows_product_name(build: Option<&str>) -> String {
    match build.and_then(|value| value.parse::<u32>().ok()) {
        Some(number) if number >= 22_000 => "Windows 11".into(),
        Some(_) => "Windows 10".into(),
        None => "Windows".into(),
    }
}

fn sanitize_sku(value: Option<String>) -> Option<String> {
    value.filter(|sku| sku.chars().any(|character| character != '0'))
}

#[cfg(windows)]
fn registry_string(
    root: windows::Win32::System::Registry::HKEY,
    path: &str,
    name: &str,
) -> Option<String> {
    use windows::{
        core::PCWSTR,
        Win32::System::Registry::{RegGetValueW, RRF_RT_REG_SZ},
    };
    let wide_path: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    let wide_name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let mut bytes = 0u32;
    unsafe {
        RegGetValueW(
            root,
            PCWSTR(wide_path.as_ptr()),
            PCWSTR(wide_name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut bytes),
        )
        .ok()
        .ok()?;
        if bytes < 2 {
            return None;
        }
        let mut value = vec![0u16; (bytes as usize).div_ceil(2)];
        RegGetValueW(
            root,
            PCWSTR(wide_path.as_ptr()),
            PCWSTR(wide_name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(value.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
        .ok()
        .ok()?;
        let end = value
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(value.len());
        Some(String::from_utf16_lossy(&value[..end]).trim().to_string()).filter(|v| !v.is_empty())
    }
}

#[cfg(windows)]
fn read_identity() -> HardwareIdentity {
    use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
    const BIOS: &str = r"HARDWARE\DESCRIPTION\System\BIOS";
    const NT: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let manufacturer = registry_string(HKEY_LOCAL_MACHINE, BIOS, "SystemManufacturer")
        .unwrap_or_else(|| "Unknown".into());
    let model = registry_string(HKEY_LOCAL_MACHINE, BIOS, "SystemProductName")
        .unwrap_or_else(|| "Unknown".into());
    let sku = sanitize_sku(registry_string(HKEY_LOCAL_MACHINE, BIOS, "SystemSKU"));
    let bios_vendor = registry_string(HKEY_LOCAL_MACHINE, BIOS, "BIOSVendor");
    let bios_version = registry_string(HKEY_LOCAL_MACHINE, BIOS, "BIOSVersion");
    let build = registry_string(HKEY_LOCAL_MACHINE, NT, "CurrentBuildNumber");
    let display_version = registry_string(HKEY_LOCAL_MACHINE, NT, "DisplayVersion");
    let mut hasher = Sha256::new();
    hasher.update(format!(
        "{manufacturer}|{model}|{}|{}",
        sku.as_deref().unwrap_or(""),
        bios_version.as_deref().unwrap_or("")
    ));
    let fingerprint = format!("{:x}", hasher.finalize())[..16].to_string();
    let operating_system = windows_product_name(build.as_deref());
    let family = (manufacturer.to_ascii_lowercase().contains("acer")
        && model.to_ascii_lowercase().contains("nitro"))
    .then(|| "Acer Nitro".into());
    HardwareIdentity {
        manufacturer,
        family,
        model,
        sku,
        bios_vendor,
        bios_version,
        operating_system,
        os_build: Some(match display_version {
            Some(v) => format!("{v} ({})", build.unwrap_or_default()),
            None => build.unwrap_or_default(),
        }),
        fingerprint,
    }
}

#[cfg(not(windows))]
fn read_identity() -> HardwareIdentity {
    HardwareIdentity {
        manufacturer: "Unknown".into(),
        family: None,
        model: "Non-Windows host".into(),
        sku: None,
        bios_vendor: None,
        bios_version: None,
        operating_system: std::env::consts::OS.into(),
        os_build: None,
        fingerprint: "non-windows".into(),
    }
}

#[async_trait]
impl DiscoveryAdapter for WindowsDiscoveryAdapter {
    async fn discover(&self) -> Result<DiscoveryReport, FanForgeError> {
        let identity = read_identity();
        let is_acer = identity.manufacturer.to_ascii_lowercase().contains("acer");
        let is_an515_55 = identity.model.to_ascii_uppercase().contains("AN515-55");
        let status = if is_an515_55 {
            CapabilityStatus::Unknown
        } else {
            CapabilityStatus::Unsupported
        };
        let reason = if is_an515_55 {
            "AN515-55 identity detected; no verified read-only fan provider is installed"
        } else if is_acer {
            "Acer hardware detected, but this exact model has no reviewed profile"
        } else {
            "The initial hardware profile is restricted to Acer Nitro 5 AN515-55"
        };
        let capability = Capability {
            operation: "fan.telemetry".into(),
            status,
            adapter: "windows-identity-v1".into(),
            evidence: EvidenceLevel::Detected,
            restoration_supported: false,
            reason: reason.into(),
        };
        let mut diagnostics = BTreeMap::new();
        diagnostics.insert("probeMode".into(), "read-only".into());
        diagnostics.insert("fanWrites".into(), "disabled".into());
        Ok(DiscoveryReport {
            identity,
            adapter: "windows-identity-v1".into(),
            fans: vec![FanInfo {
                id: "fan-interface".into(),
                name: "Firmware fan interface".into(),
                min_rpm: None,
                max_rpm: None,
                can_read_rpm: false,
                can_set_speed: false,
                can_restore_auto: false,
                capability: capability.clone(),
            }],
            sensors: vec![
                SensorInfo {
                    id: "cpu-package".into(),
                    name: "CPU package".into(),
                    kind: SensorKind::Cpu,
                    available: false,
                },
                SensorInfo {
                    id: "gpu-core".into(),
                    name: "GPU core".into(),
                    kind: SensorKind::Gpu,
                    available: false,
                },
            ],
            capabilities: vec![capability],
            diagnostics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{sanitize_sku, windows_product_name};

    #[test]
    fn recognizes_windows_11_from_build_number() {
        assert_eq!(windows_product_name(Some("26300")), "Windows 11");
        assert_eq!(windows_product_name(Some("19045")), "Windows 10");
        assert_eq!(windows_product_name(None), "Windows");
    }

    #[test]
    fn rejects_firmware_placeholder_skus() {
        assert_eq!(sanitize_sku(Some("0000000000000000".into())), None);
        assert_eq!(
            sanitize_sku(Some("NH.Q7KAA.001".into())),
            Some("NH.Q7KAA.001".into())
        );
    }
}
