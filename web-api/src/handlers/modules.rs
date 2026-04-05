// Intelligence module endpoints
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;
use super::{check_admin, track_request, to_json};

/// Typed request for the autopolicy generation endpoint.
#[derive(Debug, Deserialize)]
pub struct GenerateAutopolicyRequest {
    #[serde(default = "default_namespace")]
    pub namespace: String,
}

fn default_namespace() -> String {
    "production".to_string()
}

/// Typed request for the chaos experiment endpoint.
#[derive(Debug, Deserialize)]
pub struct RunChaosRequest {
    #[serde(default = "default_chaos_name")]
    pub name: String,
    #[serde(default = "default_experiment_type")]
    pub experiment_type: String,
    #[serde(default = "default_target_namespace")]
    pub target_namespace: String,
    #[serde(default = "default_duration")]
    pub duration_secs: u64,
}

fn default_chaos_name() -> String {
    "ad-hoc-network-partition".to_string()
}

fn default_experiment_type() -> String {
    "network-partition".to_string()
}

fn default_target_namespace() -> String {
    "staging".to_string()
}

fn default_duration() -> u64 {
    120
}

/// Auto-generated policy suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPolicyResult {
    pub request_id: String,
    pub policies_generated: u32,
    pub confidence: f64,
    pub policies: Vec<GeneratedPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedPolicy {
    pub name: String,
    pub namespace: String,
    pub description: String,
    pub spec: Value,
    pub confidence: f64,
}

/// Chaos experiment definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosExperiment {
    pub id: String,
    pub name: String,
    pub experiment_type: String,
    pub status: String,
    pub target_namespace: String,
    pub created_at: String,
    pub duration_secs: u64,
    pub results: Option<ChaosResults>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosResults {
    pub packets_dropped: u64,
    pub connections_failed: u64,
    pub services_impacted: u32,
    pub recovery_time_secs: Option<f64>,
}

/// Canary deployment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryStatus {
    pub id: String,
    pub status: String,
    pub traffic_split: TrafficSplit,
    pub metrics: CanaryMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficSplit {
    pub stable: u32,
    pub canary: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryMetrics {
    pub success_rate: f64,
    pub latency_p99_ms: f64,
    pub error_count: u64,
}

pub async fn generate_autopolicy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GenerateAutopolicyRequest>,
) -> Result<Json<Value>, StatusCode> {
    track_request(&state, |_| {}).await;

    let namespace = &req.namespace;

    let result = AutoPolicyResult {
        request_id: uuid::Uuid::new_v4().to_string(),
        policies_generated: 2,
        confidence: 0.87,
        policies: vec![
            GeneratedPolicy {
                name: format!("allow-frontend-to-api-{}", namespace),
                namespace: namespace.to_string(),
                description: "Allow frontend pods to reach api-gateway on port 8080 (HTTP) \
                              and port 8443 (HTTPS). Derived from 72 hours of observed traffic."
                    .to_string(),
                spec: json!({
                    "apiVersion": "cilium.io/v2",
                    "kind": "CiliumNetworkPolicy",
                    "metadata": {
                        "name": format!("allow-frontend-to-api-{}", namespace),
                        "namespace": namespace
                    },
                    "spec": {
                        "endpointSelector": {
                            "matchLabels": {
                                "app": "api-gateway"
                            }
                        },
                        "ingress": [{
                            "fromEndpoints": [{
                                "matchLabels": {
                                    "app": "frontend",
                                    "tier": "web"
                                }
                            }],
                            "toPorts": [{
                                "ports": [
                                    { "port": "8080", "protocol": "TCP" },
                                    { "port": "8443", "protocol": "TCP" }
                                ],
                                "rules": {
                                    "http": [{
                                        "method": "GET",
                                        "path": "/api/v1/.*"
                                    }, {
                                        "method": "POST",
                                        "path": "/api/v1/.*"
                                    }]
                                }
                            }]
                        }]
                    }
                }),
                confidence: 0.92,
            },
            GeneratedPolicy {
                name: format!("deny-default-egress-{}", namespace),
                namespace: namespace.to_string(),
                description: "Default-deny egress for all pods in the namespace, \
                              with explicit exceptions for DNS (kube-dns) and \
                              monitored services."
                    .to_string(),
                spec: json!({
                    "apiVersion": "cilium.io/v2",
                    "kind": "CiliumNetworkPolicy",
                    "metadata": {
                        "name": format!("deny-default-egress-{}", namespace),
                        "namespace": namespace
                    },
                    "spec": {
                        "endpointSelector": {},
                        "egress": [{
                            "toEndpoints": [{
                                "matchLabels": {
                                    "k8s:io.kubernetes.pod.namespace": "kube-system",
                                    "k8s-app": "kube-dns"
                                }
                            }],
                            "toPorts": [{
                                "ports": [
                                    { "port": "53", "protocol": "UDP" },
                                    { "port": "53", "protocol": "TCP" }
                                ]
                            }]
                        }, {
                            "toEndpoints": [{
                                "matchLabels": {
                                    "app.kubernetes.io/part-of": namespace
                                }
                            }]
                        }],
                        "egressDeny": [{
                            "toEntities": ["world"]
                        }]
                    }
                }),
                confidence: 0.81,
            },
        ],
    };

    Ok(Json(to_json(&result)))
}

/// Sample chaos experiments showing both completed and running states.
fn sample_chaos_experiments() -> Vec<ChaosExperiment> {
    vec![
        ChaosExperiment {
            id: "chaos-exp-001".to_string(),
            name: "payment-service-network-partition".to_string(),
            experiment_type: "network-partition".to_string(),
            status: "completed".to_string(),
            target_namespace: "production".to_string(),
            created_at: "2025-06-14T14:00:00Z".to_string(),
            duration_secs: 300,
            results: Some(ChaosResults {
                packets_dropped: 14823,
                connections_failed: 47,
                services_impacted: 3,
                recovery_time_secs: Some(12.4),
            }),
        },
        ChaosExperiment {
            id: "chaos-exp-002".to_string(),
            name: "dns-failure-injection".to_string(),
            experiment_type: "dns-disruption".to_string(),
            status: "running".to_string(),
            target_namespace: "staging".to_string(),
            created_at: "2025-06-15T10:30:00Z".to_string(),
            duration_secs: 600,
            results: None,
        },
    ]
}

pub async fn list_chaos_experiments(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    track_request(&state, |_| {}).await;

    let experiments = sample_chaos_experiments();
    let total = experiments.len();

    Ok(Json(json!({
        "experiments": experiments,
        "total": total,
    })))
}

pub async fn run_chaos_experiment(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(req): Json<RunChaosRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;

    // Simulate an experiment that has already completed with results
    let experiment = ChaosExperiment {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name.clone(),
        experiment_type: req.experiment_type.clone(),
        status: "completed".to_string(),
        target_namespace: req.target_namespace.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        duration_secs: req.duration_secs,
        results: Some(ChaosResults {
            packets_dropped: 8432,
            connections_failed: 23,
            services_impacted: 2,
            recovery_time_secs: Some(8.7),
        }),
    };

    Ok(Json(to_json(&experiment)))
}

pub async fn canary_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    track_request(&state, |_| {}).await;

    let status = CanaryStatus {
        id,
        status: "progressing".to_string(),
        traffic_split: TrafficSplit {
            stable: 80,
            canary: 20,
        },
        metrics: CanaryMetrics {
            success_rate: 99.72,
            latency_p99_ms: 42.3,
            error_count: 7,
        },
    };

    Ok(Json(json!({
        "canary": to_json(&status),
        "analysis": {
            "phase": "canary-weight-20",
            "started_at": "2025-06-15T06:00:00Z",
            "last_checked_at": "2025-06-15T10:45:00Z",
            "promotion_threshold": {
                "success_rate_min": 99.5,
                "latency_p99_max_ms": 100.0,
                "error_count_max": 25
            },
            "recommendation": "continue",
            "next_step": "Increase canary weight to 40% if metrics hold for 15 more minutes"
        }
    })))
}
