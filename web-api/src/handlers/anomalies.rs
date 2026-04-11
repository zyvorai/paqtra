// Anomaly detection endpoints
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;
use super::{check_admin, track_request, to_json};

/// Query parameters for paginated anomaly listings.
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Anomaly severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// A detected network anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub id: String,
    pub detected_at: String,
    pub severity: Severity,
    pub anomaly_type: String,
    pub description: String,
    pub source_namespace: String,
    pub source_pod: Option<String>,
    pub destination_namespace: Option<String>,
    pub destination_pod: Option<String>,
    pub status: String,
    pub remediation: Option<String>,
}

/// Result of a remediation action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationResult {
    pub id: String,
    pub status: String,
    pub action_taken: String,
    pub timestamp: String,
}

/// Build sample anomalies that demonstrate the full API structure.
fn sample_anomalies() -> Vec<Anomaly> {
    vec![
        Anomaly {
            id: "anom-001".to_string(),
            detected_at: "2025-06-15T08:23:41Z".to_string(),
            severity: Severity::High,
            anomaly_type: "traffic_spike".to_string(),
            description: "Unexpected 12x traffic increase from frontend to payment-service on port 443. \
                          Baseline: ~200 req/s, observed: ~2400 req/s over a 5-minute window."
                .to_string(),
            source_namespace: "production".to_string(),
            source_pod: Some("frontend-7b4d6f8c9-xk2nl".to_string()),
            destination_namespace: Some("production".to_string()),
            destination_pod: Some("payment-service-5c8f9d4b7-m9pqr".to_string()),
            status: "active".to_string(),
            remediation: Some("Rate-limit rule applied via CiliumNetworkPolicy".to_string()),
        },
        Anomaly {
            id: "anom-002".to_string(),
            detected_at: "2025-06-15T09:01:17Z".to_string(),
            severity: Severity::Critical,
            anomaly_type: "port_scan".to_string(),
            description: "Sequential connection attempts to ports 22, 80, 443, 3306, 5432, 6379, 8080, 9090 \
                          detected from a single pod within 30 seconds. Matches known reconnaissance pattern."
                .to_string(),
            source_namespace: "default".to_string(),
            source_pod: Some("debug-tools-6f7a8b9c0-zz1ab".to_string()),
            destination_namespace: Some("kube-system".to_string()),
            destination_pod: None,
            status: "investigating".to_string(),
            remediation: None,
        },
        Anomaly {
            id: "anom-003".to_string(),
            detected_at: "2025-06-15T07:45:02Z".to_string(),
            severity: Severity::Medium,
            anomaly_type: "latency_increase".to_string(),
            description: "P99 latency between api-gateway and inventory-service rose from 45ms to 320ms. \
                          Correlates with increased DNS resolution failures in the same namespace."
                .to_string(),
            source_namespace: "production".to_string(),
            source_pod: Some("api-gateway-3a4b5c6d7-h8ijk".to_string()),
            destination_namespace: Some("production".to_string()),
            destination_pod: Some("inventory-service-9e0f1a2b3-c4def".to_string()),
            status: "resolved".to_string(),
            remediation: Some("CoreDNS cache TTL increased; pod restarted to clear stale connections".to_string()),
        },
    ]
}

pub async fn list_anomalies(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, StatusCode> {
    track_request(&state, |_| {}).await;

    let anomalies = sample_anomalies();
    let total = anomalies.len();
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(50).min(1000);
    let page: Vec<_> = anomalies.into_iter().skip(offset).take(limit).collect();

    Ok(Json(json!({
        "anomalies": page,
        "total": total,
        "limit": limit,
        "offset": offset,
        "detection_engine": "cilium-vision-ml",
        "engine_version": "0.4.1",
        "detection_window_secs": 300,
    })))
}

pub async fn get_anomaly(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    track_request(&state, |_| {}).await;

    // Look up the anomaly by ID in the sample set
    if let Some(anomaly) = sample_anomalies().into_iter().find(|a| a.id == id) {
        Ok(Json(to_json(&anomaly)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn remediate_anomaly(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    check_admin(&state, &claims).map_err(|_| StatusCode::FORBIDDEN)?;
    track_request(&state, |_| {}).await;

    // Check whether the anomaly exists in our sample set
    let anomaly = sample_anomalies().into_iter().find(|a| a.id == id);

    let (status, action_taken) = match anomaly.as_ref().map(|a| a.anomaly_type.as_str()) {
        Some("traffic_spike") => (
            "applied".to_string(),
            "CiliumNetworkPolicy rate-limit rule deployed to namespace production; \
             ingress bandwidth capped at 500 req/s for source pod frontend-7b4d6f8c9-xk2nl"
                .to_string(),
        ),
        Some("port_scan") => (
            "applied".to_string(),
            "CiliumNetworkPolicy egress deny rule created for pod debug-tools-6f7a8b9c0-zz1ab; \
             all outbound traffic blocked pending investigation"
                .to_string(),
        ),
        Some("latency_increase") => (
            "already_resolved".to_string(),
            "Anomaly was previously resolved; no additional action required".to_string(),
        ),
        _ => (
            "not_found".to_string(),
            format!("No anomaly with id '{}' found; no action taken", id),
        ),
    };

    let result = RemediationResult {
        id: id.clone(),
        status,
        action_taken,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(to_json(&result)))
}
