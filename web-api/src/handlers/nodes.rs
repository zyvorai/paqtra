use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;
use crate::AppState;
use super::track_request;

#[derive(Debug, Clone, Serialize)]
pub struct K8sNode {
    pub name: String,
    pub status: String,
    pub roles: Vec<String>,
    pub version: String,
    pub os: String,
    pub kernel: String,
    pub cpu_capacity: u32,
    pub cpu_usage: f64,
    pub memory_capacity_gb: f64,
    pub memory_usage_gb: f64,
    pub pods_count: u32,
    pub pods_capacity: u32,
    pub age: String,
}

pub async fn list_nodes(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |m| m.k8s_queries += 1).await;

    let nodes = vec![
        K8sNode {
            name: "node-1".into(), status: "Ready".into(), roles: vec!["control-plane".into(), "master".into()],
            version: "v1.29.2".into(), os: "Ubuntu 22.04".into(), kernel: "6.5.0-35-generic".into(),
            cpu_capacity: 8, cpu_usage: 2.4, memory_capacity_gb: 32.0, memory_usage_gb: 18.5,
            pods_count: 42, pods_capacity: 110, age: "45d".into(),
        },
        K8sNode {
            name: "node-2".into(), status: "Ready".into(), roles: vec!["worker".into()],
            version: "v1.29.2".into(), os: "Ubuntu 22.04".into(), kernel: "6.5.0-35-generic".into(),
            cpu_capacity: 16, cpu_usage: 8.7, memory_capacity_gb: 64.0, memory_usage_gb: 42.3,
            pods_count: 78, pods_capacity: 110, age: "45d".into(),
        },
        K8sNode {
            name: "node-3".into(), status: "Ready".into(), roles: vec!["worker".into()],
            version: "v1.29.2".into(), os: "Ubuntu 22.04".into(), kernel: "6.5.0-35-generic".into(),
            cpu_capacity: 16, cpu_usage: 5.2, memory_capacity_gb: 64.0, memory_usage_gb: 28.1,
            pods_count: 55, pods_capacity: 110, age: "30d".into(),
        },
    ];

    Json(serde_json::json!({ "nodes": nodes }))
}
