// Anomaly detection endpoints
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;
use super::{track_request, to_json};

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

pub async fn list_anomalies(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    track_request(&state, |_| {}).await;

    // Return typed but empty list -- anomaly detection engine not yet integrated
    let anomalies: Vec<Anomaly> = Vec::new();

    Ok(Json(json!({
        "anomalies": anomalies,
        "total": 0,
        "detection_engine": "not_connected",
        "message": "Anomaly detection requires ML pipeline integration"
    })))
}

pub async fn get_anomaly(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    track_request(&state, |_| {}).await;

    // Would look up from anomaly store; for now return typed not-found
    Ok(Json(json!({
        "error": "not_found",
        "id": id,
        "message": "Anomaly not found or detection engine not connected"
    })))
}

pub async fn remediate_anomaly(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    track_request(&state, |_| {}).await;

    let result = RemediationResult {
        id: id.clone(),
        status: "pending".to_string(),
        action_taken: "none".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(to_json(&result)))
}
