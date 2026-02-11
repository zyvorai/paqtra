// WebSocket handlers for real-time updates
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade}, State},
    response::Response,
};
use std::sync::Arc;
use crate::AppState;

pub async fn flows_websocket(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(handle_flows_socket)
}

async fn handle_flows_socket(mut socket: WebSocket) {
    tracing::info!("WebSocket connection established for flows");

    // Send initial message
    if let Err(e) = socket.send(axum::extract::ws::Message::Text(
        serde_json::json!({"type": "connected", "message": "Flow stream ready"}).to_string()
    )).await {
        tracing::error!("WebSocket send error: {}", e);
        return;
    }

    // TODO: Stream flows in real-time
    // This would integrate with Hubble gRPC stream
}

pub async fn metrics_websocket(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(handle_metrics_socket)
}

async fn handle_metrics_socket(mut socket: WebSocket) {
    tracing::info!("WebSocket connection established for metrics");

    // Send metrics updates periodically
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

    loop {
        interval.tick().await;

        let metrics = serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "requests_per_sec": 125.3,
            "avg_latency_ms": 45.2,
            "error_rate": 0.01,
        });

        if socket.send(axum::extract::ws::Message::Text(metrics.to_string())).await.is_err() {
            break;
        }
    }
}
