use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    MonitorOnly,
    Controllable,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLevel {
    Detected,
    ProfileMatched,
    LabVerified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    pub operation: String,
    pub status: CapabilityStatus,
    pub adapter: String,
    pub evidence: EvidenceLevel,
    pub restoration_supported: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HardwareIdentity {
    pub manufacturer: String,
    pub family: Option<String>,
    pub model: String,
    pub sku: Option<String>,
    pub bios_vendor: Option<String>,
    pub bios_version: Option<String>,
    pub operating_system: String,
    pub os_build: Option<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SensorKind {
    Cpu,
    Gpu,
    Ambient,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SensorInfo {
    pub id: String,
    pub name: String,
    pub kind: SensorKind,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FanInfo {
    pub id: String,
    pub name: String,
    pub min_rpm: Option<u32>,
    pub max_rpm: Option<u32>,
    pub can_read_rpm: bool,
    pub can_set_speed: bool,
    pub can_restore_auto: bool,
    pub capability: Capability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SensorReading {
    pub id: String,
    pub temperature_c: Option<f32>,
    pub valid: bool,
    pub age_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FanReading {
    pub id: String,
    pub rpm: Option<u32>,
    pub mode: FanMode,
    pub valid: bool,
    pub age_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FanMode {
    Automatic,
    Maximum,
    Custom,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryFrame {
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub sequence: u64,
    pub sensors: Vec<SensorReading>,
    pub fans: Vec<FanReading>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryReport {
    pub identity: HardwareIdentity,
    pub adapter: String,
    pub fans: Vec<FanInfo>,
    pub sensors: Vec<SensorInfo>,
    pub capabilities: Vec<Capability>,
    pub diagnostics: BTreeMap<String, String>,
}

#[derive(Debug, Error)]
pub enum FanForgeError {
    #[error("unsupported hardware: {0}")]
    UnsupportedHardware(String),
    #[error("BIOS is not verified: {0}")]
    BiosNotVerified(String),
    #[error("telemetry is stale")]
    TelemetryStale,
    #[error("control lease is unavailable")]
    LeaseUnavailable,
    #[error("automatic-mode restoration failed: {0}")]
    RestoreFailed(String),
    #[error("adapter error: {0}")]
    Adapter(String),
}

impl FanForgeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedHardware(_) => "unsupported_hardware",
            Self::BiosNotVerified(_) => "bios_not_verified",
            Self::TelemetryStale => "telemetry_stale",
            Self::LeaseUnavailable => "lease_unavailable",
            Self::RestoreFailed(_) => "restore_failed",
            Self::Adapter(_) => "adapter_error",
        }
    }
}

#[async_trait]
pub trait DiscoveryAdapter: Send + Sync {
    async fn discover(&self) -> Result<DiscoveryReport, FanForgeError>;
}

#[async_trait]
pub trait TelemetryAdapter: Send + Sync {
    async fn read(&self) -> Result<TelemetryFrame, FanForgeError>;
}

#[async_trait]
pub trait ControlAdapter: Send + Sync {
    async fn supported_modes(&self, fan_id: &str) -> Result<Vec<FanMode>, FanForgeError>;
    async fn acquire_lease(&self, lease_id: &str) -> Result<(), FanForgeError>;
    async fn apply_mode(&self, fan_id: &str, mode: FanMode) -> Result<(), FanForgeError>;
    async fn verify_mode(&self, fan_id: &str, mode: FanMode) -> Result<bool, FanForgeError>;
    async fn restore_automatic(&self) -> Result<(), FanForgeError>;
}

pub struct MockAdapter {
    sequence: AtomicU64,
}

impl Default for MockAdapter {
    fn default() -> Self {
        Self {
            sequence: AtomicU64::new(0),
        }
    }
}

pub struct UnavailableTelemetryAdapter {
    sequence: AtomicU64,
}

impl Default for UnavailableTelemetryAdapter {
    fn default() -> Self {
        Self {
            sequence: AtomicU64::new(0),
        }
    }
}

#[async_trait]
impl TelemetryAdapter for UnavailableTelemetryAdapter {
    async fn read(&self) -> Result<TelemetryFrame, FanForgeError> {
        Ok(TelemetryFrame {
            event_type: "telemetry".into(),
            timestamp: Utc::now(),
            sequence: self.sequence.fetch_add(1, Ordering::Relaxed),
            sensors: vec![
                SensorReading {
                    id: "cpu-package".into(),
                    temperature_c: None,
                    valid: false,
                    age_ms: 0,
                },
                SensorReading {
                    id: "gpu-core".into(),
                    temperature_c: None,
                    valid: false,
                    age_ms: 0,
                },
            ],
            fans: vec![FanReading {
                id: "fan-interface".into(),
                rpm: None,
                mode: FanMode::Unknown,
                valid: false,
                age_ms: 0,
            }],
        })
    }
}

#[async_trait]
impl DiscoveryAdapter for MockAdapter {
    async fn discover(&self) -> Result<DiscoveryReport, FanForgeError> {
        let capability = Capability {
            operation: "fan.telemetry".into(),
            status: CapabilityStatus::MonitorOnly,
            adapter: "mock-v1".into(),
            evidence: EvidenceLevel::LabVerified,
            restoration_supported: false,
            reason: "Deterministic simulation; no hardware access".into(),
        };
        Ok(DiscoveryReport {
            identity: HardwareIdentity {
                manufacturer: "FanForge Labs".into(),
                family: Some("Nitro simulation".into()),
                model: "AN515-55 fixture".into(),
                sku: Some("MOCK-001".into()),
                bios_vendor: Some("Fixture".into()),
                bios_version: Some("0.0".into()),
                operating_system: std::env::consts::OS.into(),
                os_build: None,
                fingerprint: "mock-an515-55".into(),
            },
            adapter: "mock-v1".into(),
            fans: vec!["cpu-fan", "gpu-fan"]
                .into_iter()
                .map(|id| FanInfo {
                    id: id.into(),
                    name: if id == "cpu-fan" {
                        "CPU fan"
                    } else {
                        "GPU fan"
                    }
                    .into(),
                    min_rpm: Some(1200),
                    max_rpm: Some(6200),
                    can_read_rpm: true,
                    can_set_speed: false,
                    can_restore_auto: false,
                    capability: capability.clone(),
                })
                .collect(),
            sensors: vec![
                SensorInfo {
                    id: "cpu-package".into(),
                    name: "CPU package".into(),
                    kind: SensorKind::Cpu,
                    available: true,
                },
                SensorInfo {
                    id: "gpu-core".into(),
                    name: "GPU core".into(),
                    kind: SensorKind::Gpu,
                    available: true,
                },
            ],
            capabilities: vec![capability],
            diagnostics: BTreeMap::new(),
        })
    }
}

#[async_trait]
impl TelemetryAdapter for MockAdapter {
    async fn read(&self) -> Result<TelemetryFrame, FanForgeError> {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let phase = sequence as f32 / 8.0;
        let missing_gpu = sequence % 29 == 20;
        Ok(TelemetryFrame {
            event_type: "telemetry".into(),
            timestamp: Utc::now(),
            sequence,
            sensors: vec![
                SensorReading {
                    id: "cpu-package".into(),
                    temperature_c: Some(61.0 + phase.sin() * 8.0),
                    valid: true,
                    age_ms: 0,
                },
                SensorReading {
                    id: "gpu-core".into(),
                    temperature_c: if missing_gpu {
                        None
                    } else {
                        Some(55.0 + (phase * 0.7).cos() * 6.0)
                    },
                    valid: !missing_gpu,
                    age_ms: 0,
                },
            ],
            fans: vec![
                FanReading {
                    id: "cpu-fan".into(),
                    rpm: Some((2900.0 + phase.sin() * 650.0) as u32),
                    mode: FanMode::Automatic,
                    valid: true,
                    age_ms: 0,
                },
                FanReading {
                    id: "gpu-fan".into(),
                    rpm: if missing_gpu {
                        None
                    } else {
                        Some((2450.0 + (phase * 0.7).cos() * 500.0) as u32)
                    },
                    mode: FanMode::Automatic,
                    valid: !missing_gpu,
                    age_ms: 0,
                },
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn mock_never_advertises_control() {
        let report = MockAdapter::default().discover().await.unwrap();
        assert!(report.fans.iter().all(|fan| !fan.can_set_speed));
    }
    #[test]
    fn errors_have_stable_codes() {
        assert_eq!(FanForgeError::TelemetryStale.code(), "telemetry_stale");
    }
}
