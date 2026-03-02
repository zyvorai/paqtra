// Policy management endpoints
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;
use crate::models::policy::CreatePolicyRequest;
use super::{track_request, track_error, to_json};

/// Query parameters for paginated list endpoints.
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

pub async fn list_policies(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Listing policies");

    track_request(&state, |m| m.k8s_queries += 1).await;

    match state.k8s.list_policies().await {
        Ok(policies) => {
            let total = policies.len();
            let offset = params.offset.unwrap_or(0);
            let limit = params.limit.unwrap_or(50);
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
                None => Err(StatusCode::NOT_FOUND),
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

    track_request(&state, |m| m.k8s_queries += 1).await;

    // Analyze the policy spec to produce a meaningful impact assessment
    let spec = &req.spec;

    // Count ingress and egress rules from the spec
    let ingress_rules = spec.get("ingress")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let egress_rules = spec.get("egress")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let total_rules = ingress_rules + egress_rules;

    // Determine whether an endpoint selector is restrictive or cluster-wide
    let has_endpoint_selector = spec.get("endpointSelector")
        .and_then(|v| v.as_object())
        .map(|obj| !obj.is_empty())
        .unwrap_or(false);

    // Check for port restrictions
    let has_port_rules = spec.get("ingress")
        .and_then(|v| v.as_array())
        .map(|rules| rules.iter().any(|r| r.get("toPorts").is_some()))
        .unwrap_or(false)
        || spec.get("egress")
            .and_then(|v| v.as_array())
            .map(|rules| rules.iter().any(|r| r.get("toPorts").is_some()))
            .unwrap_or(false);

    // Estimate affected flows based on namespace scope and rule count
    let m = state.metrics.read().await;
    let recent_flows = m.flows_fetched;
    drop(m);

    // Rough heuristic: broader selectors affect more flows
    let estimated_affected = if has_endpoint_selector {
        // Targeted policy -- estimate a fraction of recent flows
        std::cmp::max(total_rules as u64 * 5, recent_flows / 10)
    } else {
        // Namespace-wide policy -- larger blast radius
        std::cmp::max(total_rules as u64 * 20, recent_flows / 3)
    };

    let risk_level = match (total_rules, has_endpoint_selector, has_port_rules) {
        (0, _, _) => "low",           // No rules -- no-op policy
        (_, true, true) => "low",     // Targeted selector with port restrictions
        (_, true, false) => "medium", // Targeted selector, no port restriction
        (_, false, true) => "medium", // Broad selector but ports are restricted
        (_, false, false) => "high",  // Broad selector, no port restriction
    };

    let services_impacted = if has_endpoint_selector {
        std::cmp::max(1, total_rules)
    } else {
        // Namespace-wide -- assume multiple services
        std::cmp::max(total_rules, 3)
    };

    Ok(Json(json!({
        "policy": req.name,
        "namespace": req.namespace,
        "analysis": {
            "ingress_rules": ingress_rules,
            "egress_rules": egress_rules,
            "total_rules": total_rules,
            "has_endpoint_selector": has_endpoint_selector,
            "has_port_restrictions": has_port_rules,
        },
        "impact": {
            "estimated_flows_affected": estimated_affected,
            "services_impacted": services_impacted,
            "risk_level": risk_level,
        }
    })))
}
