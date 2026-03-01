// Health check endpoints
use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;

pub async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}

pub async fn readiness_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<Value>) {
    let mut checks = serde_json::Map::new();
    let mut all_ok = true;

    // Check Redis/Cache connectivity
    let cache_healthy = state.cache.is_healthy().await;
    if cache_healthy {
        checks.insert("redis".to_string(), json!("ok"));
    } else {
        all_ok = false;
        checks.insert("redis".to_string(), json!("error"));
        tracing::warn!("Redis readiness check failed");
    }

    // Check Hubble connectivity
    let hubble_healthy = state.hubble.is_healthy().await;
    if hubble_healthy {
        checks.insert("hubble".to_string(), json!("ok"));
    } else {
        // Hubble being down is degraded, not fatal
        checks.insert("hubble".to_string(), json!("unavailable"));
        tracing::info!("Hubble relay is not reachable (degraded mode)");
    }

    // Check Kubernetes connectivity
    let k8s_healthy = state.k8s.is_healthy().await;
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

    (
        status,
        Json(json!({
            "status": overall,
            "checks": checks,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}
