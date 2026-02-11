// Health check endpoints
use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

pub async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}

pub async fn readiness_check() -> (StatusCode, Json<Value>) {
    // TODO: Check dependencies (Redis, Hubble, K8s)
    (
        StatusCode::OK,
        Json(json!({
            "status": "ready",
            "checks": {
                "redis": "ok",
                "hubble": "ok",
                "kubernetes": "ok"
            }
        })),
    )
}
