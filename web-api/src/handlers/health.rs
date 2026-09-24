// Health check endpoints
use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;

use crate::AppState;

/// Timeout for individual health-check probes.
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(3);

/// Tracks when the process started, used to compute uptime in health checks.
static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Return the process start time, initialising it on first call.
fn process_start() -> Instant {
    *START_TIME.get_or_init(Instant::now)
}

pub async fn health_check(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    let uptime = process_start().elapsed();

    let (hubble_res, k8s_res) = tokio::join!(
        timeout(HEALTH_CHECK_TIMEOUT, state.hubble.is_healthy()),
        timeout(HEALTH_CHECK_TIMEOUT, state.k8s.is_healthy()),
    );
    let hubble_ok = hubble_res.unwrap_or(false);
    let k8s_ok = k8s_res.unwrap_or(false);

    let overall = if hubble_ok && k8s_ok {
        "healthy"
    } else {
        "degraded"
    };

    let flow_stats = state.flow_store.stats();

    (
        StatusCode::OK,
        Json(json!({
            "status": overall,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_secs": uptime.as_secs(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "subsystems": {
                "cache": "ok",
                "hubble_relay": if hubble_ok { "ok" } else { "unavailable" },
                "kubernetes": if k8s_ok { "ok" } else { "unavailable" },
                "flow_ingest": {
                    "status": if flow_stats.last_ingest_ok { "ok" } else { "unavailable" },
                    "source": flow_stats.ingest_source,
                    "indexed": flow_stats.total,
                    "last_at": flow_stats.last_ingest_at,
                    "hubble_mode": match state.hubble.mode() {
                        crate::config::HubbleMode::Auto => "auto",
                        crate::config::HubbleMode::Grpc => "grpc",
                        crate::config::HubbleMode::Cli => "cli",
                    },
                    "connected": flow_stats.stream_connected,
                    "disconnects": flow_stats.disconnect_count,
                    "gaps": flow_stats.gap_count,
                    "last_gap_at": flow_stats.last_gap_at,
                    "events_per_sec": flow_stats.events_per_sec,
                    "lag_secs": flow_stats.lag_secs,
                },
            }
        })),
    )
}

pub async fn readiness_check(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    let mut checks = serde_json::Map::new();

    let (hubble_res, k8s_res) = tokio::join!(
        timeout(HEALTH_CHECK_TIMEOUT, state.hubble.is_healthy()),
        timeout(HEALTH_CHECK_TIMEOUT, state.k8s.is_healthy()),
    );

    checks.insert("cache".to_string(), json!("ok"));

    let hubble_healthy = hubble_res.unwrap_or(false);
    if hubble_healthy {
        checks.insert("hubble".to_string(), json!("ok"));
    } else {
        checks.insert("hubble".to_string(), json!("unavailable"));
        tracing::info!("Hubble relay is not reachable (degraded mode)");
    }

    let k8s_healthy = k8s_res.unwrap_or(false);
    if k8s_healthy {
        checks.insert("kubernetes".to_string(), json!("ok"));
    } else {
        checks.insert("kubernetes".to_string(), json!("unavailable"));
        tracing::info!("Kubernetes API is not reachable (degraded mode)");
    }

    let overall = if hubble_healthy && k8s_healthy {
        "ready"
    } else {
        "degraded"
    };

    let uptime = process_start().elapsed();

    (
        StatusCode::OK,
        Json(json!({
            "status": overall,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_secs": uptime.as_secs(),
            "checks": checks,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}
