//! Investigation API: path explain + evidence bundles + export.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::services::investigate::{
    self, bundle_namespaces, bundle_to_markdown, InvestigateFlowRequest, InvestigatePathRequest,
};
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
    if !super::has_namespace_access(&state, &claims, &req.source.namespace)
        || !super::has_namespace_access(&state, &claims, &req.destination.namespace)
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "namespace not in scope"})),
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
        Ok(Some(v)) => {
            enforce_bundle_ns(&state, &claims, &v)?;
            Ok(Json(v))
        }
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

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    /// `json` (default) or `markdown`
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "json".into()
}

/// GET /api/v1/investigate/bundles/{id}/export?format=json|markdown
pub async fn export_bundle(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Path(id): Path<String>,
    Query(q): Query<ExportQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    super::check_editor(&state, &claims)?;

    if id.is_empty() || id.contains('/') || id.contains("..") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid bundle id"})),
        ));
    }

    let key = format!("cv:investigate_bundle:{id}");
    let bundle = match state.cache.get::<Value>(&key).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({"error": "bundle not found"})),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            ))
        }
    };
    enforce_bundle_ns(&state, &claims, &bundle)?;

    let fmt = q.format.to_lowercase();
    if fmt == "markdown" || fmt == "md" {
        // For flow-deny bundles nested under `result`, unwrap for readability.
        let src = bundle
            .get("result")
            .cloned()
            .unwrap_or_else(|| bundle.clone());
        let md = if src.get("steps").is_some() {
            bundle_to_markdown(&json!({
                "id": bundle.get("id").or(src.get("id")),
                "created_at": src.get("created_at"),
                "likely_owner": src.get("likely_owner"),
                "request": src.get("request"),
                "steps": src.get("steps"),
                "source_health": bundle.get("source_health").or(src.get("flow_ingest")),
                "cited_flow_ids": bundle.get("cited_flow_ids").cloned().unwrap_or(json!([])),
                "related_change_ids": bundle.get("related_change_ids").cloned().unwrap_or(json!([])),
            }))
        } else {
            bundle_to_markdown(&bundle)
        };
        return Ok(Json(json!({
            "id": id,
            "format": "markdown",
            "content": md,
            "redaction": "No payloads, argv, or Secret contents included.",
        })));
    }

    Ok(Json(json!({
        "id": id,
        "format": "json",
        "bundle": bundle,
        "redaction": "No payloads, argv, or Secret contents included.",
    })))
}

fn enforce_bundle_ns(
    state: &AppState,
    claims: &Option<axum::Extension<crate::middleware::auth::Claims>>,
    bundle: &Value,
) -> Result<(), (StatusCode, Json<Value>)> {
    let namespaces = bundle_namespaces(bundle);
    if namespaces.is_empty() {
        return Ok(());
    }
    if namespaces
        .iter()
        .any(|ns| super::has_namespace_access(state, claims, ns))
    {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "bundle namespaces not in scope"})),
        ))
    }
}
