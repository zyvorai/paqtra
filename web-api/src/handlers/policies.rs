// Policy management endpoints
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::{
    actor_from_claims, audit_log, has_namespace_access, to_json, track_error,
    track_request,
};
use crate::error::ApiError;
use crate::models::policy::CreatePolicyRequest;
use crate::AppState;

pub async fn list_policies(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Query(params): Query<super::PaginationQuery>,
) -> Result<Json<Value>, ApiError> {
    tracing::info!("Listing policies");

    track_request(&state, |m| {
        m.k8s_queries.fetch_add(1, Ordering::Relaxed);
    })
    .await;

    match state.k8s.list_policies().await {
        Ok(policies) => {
            // Apply namespace RBAC filter
            let policies: Vec<_> = policies
                .into_iter()
                .filter(|p| has_namespace_access(&state, &claims, &p.namespace))
                .collect();
            let total = policies.len();
            let offset = params.offset.unwrap_or(0);
            let limit = params.limit.unwrap_or(50).min(1000);
            let page: Vec<_> = policies.into_iter().skip(offset).take(limit).collect();
            Ok(Json(json!({
                "policies": page,
                "total": total,
                "limit": limit,
                "offset": offset,
            })))
        }
        Err(e) => {
            tracing::error!("Failed to list policies: {}", e);
            track_error(&state).await;
            Err(ApiError::InternalError(e.to_string()))
        }
    }
}

pub async fn create_policy(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(req): Json<CreatePolicyRequest>,
) -> Result<Json<Value>, ApiError> {
    super::check_editor(&state, &claims).map_err(|_| ApiError::Forbidden)?;

    // Validate spec size and depth
    req.validate_spec().map_err(ApiError::BadRequest)?;

    tracing::info!("Creating policy: {}/{}", req.namespace, req.name);

    track_request(&state, |m| {
        m.k8s_queries.fetch_add(1, Ordering::Relaxed);
    })
    .await;

    match state.k8s.create_policy(&req).await {
        Ok(policy) => {
            state
                .metrics
                .policies_created
                .fetch_add(1, Ordering::Relaxed);
            audit_log(
                &state,
                "policy.create",
                &req.name,
                &req.namespace,
                "Policy created",
                &actor_from_claims(&claims),
                "success",
            )
            .await;
            Ok(Json(to_json(&policy)))
        }
        Err(e) => {
            tracing::error!("Failed to create policy: {}", e);
            track_error(&state).await;
            Err(ApiError::InternalError(e.to_string()))
        }
    }
}

pub async fn get_policy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    tracing::info!("Getting policy: {}", id);

    track_request(&state, |m| {
        m.k8s_queries.fetch_add(1, Ordering::Relaxed);
    })
    .await;

    // Fetch all and find by id
    match state.k8s.list_policies().await {
        Ok(policies) => match policies.into_iter().find(|p| p.id == id || p.name == id) {
            Some(policy) => Ok(Json(to_json(&policy))),
            None => Err(ApiError::NotFound),
        },
        Err(e) => {
            tracing::error!("Failed to get policy: {}", e);
            track_error(&state).await;
            Err(ApiError::InternalError(e.to_string()))
        }
    }
}

pub async fn update_policy(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Path(id): Path<String>,
    Json(req): Json<CreatePolicyRequest>,
) -> Result<Json<Value>, ApiError> {
    super::check_editor(&state, &claims).map_err(|_| ApiError::Forbidden)?;

    // Validate spec size and depth
    req.validate_spec().map_err(ApiError::BadRequest)?;

    tracing::info!("Updating policy: {}", id);

    track_request(&state, |m| {
        m.k8s_queries.fetch_add(1, Ordering::Relaxed);
    })
    .await;

    // Verify the policy exists before updating (prevent IDOR)
    // Also validate that the request body name matches the path ID
    match state.k8s.list_policies().await {
        Ok(policies) => {
            if !policies.iter().any(|p| p.id == id || p.name == id) {
                return Err(ApiError::NotFound);
            }
            // Prevent IDOR: ensure the request body name is consistent with the path ID
            if req.name != id && !policies.iter().any(|p| p.id == id && p.name == req.name) {
                return Err(ApiError::BadRequest(
                    "Policy name does not match path ID".into(),
                ));
            }
        }
        Err(e) => {
            tracing::error!("Failed to look up policy {}: {}", id, e);
            track_error(&state).await;
            return Err(ApiError::InternalError(e.to_string()));
        }
    }

    // kubectl apply is idempotent, so create == update
    match state.k8s.create_policy(&req).await {
        Ok(mut policy) => {
            policy.id = id;
            policy.status = "updated".to_string();
            Ok(Json(to_json(&policy)))
        }
        Err(e) => {
            tracing::error!("Failed to update policy: {}", e);
            track_error(&state).await;
            Err(ApiError::InternalError(e.to_string()))
        }
    }
}

pub async fn delete_policy(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    super::check_editor(&state, &claims).map_err(|_| ApiError::Forbidden)?;
    tracing::info!("Deleting policy: {}", id);

    track_request(&state, |m| {
        m.k8s_queries.fetch_add(1, Ordering::Relaxed);
    })
    .await;

    match state.k8s.delete_policy(&id).await {
        Ok(()) => {
            state
                .metrics
                .policies_deleted
                .fetch_add(1, Ordering::Relaxed);
            audit_log(
                &state,
                "policy.delete",
                &id,
                "",
                "Policy deleted",
                &actor_from_claims(&claims),
                "success",
            )
            .await;
            Ok(axum::http::StatusCode::NO_CONTENT)
        }
        Err(e) => {
            tracing::error!("Failed to delete policy: {}", e);
            track_error(&state).await;
            Err(ApiError::InternalError(e.to_string()))
        }
    }
}

pub async fn simulate_policy(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(req): Json<CreatePolicyRequest>,
) -> Result<Json<Value>, ApiError> {
    super::check_editor(&state, &claims).map_err(|_| ApiError::Forbidden)?;

    // Validate spec size and depth
    req.validate_spec().map_err(ApiError::BadRequest)?;

    tracing::info!("Previewing policy impact: {}", req.name);

    track_request(&state, |m| {
        m.k8s_queries.fetch_add(1, Ordering::Relaxed);
    })
    .await;

    let preview =
        crate::services::investigate::preview_policy(&state, &req.name, &req.namespace, &req.spec)
            .await;

    audit_log(
        &state,
        "policy.simulate",
        &req.name,
        &req.namespace,
        "Policy impact previewed (evidence-backed)",
        &actor_from_claims(&claims),
        "success",
    )
    .await;

    Ok(Json(serde_json::to_value(preview).unwrap_or_else(|_| {
        serde_json::json!({"error": "serialize failed"})
    })))
}
