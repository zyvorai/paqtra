// HTTP request handlers
pub mod health;
pub mod flows;
pub mod policies;
pub mod anomalies;
pub mod compliance;
pub mod modules;
pub mod metrics;
pub mod events;
pub mod endpoints;
pub mod nodes;
pub mod extended;
pub mod extended2;
pub mod extended3;
pub mod extended4;
pub mod ebpf;

use axum::{http::StatusCode, Json};
use crate::{AppMetrics, AppState};
use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::Ordering;

/// RBAC check: require admin role for destructive operations.
/// Returns Ok(()) if auth is disabled or the caller has role == "admin".
/// Returns Err(403) otherwise.
///
/// `claims` comes from `Option<axum::Extension<Claims>>` extractors injected
/// by the auth middleware.
pub fn check_admin(
    state: &AppState,
    claims: &Option<axum::Extension<crate::middleware::auth::Claims>>,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if state.config.auth_disabled { return Ok(()); }
    match claims.as_ref().map(|c| c.role.as_str()) {
        Some("admin") => Ok(()),
        _ => Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Admin role required"})))),
    }
}

/// Increment total_requests and run an extra closure on the metrics.
pub async fn track_request(state: &AppState, f: impl FnOnce(&AppMetrics)) {
    state.metrics.total_requests.fetch_add(1, Ordering::Relaxed);
    f(&state.metrics);
}

/// Increment total_errors.
pub async fn track_error(state: &AppState) {
    state.metrics.total_errors.fetch_add(1, Ordering::Relaxed);
}

/// Serialize to JSON with empty-object fallback.
pub fn to_json<T: Serialize>(val: &T) -> Value {
    serde_json::to_value(val).unwrap_or_else(|_| serde_json::json!({}))
}

/// Shared pagination query params for list endpoints.
#[derive(Debug, serde::Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Apply pagination to a JSON array field and return the response with metadata.
/// `items_key` is the JSON field name for the items array.
pub fn paginate_json(items: Vec<Value>, params: &PaginationQuery, items_key: &str) -> Value {
    let total = items.len();
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(50).min(1000);
    let page: Vec<_> = items.into_iter().skip(offset).take(limit).collect();
    serde_json::json!({
        items_key: page,
        "total": total,
        "limit": limit,
        "offset": offset,
    })
}
