use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;
use crate::AppState;
use super::track_request;

#[derive(Debug, Clone, Serialize)]
pub struct K8sEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub reason: String,
    pub object: String,
    pub message: String,
    pub namespace: String,
    pub count: u32,
    pub first_seen: String,
    pub last_seen: String,
}

pub async fn list_events(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |m| m.k8s_queries += 1).await;

    let events = vec![
        K8sEvent {
            id: "evt-001".into(), event_type: "Normal".into(), reason: "Scheduled".into(),
            object: "pod/frontend-abc123".into(), message: "Successfully assigned default/frontend-abc123 to node-1".into(),
            namespace: "default".into(), count: 1, first_seen: "2026-04-03T10:00:00Z".into(), last_seen: "2026-04-03T10:00:00Z".into(),
        },
        K8sEvent {
            id: "evt-002".into(), event_type: "Warning".into(), reason: "FailedMount".into(),
            object: "pod/backend-xyz789".into(), message: "MountVolume.SetUp failed for volume \"config\"".into(),
            namespace: "default".into(), count: 3, first_seen: "2026-04-03T09:55:00Z".into(), last_seen: "2026-04-03T10:01:00Z".into(),
        },
        K8sEvent {
            id: "evt-003".into(), event_type: "Normal".into(), reason: "Pulled".into(),
            object: "pod/monitoring-prom-0".into(), message: "Container image \"prom/prometheus:v2.50\" already present".into(),
            namespace: "monitoring".into(), count: 1, first_seen: "2026-04-03T09:50:00Z".into(), last_seen: "2026-04-03T09:50:00Z".into(),
        },
        K8sEvent {
            id: "evt-004".into(), event_type: "Warning".into(), reason: "Unhealthy".into(),
            object: "pod/api-gateway-def456".into(), message: "Readiness probe failed: connection refused".into(),
            namespace: "default".into(), count: 5, first_seen: "2026-04-03T09:45:00Z".into(), last_seen: "2026-04-03T10:02:00Z".into(),
        },
        K8sEvent {
            id: "evt-005".into(), event_type: "Normal".into(), reason: "CiliumEndpointUpdated".into(),
            object: "ciliumendpoint/frontend-abc123".into(), message: "Endpoint identity updated to 12345".into(),
            namespace: "default".into(), count: 2, first_seen: "2026-04-03T10:00:00Z".into(), last_seen: "2026-04-03T10:00:30Z".into(),
        },
    ];

    Json(serde_json::json!({ "events": events, "total": events.len() }))
}
