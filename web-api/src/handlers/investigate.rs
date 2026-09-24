//! Investigation API: path explain + evidence bundles.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::services::investigate::{self, InvestigateFlowRequest, InvestigatePathRequest};
use crate::AppState;

/// POST /api/v1/investigate/path
pub async fn investigate_path(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(req): Json<InvestigatePathRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    super::check_editor(&state, &claims)?;

    if req.source.namespace.is_empty() || req.destination.namespace.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "source.namespace and destination.namespace are required"})),
        ));
    }
    if req.port == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "port must be non-zero"})),
        ));
    }

    let result = investigate::investigate_path(&state, &req).await;
    Ok(Json(serde_json::to_value(result).unwrap_or(json!({}))))
}

/// POST /api/v1/investigate/flow — why was this connection denied?
pub async fn investigate_flow(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(req): Json<InvestigateFlowRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    super::check_editor(&state, &claims)?;

    if req.flow_id.is_empty() && req.flow.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "flow_id or flow snapshot is required"})),
        ));
    }

    let result = investigate::investigate_flow(&state, &req).await;
    Ok(Json(serde_json::to_value(result).unwrap_or(json!({}))))
}

/// GET /api/v1/investigate/bundles/{id}
pub async fn get_bundle(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    super::check_editor(&state, &claims)?;

    if id.is_empty() || id.contains('/') || id.contains("..") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid bundle id"})),
        ));
    }

    let key = format!("cv:investigate_bundle:{id}");
    match state.cache.get::<Value>(&key).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "bundle not found"})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )),
    }
}
