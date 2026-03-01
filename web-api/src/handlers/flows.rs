// Flow monitoring endpoints
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;
use crate::models::flow::{Flow, FlowEndpoint, FlowQueryParams};

pub async fn list_flows(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<FlowQueryParams>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Fetching flows with params: {:?}", params);

    // TODO: Integrate with Cilium Vision core to get flows
    let flows = vec![
        Flow {
            id: "flow-1".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            source: FlowEndpoint {
                namespace: "default".to_string(),
                pod: "frontend-7d4b9c".to_string(),
                ip: "10.0.1.5".to_string(),
            },
            destination: FlowEndpoint {
                namespace: "default".to_string(),
                pod: "backend-9f8a2b".to_string(),
                ip: "10.0.1.10".to_string(),
            },
            verdict: "FORWARDED".to_string(),
            protocol: "TCP".to_string(),
            port: 8080,
        },
    ];

    Ok(Json(json!({
        "flows": flows,
        "total": 1,
        "limit": params.limit.unwrap_or(100),
        "offset": params.offset.unwrap_or(0),
    })))
}

pub async fn get_flow(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Fetching flow: {}", id);

    // TODO: Get specific flow from Hubble
    Ok(Json(json!({
        "id": id,
        "message": "Flow details would go here"
    })))
}

pub async fn flow_stats(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    // TODO: Calculate flow statistics from Hubble
    Ok(Json(json!({
        "total_flows": 0,
        "forwarded": 0,
        "dropped": 0,
        "requests_per_second": 0,
        "avg_latency_ms": 0,
        "note": "Connect to Hubble for real statistics"
    })))
}
