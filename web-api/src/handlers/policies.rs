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

pub async fn list_policies(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Listing policies");

    {
        let mut m = state.metrics.write().await;
        m.total_requests += 1;
        m.k8s_queries += 1;
    }

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
            let mut m = state.metrics.write().await;
            m.total_errors += 1;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn create_policy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePolicyRequest>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Creating policy: {}/{}", req.namespace, req.name);

    {
        let mut m = state.metrics.write().await;
        m.total_requests += 1;
        m.k8s_queries += 1;
    }

    match state.k8s.create_policy(&req).await {
        Ok(policy) => {
            {
                let mut m = state.metrics.write().await;
                m.policies_created += 1;
            }
            Ok(Json(serde_json::to_value(policy).unwrap_or(json!({
                "status": "created"
            }))))
        }
        Err(e) => {
            tracing::error!("Failed to create policy: {}", e);
            let mut m = state.metrics.write().await;
            m.total_errors += 1;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_policy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Getting policy: {}", id);

    {
        let mut m = state.metrics.write().await;
        m.total_requests += 1;
        m.k8s_queries += 1;
    }

    // Fetch all and find by id
    match state.k8s.list_policies().await {
        Ok(policies) => {
            match policies.into_iter().find(|p| p.id == id || p.name == id) {
                Some(policy) => {
                    Ok(Json(serde_json::to_value(policy).unwrap_or(json!({}))))
                }
                None => Ok(Json(json!({
                    "error": "not_found",
                    "id": id,
                }))),
            }
        }
        Err(e) => {
            tracing::error!("Failed to get policy: {}", e);
            let mut m = state.metrics.write().await;
            m.total_errors += 1;
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

    {
        let mut m = state.metrics.write().await;
        m.total_requests += 1;
        m.k8s_queries += 1;
    }

    // kubectl apply is idempotent, so create == update
    match state.k8s.create_policy(&req).await {
        Ok(mut policy) => {
            policy.id = id;
            policy.status = "updated".to_string();
            Ok(Json(serde_json::to_value(policy).unwrap_or(json!({
                "status": "updated"
            }))))
        }
        Err(e) => {
            tracing::error!("Failed to update policy: {}", e);
            let mut m = state.metrics.write().await;
            m.total_errors += 1;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn delete_policy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Deleting policy: {}", id);

    {
        let mut m = state.metrics.write().await;
        m.total_requests += 1;
        m.k8s_queries += 1;
    }

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
            let mut m = state.metrics.write().await;
            m.total_errors += 1;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn simulate_policy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePolicyRequest>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Simulating policy: {}", req.name);

    {
        let mut m = state.metrics.write().await;
        m.total_requests += 1;
    }

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
