use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use fanforge_core::{DiscoveryReport, FanForgeError, TelemetryAdapter, TelemetryFrame};
use fanforge_safety::SafetyState;
use serde::Serialize;
use tokio::sync::{broadcast, RwLock};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Clone)]
pub struct AppState {
    pub report: Arc<DiscoveryReport>,
    pub safety: Arc<RwLock<SafetyState>>,
    pub latest: Arc<RwLock<Option<TelemetryFrame>>>,
    pub telemetry_tx: broadcast::Sender<TelemetryFrame>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Health {
    status: &'static str,
    api_version: &'static str,
    mode: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiErrorBody {
    code: &'static str,
    message: String,
}

struct ApiError(FanForgeError);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            FanForgeError::UnsupportedHardware(_) | FanForgeError::BiosNotVerified(_) => {
                StatusCode::CONFLICT
            }
            FanForgeError::TelemetryStale | FanForgeError::LeaseUnavailable => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            FanForgeError::RestoreFailed(_) | FanForgeError::Adapter(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        (
            status,
            Json(ApiErrorBody {
                code: self.0.code(),
                message: self.0.to_string(),
            }),
        )
            .into_response()
    }
}

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse::<HeaderValue>().unwrap(),
            "http://127.0.0.1:5173".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([Method::GET]);
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/system", get(system))
        .route("/api/v1/fans", get(fans))
        .route("/api/v1/sensors", get(sensors))
        .route("/api/v1/safety", get(safety))
        .route("/api/v1/telemetry/latest", get(latest))
        .route("/api/v1/telemetry", get(websocket))
        .route("/api/v1/fans/{id}/manual", post(control_disabled))
        .route("/api/v1/fans/{id}/auto", post(control_disabled))
        .route("/api/v1/curves/{id}", put(control_disabled))
        .route("/api/v1/profiles/activate", post(control_disabled))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn serve(address: SocketAddr, state: AppState) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "FanForge API listening");
    axum::serve(listener, router(state)).await?;
    Ok(())
}

pub fn spawn_telemetry(adapter: Arc<dyn TelemetryAdapter>, state: AppState, interval: Duration) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            match adapter.read().await {
                Ok(frame) => {
                    let fresh = frame.sensors.iter().any(|reading| reading.valid)
                        || frame.fans.iter().any(|reading| reading.valid);
                    state.safety.write().await.telemetry_fresh = fresh;
                    *state.latest.write().await = Some(frame.clone());
                    let _ = state.telemetry_tx.send(frame);
                }
                Err(error) => {
                    state.safety.write().await.telemetry_fresh = false;
                    tracing::warn!(%error, "telemetry sample failed");
                }
            }
        }
    });
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        api_version: "v1",
        mode: "read_only",
    })
}
async fn system(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(
        serde_json::json!({ "identity": state.report.identity, "adapter": state.report.adapter, "capabilities": state.report.capabilities, "diagnostics": state.report.diagnostics }),
    )
}
async fn fans(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "fans": state.report.fans }))
}
async fn sensors(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "sensors": state.report.sensors }))
}
async fn safety(State(state): State<AppState>) -> Json<SafetyState> {
    Json(state.safety.read().await.clone())
}
async fn latest(State(state): State<AppState>) -> Response {
    match state.latest.read().await.clone() {
        Some(frame) => Json(frame).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}
async fn control_disabled() -> Result<Response, ApiError> {
    Err(ApiError(FanForgeError::UnsupportedHardware(
        "fan writes are disabled in the read-only milestone".into(),
    )))
}
async fn websocket(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| stream(socket, state))
}
async fn stream(mut socket: WebSocket, state: AppState) {
    if let Some(frame) = state.latest.read().await.clone() {
        if socket
            .send(Message::Text(serde_json::to_string(&frame).unwrap().into()))
            .await
            .is_err()
        {
            return;
        }
    }
    let mut receiver = state.telemetry_tx.subscribe();
    while let Ok(frame) = receiver.recv().await {
        if socket
            .send(Message::Text(serde_json::to_string(&frame).unwrap().into()))
            .await
            .is_err()
        {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fanforge_core::{DiscoveryAdapter, MockAdapter};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn test_state() -> AppState {
        let report = Arc::new(MockAdapter::default().discover().await.unwrap());
        let (telemetry_tx, _) = broadcast::channel(8);
        AppState {
            report,
            safety: Arc::new(RwLock::new(SafetyState::default())),
            latest: Arc::new(RwLock::new(None)),
            telemetry_tx,
        }
    }

    #[tokio::test]
    async fn health_contract_is_available() {
        let response = router(test_state().await)
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn read_only_inventory_routes_are_available() {
        for path in [
            "/api/v1/system",
            "/api/v1/fans",
            "/api/v1/sensors",
            "/api/v1/safety",
        ] {
            let response = router(test_state().await)
                .oneshot(
                    axum::http::Request::builder()
                        .uri(path)
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "route {path}");
        }
    }

    #[tokio::test]
    async fn every_mutation_fails_closed_with_stable_error_code() {
        let requests = [
            (Method::POST, "/api/v1/fans/cpu-fan/manual"),
            (Method::POST, "/api/v1/fans/cpu-fan/auto"),
            (Method::PUT, "/api/v1/curves/default"),
            (Method::POST, "/api/v1/profiles/activate"),
        ];
        for (method, path) in requests {
            let response = router(test_state().await)
                .oneshot(
                    axum::http::Request::builder()
                        .method(method)
                        .uri(path)
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::CONFLICT, "route {path}");
            let body = response.into_body().collect().await.unwrap().to_bytes();
            let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(json["code"], "unsupported_hardware", "route {path}");
        }
    }
}
