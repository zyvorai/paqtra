use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use crate::AppState;
use super::track_request;

#[derive(Debug, Clone, Serialize)]
pub struct CiliumEndpoint {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub identity: u32,
    pub ipv4: String,
    pub ipv6: String,
    pub status: String,
    pub policy_enforcement: String,
    pub labels: Vec<String>,
}

pub async fn list_endpoints(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |m| { m.k8s_queries.fetch_add(1, Ordering::Relaxed); }).await;

    let endpoints = vec![
        CiliumEndpoint {
            id: "ep-001".into(), name: "frontend-abc123".into(), namespace: "default".into(),
            identity: 12345, ipv4: "10.0.1.15".into(), ipv6: "fd00::1:f".into(),
            status: "ready".into(), policy_enforcement: "default".into(),
            labels: vec!["app=frontend".into(), "version=v2".into()],
        },
        CiliumEndpoint {
            id: "ep-002".into(), name: "backend-xyz789".into(), namespace: "default".into(),
            identity: 12346, ipv4: "10.0.1.20".into(), ipv6: "fd00::1:14".into(),
            status: "ready".into(), policy_enforcement: "always".into(),
            labels: vec!["app=backend".into(), "version=v1".into()],
        },
        CiliumEndpoint {
            id: "ep-003".into(), name: "redis-master-0".into(), namespace: "default".into(),
            identity: 12347, ipv4: "10.0.2.5".into(), ipv6: "fd00::2:5".into(),
            status: "ready".into(), policy_enforcement: "always".into(),
            labels: vec!["app=redis".into(), "role=master".into()],
        },
        CiliumEndpoint {
            id: "ep-004".into(), name: "coredns-abc".into(), namespace: "kube-system".into(),
            identity: 10001, ipv4: "10.0.0.10".into(), ipv6: "fd00::a".into(),
            status: "ready".into(), policy_enforcement: "default".into(),
            labels: vec!["k8s-app=kube-dns".into()],
        },
        CiliumEndpoint {
            id: "ep-005".into(), name: "cilium-agent-node1".into(), namespace: "kube-system".into(),
            identity: 1, ipv4: "10.0.0.1".into(), ipv6: "fd00::1".into(),
            status: "ready".into(), policy_enforcement: "default".into(),
            labels: vec!["k8s-app=cilium".into(), "reserved:host".into()],
        },
    ];

    Json(serde_json::json!({ "endpoints": endpoints, "total": endpoints.len() }))
}
