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

pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<Value>) {
    let uptime = process_start().elapsed();

    // Probe each subsystem concurrently with a timeout so one slow probe
    // cannot block the others or hang the endpoint indefinitely.
    let (redis_res, hubble_res, k8s_res) = tokio::join!(
        timeout(HEALTH_CHECK_TIMEOUT, state.cache.is_healthy()),
        timeout(HEALTH_CHECK_TIMEOUT, state.hubble.is_healthy()),
        timeout(HEALTH_CHECK_TIMEOUT, state.k8s.is_healthy()),
    );
    let redis_ok = redis_res.unwrap_or(false);
    let hubble_ok = hubble_res.unwrap_or(false);
    let k8s_ok = k8s_res.unwrap_or(false);

    let overall = if redis_ok && hubble_ok && k8s_ok {
        "healthy"
    } else if redis_ok {
        "degraded"
    } else {
        "unhealthy"
    };
    let code = if redis_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        code,
        Json(json!({
            "status": overall,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_secs": uptime.as_secs(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "subsystems": {
                "redis": if redis_ok { "ok" } else { "error" },
                "hubble_relay": if hubble_ok { "ok" } else { "unavailable" },
                "kubernetes": if k8s_ok { "ok" } else { "unavailable" },
            }
        })),
    )
}

pub async fn readiness_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<Value>) {
    let mut checks = serde_json::Map::new();
    let mut all_ok = true;

    // Run all readiness probes concurrently with a timeout
    let (cache_res, hubble_res, k8s_res) = tokio::join!(
        timeout(HEALTH_CHECK_TIMEOUT, state.cache.is_healthy()),
        timeout(HEALTH_CHECK_TIMEOUT, state.hubble.is_healthy()),
        timeout(HEALTH_CHECK_TIMEOUT, state.k8s.is_healthy()),
    );

    let cache_healthy = cache_res.unwrap_or(false);
    if cache_healthy {
        checks.insert("redis".to_string(), json!("ok"));
    } else {
        all_ok = false;
        checks.insert("redis".to_string(), json!("error"));
        tracing::warn!("Redis readiness check failed");
    }

    let hubble_healthy = hubble_res.unwrap_or(false);
    if hubble_healthy {
        checks.insert("hubble".to_string(), json!("ok"));
    } else {
        // Hubble being down is degraded, not fatal
        checks.insert("hubble".to_string(), json!("unavailable"));
        tracing::info!("Hubble relay is not reachable (degraded mode)");
    }

    let k8s_healthy = k8s_res.unwrap_or(false);
    if k8s_healthy {
        checks.insert("kubernetes".to_string(), json!("ok"));
    } else {
        // K8s being down is degraded, not fatal
        checks.insert("kubernetes".to_string(), json!("unavailable"));
        tracing::info!("Kubernetes API is not reachable (degraded mode)");
    }

    let status = if all_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let overall = if all_ok && hubble_healthy && k8s_healthy {
        "ready"
    } else if all_ok {
        "degraded"
    } else {
        "not_ready"
    };

    let uptime = process_start().elapsed();

    (
        status,
        Json(json!({
            "status": overall,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_secs": uptime.as_secs(),
            "checks": checks,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}
