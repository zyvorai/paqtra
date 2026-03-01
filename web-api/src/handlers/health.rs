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

    // Check Redis connectivity
    let redis_status = match redis::cmd("PING")
        .query_async::<String>(&mut state.redis.clone())
        .await
    {
        Ok(_) => "ok".to_string(),
        Err(e) => {
            all_ok = false;
            tracing::warn!("Redis readiness check failed: {}", e);
            format!("error: {}", e)
        }
    };
    checks.insert("redis".to_string(), json!(redis_status));

    // Hubble and K8s are reported as unknown until services are connected
    checks.insert("hubble".to_string(), json!("not_connected"));
    checks.insert("kubernetes".to_string(), json!("not_connected"));

    let status = if all_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(json!({
            "status": if all_ok { "ready" } else { "not_ready" },
            "checks": checks
        })),
    )
}
