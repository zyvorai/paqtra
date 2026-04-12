// WebSocket handlers for real-time updates
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::AppState;
use crate::middleware::auth::Claims;
use crate::services::hubble::hubble_json_to_flow;

/// Query parameter for WebSocket token-based authentication.
#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
}

/// Validate a JWT token from the WebSocket query string.
/// Returns Ok(()) if auth is disabled or the token is valid.
fn validate_ws_token(state: &AppState, query: &WsAuthQuery) -> Result<(), (StatusCode, String)> {
    if state.config.auth_disabled {
        return Ok(());
    }
    let token = query.token.as_deref().ok_or((
        StatusCode::UNAUTHORIZED,
        "Missing 'token' query parameter for WebSocket authentication".to_string(),
    ))?;
    let decoding_key = DecodingKey::from_secret(state.config.jwt_secret.as_bytes());
    let validation = Validation::new(Algorithm::HS256);
    decode::<Claims>(token, &decoding_key, &validation).map_err(|e| {
        tracing::debug!("WebSocket token validation failed: {}", e);
        (
            StatusCode::UNAUTHORIZED,
            "Invalid or expired token".to_string(),
        )
    })?;
    Ok(())
}

/// Maximum time without a pong before considering the connection dead
const PING_INTERVAL_SECS: u64 = 30;

/// Maximum concurrent WebSocket connections
const MAX_WS_CONNECTIONS: usize = 100;

/// Global counter of active WebSocket connections
static WS_CONNECTION_COUNT: AtomicUsize = AtomicUsize::new(0);

/// RAII guard that decrements the connection counter on drop
struct WsConnectionGuard;

impl WsConnectionGuard {
    fn try_acquire() -> Option<Self> {
        let mut current = WS_CONNECTION_COUNT.load(Ordering::Relaxed);
        loop {
            if current >= MAX_WS_CONNECTIONS {
                return None;
            }
            match WS_CONNECTION_COUNT.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(Self),
                Err(actual) => current = actual,
            }
        }
    }
}

impl Drop for WsConnectionGuard {
    fn drop(&mut self) {
        WS_CONNECTION_COUNT.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Send a ping; returns false if the client disconnected.
async fn ws_ping(socket: &mut WebSocket, label: &str) -> bool {
    if socket.send(Message::Ping(vec![].into())).await.is_err() {
        tracing::debug!("{} WebSocket client disconnected (ping failed)", label);
        return false;
    }
    true
}

/// Handle a received WebSocket message. Returns `true` to keep looping.
fn handle_ws_message(msg: Option<Result<Message, axum::Error>>, label: &str) -> bool {
    match msg {
        Some(Ok(Message::Close(_))) | None => {
            tracing::debug!("{} WebSocket client disconnected", label);
            false
        }
        Some(Ok(Message::Pong(_))) => true,
        Some(Err(e)) => {
            tracing::debug!("{} WebSocket error: {}", label, e);
            false
        }
        _ => true,
    }
}

pub async fn flows_websocket(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(auth_query): Query<WsAuthQuery>,
) -> Response {
    if let Err((status, msg)) = validate_ws_token(&state, &auth_query) {
        return (status, msg).into_response();
    }
    let guard = match WsConnectionGuard::try_acquire() {
        Some(g) => g,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Too many WebSocket connections").into_response(),
    };
    ws.on_upgrade(move |socket| handle_flows_socket(socket, state, guard))
}

async fn handle_flows_socket(mut socket: WebSocket, state: Arc<AppState>, _guard: WsConnectionGuard) {
    tracing::info!("WebSocket connection established for flows");

    // Send initial message
    if let Err(e) = socket.send(Message::Text(
        serde_json::json!({"type": "connected", "message": "Flow stream ready"}).to_string().into()
    )).await {
        tracing::error!("WebSocket send error: {}", e);
        return;
    }

    let mut ping_interval = tokio::time::interval(
        tokio::time::Duration::from_secs(PING_INTERVAL_SECS)
    );
    let mut flow_interval = tokio::time::interval(
        tokio::time::Duration::from_secs(5)
    );

    loop {
        tokio::select! {
            _ = flow_interval.tick() => {
                let summary = serde_json::json!({
                    "type": "flow_summary",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "flows_fetched": state.metrics.flows_fetched.load(Ordering::Relaxed),
                    "hubble_queries": state.metrics.hubble_queries.load(Ordering::Relaxed),
                });
                if socket.send(Message::Text(summary.to_string().into())).await.is_err() {
                    tracing::debug!("Flow WebSocket client disconnected");
                    break;
                }
            }
            _ = ping_interval.tick() => {
                if !ws_ping(&mut socket, "Flow").await { break; }
            }
            msg = socket.recv() => {
                if !handle_ws_message(msg, "Flow") { break; }
            }
        }
    }
}

pub async fn metrics_websocket(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(auth_query): Query<WsAuthQuery>,
) -> Response {
    if let Err((status, msg)) = validate_ws_token(&state, &auth_query) {
        return (status, msg).into_response();
    }
    let guard = match WsConnectionGuard::try_acquire() {
        Some(g) => g,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Too many WebSocket connections").into_response(),
    };
    ws.on_upgrade(move |socket| handle_metrics_socket(socket, state, guard))
}

async fn handle_metrics_socket(mut socket: WebSocket, state: Arc<AppState>, _guard: WsConnectionGuard) {
    tracing::info!("WebSocket connection established for metrics");

    let mut metrics_interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
    let mut ping_interval = tokio::time::interval(
        tokio::time::Duration::from_secs(PING_INTERVAL_SECS)
    );

    loop {
        tokio::select! {
            _ = metrics_interval.tick() => {
                let m = &state.metrics;
                let metrics = serde_json::json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "total_requests": m.total_requests.load(Ordering::Relaxed),
                    "total_errors": m.total_errors.load(Ordering::Relaxed),
                    "flows_fetched": m.flows_fetched.load(Ordering::Relaxed),
                    "cache_hits": m.cache_hits.load(Ordering::Relaxed),
                    "cache_misses": m.cache_misses.load(Ordering::Relaxed),
                    "hubble_queries": m.hubble_queries.load(Ordering::Relaxed),
                    "k8s_queries": m.k8s_queries.load(Ordering::Relaxed),
                });

                if socket.send(Message::Text(metrics.to_string().into())).await.is_err() {
                    tracing::debug!("Metrics WebSocket client disconnected");
                    break;
                }
            }
            _ = ping_interval.tick() => {
                if !ws_ping(&mut socket, "Metrics").await { break; }
            }
            msg = socket.recv() => {
                if !handle_ws_message(msg, "Metrics") { break; }
            }
        }
    }
}

pub async fn ws_live_flows(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsAuthQuery>,
    ws: WebSocketUpgrade,
) -> Result<Response, (StatusCode, String)> {
    validate_ws_token(&state, &query)?;

    let guard = match WsConnectionGuard::try_acquire() {
        Some(g) => g,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "Too many WebSocket connections".to_string(),
            ));
        }
    };

    let namespace = query.namespace.clone();

    Ok(ws.on_upgrade(move |socket| handle_live_flows(socket, state, guard, namespace)))
}

async fn handle_live_flows(
    mut socket: WebSocket,
    state: Arc<AppState>,
    _guard: WsConnectionGuard,
    namespace: Option<String>,
) {
    tracing::info!(
        "Live flow WebSocket connection established (namespace: {:?})",
        namespace
    );

    // Send initial connected message
    if let Err(e) = socket
        .send(Message::Text(
            serde_json::json!({
                "type": "connected",
                "message": "Live flow stream ready",
                "namespace": namespace,
            })
            .to_string()
            .into(),
        ))
        .await
    {
        tracing::error!("Live flow WebSocket send error: {}", e);
        return;
    }

    // Validate namespace to prevent flag injection
    if let Some(ref ns) = namespace {
        if ns.starts_with('-') || ns.contains(char::is_whitespace) {
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({
                        "type": "error",
                        "message": "Invalid namespace parameter",
                    })
                    .to_string()
                    .into(),
                ))
                .await;
            return;
        }
    }

    // Spawn hubble observe --follow
    let mut cmd = tokio::process::Command::new("hubble");
    cmd.arg("observe")
        .arg("--follow")
        .arg("--output")
        .arg("json")
        .arg("--server")
        .arg(state.hubble.address());

    if let Some(ref ns) = namespace {
        cmd.arg("--namespace").arg(ns);
    }

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to spawn hubble observe: {}", e);
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({
                        "type": "error",
                        "message": format!("Failed to start hubble observe: {}", e),
                    })
                    .to_string()
                    .into(),
                ))
                .await;
            return;
        }
    };

    let stdout = child.stdout.take().expect("stdout was piped");
    let mut lines = BufReader::new(stdout).lines();

    let mut ping_interval =
        tokio::time::interval(tokio::time::Duration::from_secs(PING_INTERVAL_SECS));
    let mut flow_index: usize = 0;

    loop {
        tokio::select! {
            line_result = lines.next_line() => {
                match line_result {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        let parsed = match serde_json::from_str::<serde_json::Value>(&line) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let flow = hubble_json_to_flow(flow_index, &parsed);
                        flow_index = flow_index.wrapping_add(1);

                        let msg = serde_json::json!({
                            "type": "flow",
                            "data": flow,
                        });
                        if socket
                            .send(Message::Text(msg.to_string().into()))
                            .await
                            .is_err()
                        {
                            tracing::debug!("Live flow WebSocket client disconnected");
                            break;
                        }
                    }
                    Ok(None) => {
                        // hubble process exited
                        tracing::debug!("hubble observe process exited");
                        let _ = socket
                            .send(Message::Text(
                                serde_json::json!({
                                    "type": "error",
                                    "message": "hubble observe process exited",
                                })
                                .to_string()
                                .into(),
                            ))
                            .await;
                        break;
                    }
                    Err(e) => {
                        tracing::error!("Error reading hubble output: {}", e);
                        break;
                    }
                }
            }
            _ = ping_interval.tick() => {
                if !ws_ping(&mut socket, "LiveFlow").await {
                    break;
                }
            }
            msg = socket.recv() => {
                if !handle_ws_message(msg, "LiveFlow") {
                    break;
                }
            }
        }
    }

    // Kill the child process on disconnect
    if let Err(e) = child.kill().await {
        tracing::debug!("Failed to kill hubble observe process: {}", e);
    }
}
