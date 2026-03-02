// WebSocket handlers for real-time updates
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State},
    response::Response,
};
use std::sync::Arc;
use crate::AppState;

/// Maximum time without a pong before considering the connection dead
const PING_INTERVAL_SECS: u64 = 30;

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
    State(_state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(handle_flows_socket)
}

async fn handle_flows_socket(mut socket: WebSocket) {
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

    loop {
        tokio::select! {
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
    State(_state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(handle_metrics_socket)
}

async fn handle_metrics_socket(mut socket: WebSocket) {
    tracing::info!("WebSocket connection established for metrics");

    let mut metrics_interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
    let mut ping_interval = tokio::time::interval(
        tokio::time::Duration::from_secs(PING_INTERVAL_SECS)
    );

    loop {
        tokio::select! {
            _ = metrics_interval.tick() => {
                let metrics = serde_json::json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "requests_per_sec": 0,
                    "avg_latency_ms": 0,
                    "error_rate": 0.0,
                    "note": "Connect to Hubble for real metrics"
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
