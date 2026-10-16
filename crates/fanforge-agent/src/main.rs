use std::{net::SocketAddr, sync::Arc, time::Duration};

use fanforge_api::{spawn_telemetry, AppState};
use fanforge_core::{DiscoveryAdapter, MockAdapter, TelemetryAdapter, UnavailableTelemetryAdapter};
use fanforge_safety::SafetyState;
use fanforge_windows::WindowsDiscoveryAdapter;
use tokio::sync::{broadcast, RwLock};

fn validate_bind(value: &str) -> anyhow::Result<SocketAddr> {
    let address: SocketAddr = value.parse()?;
    anyhow::ensure!(
        address.ip().is_loopback(),
        "FANFORGE_BIND must use a loopback address"
    );
    Ok(address)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "fanforge=info,tower_http=info".into()),
        )
        .init();

    let mock_mode = std::env::var("FANFORGE_ADAPTER")
        .map(|value| value.eq_ignore_ascii_case("mock"))
        .unwrap_or(!cfg!(windows));
    let (report, telemetry): (_, Arc<dyn TelemetryAdapter>) = if mock_mode {
        let discovery = MockAdapter::default();
        (
            discovery.discover().await?,
            Arc::new(MockAdapter::default()),
        )
    } else {
        let discovery = WindowsDiscoveryAdapter;
        (
            discovery.discover().await?,
            Arc::new(UnavailableTelemetryAdapter::default()),
        )
    };

    let (telemetry_tx, _) = broadcast::channel(32);
    let state = AppState {
        report: Arc::new(report),
        safety: Arc::new(RwLock::new(SafetyState::default())),
        latest: Arc::new(RwLock::new(None)),
        telemetry_tx,
    };
    spawn_telemetry(telemetry, state.clone(), Duration::from_secs(1));
    let address =
        validate_bind(&std::env::var("FANFORGE_BIND").unwrap_or_else(|_| "127.0.0.1:4077".into()))?;
    fanforge_api::serve(address, state).await
}

#[cfg(test)]
mod tests {
    use super::validate_bind;

    #[test]
    fn rejects_non_loopback_bind_addresses() {
        assert!(validate_bind("127.0.0.1:4077").is_ok());
        assert!(validate_bind("[::1]:4077").is_ok());
        assert!(validate_bind("0.0.0.0:4077").is_err());
        assert!(validate_bind("192.168.1.10:4077").is_err());
    }
}
