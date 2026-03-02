// Policy management endpoints
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;
use crate::models::policy::CreatePolicyRequest;
use super::{track_request, track_error, to_json};

pub async fn list_policies(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Listing policies");

    track_request(&state, |m| m.k8s_queries += 1).await;

    match state.k8s.list_policies().await {
        Ok(policies) => {
            let total = policies.len();
            Ok(Json(json!({
                "policies": policies,
                "total": total,
            })))
        }
        Err(e) => {
            tracing::error!("Failed to list policies: {}", e);
            track_error(&state).await;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn create_policy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePolicyRequest>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Creating policy: {}/{}", req.namespace, req.name);

    track_request(&state, |m| m.k8s_queries += 1).await;

    match state.k8s.create_policy(&req).await {
        Ok(policy) => {
            {
                let mut m = state.metrics.write().await;
                m.policies_created += 1;
            }
            Ok(Json(to_json(&policy)))
        }
        Err(e) => {
            tracing::error!("Failed to create policy: {}", e);
            track_error(&state).await;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_policy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Getting policy: {}", id);

    track_request(&state, |m| m.k8s_queries += 1).await;

    // Fetch all and find by id
    match state.k8s.list_policies().await {
        Ok(policies) => {
            match policies.into_iter().find(|p| p.id == id || p.name == id) {
                Some(policy) => {
                    Ok(Json(to_json(&policy)))
                }
                None => Ok(Json(json!({
                    "error": "not_found",
                    "id": id,
                }))),
            }
        }
        Err(e) => {
            tracing::error!("Failed to get policy: {}", e);
            track_error(&state).await;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn update_policy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreatePolicyRequest>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Updating policy: {}", id);

    track_request(&state, |m| m.k8s_queries += 1).await;

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
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn delete_policy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Deleting policy: {}", id);

    track_request(&state, |m| m.k8s_queries += 1).await;

    match state.k8s.delete_policy(&id).await {
        Ok(()) => {
            {
                let mut m = state.metrics.write().await;
                m.policies_deleted += 1;
            }
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            tracing::error!("Failed to delete policy: {}", e);
            track_error(&state).await;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn simulate_policy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePolicyRequest>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Simulating policy: {}", req.name);

    track_request(&state, |_| {}).await;

    // Simulation would analyze flows against the proposed policy
    // For now, return structured response
    Ok(Json(json!({
        "policy": req.name,
        "namespace": req.namespace,
        "impact": {
            "flows_affected": 0,
            "services_impacted": 0,
            "risk_level": "unknown",
            "note": "Policy simulation engine not yet connected"
        }
    })))
}
