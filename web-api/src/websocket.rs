// WebSocket handlers for real-time updates
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State},
    response::Response,
};
use std::sync::Arc;
use crate::AppState;

/// Maximum time without a pong before considering the connection dead
const PING_INTERVAL_SECS: u64 = 30;

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

    // Handle incoming messages (client may send filter commands)
    let mut ping_interval = tokio::time::interval(
        tokio::time::Duration::from_secs(PING_INTERVAL_SECS)
    );

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                if socket.send(Message::Ping(vec![].into())).await.is_err() {
                    tracing::debug!("Flow WebSocket client disconnected (ping failed)");
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::debug!("Flow WebSocket client disconnected");
                        break;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Connection is alive
                    }
                    Some(Ok(Message::Text(_text))) => {
                        // Could handle filter commands from client here
                    }
                    Some(Err(e)) => {
                        tracing::debug!("Flow WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
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

    // Send metrics updates periodically with proper ping/pong
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
                if socket.send(Message::Ping(vec![].into())).await.is_err() {
                    tracing::debug!("Metrics WebSocket client disconnected (ping failed)");
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::debug!("Metrics WebSocket client disconnected");
                        break;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Connection is alive
                    }
                    Some(Err(e)) => {
                        tracing::debug!("Metrics WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}
