use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareProfile {
    pub schema_version: u32,
    pub id: String,
    pub manufacturer: String,
    pub models: Vec<String>,
    pub bios_allowlist: Vec<String>,
    pub adapter: String,
    pub monitoring_verified: bool,
    pub control_verified: bool,
    pub restoration_verified: bool,
    pub evidence: Vec<String>,
}

impl HardwareProfile {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1 || self.id.trim().is_empty() || self.models.is_empty() {
            return Err("invalid profile identity");
        }
        if self.control_verified && (!self.restoration_verified || self.bios_allowlist.is_empty()) {
            return Err("control requires a BIOS allowlist and verified restoration");
        }
        if self.adapter.contains("script") || self.adapter.contains("ec-write") {
            return Err("profiles cannot authorize executable scripts or arbitrary EC writes");
        }
        Ok(())
    }
}

pub struct AuditStore {
    connection: Connection,
}

impl AuditStore {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS safety_events (id INTEGER PRIMARY KEY, occurred_at TEXT NOT NULL, event_type TEXT NOT NULL, details TEXT NOT NULL);")?;
        Ok(Self { connection })
    }
    pub fn record(&self, event_type: &str, details: &str) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO safety_events (occurred_at, event_type, details) VALUES (?1, ?2, ?3)",
            params![Utc::now().to_rfc3339(), event_type, details],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unverified_profile_cannot_enable_control() {
        let profile = HardwareProfile {
            schema_version: 1,
            id: "acer.test".into(),
            manufacturer: "Acer".into(),
            models: vec!["AN515-55".into()],
            bios_allowlist: vec![],
            adapter: "acer-wmi".into(),
            monitoring_verified: false,
            control_verified: true,
            restoration_verified: false,
            evidence: vec![],
        };
        assert!(profile.validate().is_err());
    }
}
