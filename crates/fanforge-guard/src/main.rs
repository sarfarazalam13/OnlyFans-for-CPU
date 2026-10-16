//! Privileged safety-boundary skeleton.
//!
//! The guard intentionally exposes no hardware adapter in milestone one. When a writable adapter
//! is validated, this process will own it and accept lease-bound commands over an ACL-restricted
//! Windows named pipe. Keeping the binary present now makes the process boundary reviewable without
//! creating a path to fan writes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum GuardRequest {
    Status,
    Heartbeat { lease_id: String },
    RestoreAutomatic { lease_id: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuardStatus {
    writes_enabled: bool,
    adapter_loaded: bool,
    reason: &'static str,
}

fn handle_request(request: GuardRequest) -> GuardStatus {
    let reason = match request {
        GuardRequest::Status => "No lab-verified AN515-55 profile is installed",
        GuardRequest::Heartbeat { lease_id } if lease_id.is_empty() => {
            "Invalid empty lease identifier"
        }
        GuardRequest::Heartbeat { lease_id: _ } => "Control leases are disabled",
        GuardRequest::RestoreAutomatic { lease_id } if lease_id.is_empty() => {
            "Invalid empty lease identifier"
        }
        GuardRequest::RestoreAutomatic { lease_id: _ } => {
            "No writable adapter is loaded; firmware ownership is retained"
        }
    };
    GuardStatus {
        writes_enabled: false,
        adapter_loaded: false,
        reason,
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let status = handle_request(GuardRequest::Status);
    tracing::info!(status = %serde_json::to_string(&status).unwrap(), "FanForge guard started fail-closed");
    tokio::signal::ctrl_c().await.ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_guard_request_remains_fail_closed() {
        for request in [
            GuardRequest::Status,
            GuardRequest::Heartbeat {
                lease_id: "test".into(),
            },
            GuardRequest::RestoreAutomatic {
                lease_id: "test".into(),
            },
        ] {
            let status = handle_request(request);
            assert!(!status.writes_enabled);
            assert!(!status.adapter_loaded);
        }
    }
}
