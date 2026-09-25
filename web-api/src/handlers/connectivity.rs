//! Connectivity path monitoring handlers (observe-only).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::services::connectivity::{self, CreatePathRequest};
use crate::AppState;

pub async fn list_paths(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    super::check_editor(&state, &claims)?;
    let paths = connectivity::list_paths(&state, &claims).await;
    let mut with_status = Vec::new();
    for p in paths {
        let status = connectivity::status_for_path(&state, &p).await;
        with_status.push(json!({ "path": p, "status": status }));
    }
    Ok(Json(json!({ "paths": with_status })))
}

pub async fn create_path(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(req): Json<CreatePathRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    super::check_editor(&state, &claims)?;
    match connectivity::create_path(&state, &claims, req).await {
        Ok(p) => Ok(Json(serde_json::to_value(p).unwrap_or(json!({})))),
        Err((st, msg)) => Err((st, Json(json!({ "error": msg })))),
    }
}

pub async fn delete_path(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    super::check_editor(&state, &claims)?;
    match connectivity::delete_path(&state, &claims, &id).await {
        Ok(()) => Ok(Json(json!({ "id": id, "status": "deleted" }))),
        Err((st, msg)) => Err((st, Json(json!({ "error": msg })))),
    }
}

pub async fn list_alerts(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    super::check_editor(&state, &claims)?;
    Ok(Json(json!({
        "alerts": connectivity::list_alerts(&state, &claims).await
    })))
}
