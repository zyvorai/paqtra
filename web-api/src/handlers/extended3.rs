use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use std::sync::Arc;
use crate::AppState;
use super::{check_admin, track_request};

#[derive(Debug, Deserialize)]
pub struct CreateMirrorRequest {
    #[serde(default = "default_mirror_name")]
    pub name: String,
}

fn default_mirror_name() -> String { "new-rule".to_string() }

#[derive(Debug, Deserialize)]
pub struct TroubleshootRequest {
    #[serde(default = "default_target")]
    pub target: String,
}

fn default_target() -> String { "cluster".to_string() }

// ── Cost Breakdown ─────────────────────────────────────────

pub async fn cost_breakdown(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
        "namespaces": [
            { "namespace": "production", "cpu_cost": 245.80, "memory_cost": 189.50, "network_cost": 67.20, "storage_cost": 120.00, "total_cost": 622.50, "trend": "+3.2%" },
            { "namespace": "staging", "cpu_cost": 85.40, "memory_cost": 62.30, "network_cost": 18.90, "storage_cost": 45.00, "total_cost": 211.60, "trend": "-1.5%" },
            { "namespace": "monitoring", "cpu_cost": 120.00, "memory_cost": 95.70, "network_cost": 42.10, "storage_cost": 200.00, "total_cost": 457.80, "trend": "+0.8%" },
            { "namespace": "kube-system", "cpu_cost": 55.20, "memory_cost": 38.40, "network_cost": 12.60, "storage_cost": 25.00, "total_cost": 131.20, "trend": "+0.1%" },
            { "namespace": "default", "cpu_cost": 32.10, "memory_cost": 24.80, "network_cost": 8.50, "storage_cost": 15.00, "total_cost": 80.40, "trend": "-0.3%" }
        ],
        "summary": {
            "total_monthly_cost": 1503.50,
            "projected_annual_cost": 18042.00,
            "cost_trend": "+1.8%",
            "currency": "USD",
            "billing_period": "2026-04"
        }
    }))
}

// ── Forecast Metrics ───────────────────────────────────────

pub async fn forecast_metrics(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
        "metrics": ["cpu_usage", "memory_usage", "network_throughput", "pod_count"]
    }))
}

// ── Forecast Data ──────────────────────────────────────────

pub async fn forecast_data(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(metric): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
        "metric": metric,
        "unit": "percent",
        "forecast_horizon": "7d",
        "points": [
            { "timestamp": "2026-04-03T00:00:00Z", "actual": 42.5, "predicted": 43.1, "lower_bound": 38.2, "upper_bound": 48.0 },
            { "timestamp": "2026-04-04T00:00:00Z", "actual": null, "predicted": 44.8, "lower_bound": 39.0, "upper_bound": 50.6 },
            { "timestamp": "2026-04-05T00:00:00Z", "actual": null, "predicted": 46.2, "lower_bound": 39.8, "upper_bound": 52.6 },
            { "timestamp": "2026-04-06T00:00:00Z", "actual": null, "predicted": 45.0, "lower_bound": 38.5, "upper_bound": 51.5 },
            { "timestamp": "2026-04-07T00:00:00Z", "actual": null, "predicted": 47.3, "lower_bound": 40.1, "upper_bound": 54.5 },
            { "timestamp": "2026-04-08T00:00:00Z", "actual": null, "predicted": 48.9, "lower_bound": 41.2, "upper_bound": 56.6 },
            { "timestamp": "2026-04-09T00:00:00Z", "actual": null, "predicted": 50.1, "lower_bound": 42.0, "upper_bound": 58.2 }
        ],
        "model": "arima",
        "confidence_interval": 0.95
    }))
}

// ── Encryption Status ──────────────────────────────────────

pub async fn encryption_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    // Query Cilium ConfigMap for encryption settings
    let cm = state.k8s.kubectl_json(&[
        "get", "configmap", "cilium-config", "-n", "kube-system", "-o", "json",
    ]).await;

    let empty = serde_json::json!({});
    let data = cm.get("data").unwrap_or(&empty);
    let enc_type = data.get("encrypt-node").and_then(|v| v.as_str())
        .or_else(|| data.get("encryption.type").and_then(|v| v.as_str()))
        .unwrap_or("disabled")
        .to_string();
    let enabled = enc_type != "disabled" && !enc_type.is_empty();

    // Count nodes
    let nodes = state.k8s.kubectl_json(&["get", "nodes", "-o", "json"]).await;
    let nodes_total = nodes.get("items").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);

    Json(serde_json::json!({
        "enabled": enabled,
        "type": if enabled { &enc_type } else { "none" },
        "nodes_total": nodes_total,
        "config_source": "cilium-config ConfigMap",
    }))
}

// ── Load Balancer Services ─────────────────────────────────

pub async fn lb_services(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    // Query real K8s Service resources of type LoadBalancer and ClusterIP
    let data = state.k8s.kubectl_json(&[
        "get", "services", "--all-namespaces", "-o", "json",
    ]).await;

    let services: Vec<serde_json::Value> = data
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items.iter().map(|item| {
                let meta = item.get("metadata").unwrap_or(item);
                let spec = item.get("spec").unwrap_or(item);
                let name = meta.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let ns = meta.get("namespace").and_then(|v| v.as_str()).unwrap_or("");
                let svc_type = spec.get("type").and_then(|v| v.as_str()).unwrap_or("ClusterIP");
                let cluster_ip = spec.get("clusterIP").and_then(|v| v.as_str()).unwrap_or("");
                let ports: Vec<serde_json::Value> = spec
                    .get("ports")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let session_affinity = spec.get("sessionAffinity").and_then(|v| v.as_str()).unwrap_or("None");

                serde_json::json!({
                    "name": name,
                    "namespace": ns,
                    "type": svc_type,
                    "cluster_ip": cluster_ip,
                    "ports": ports,
                    "session_affinity": session_affinity,
                })
            }).collect()
        })
        .unwrap_or_default();

    let total = services.len();
    Json(serde_json::json!({ "services": services, "total": total }))
}

// ── Ingress Routes ─────────────────────────────────────────

pub async fn ingress_routes(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    let data = state.k8s.kubectl_json(&[
        "get", "ingress", "--all-namespaces", "-o", "json",
    ]).await;

    let routes: Vec<serde_json::Value> = data
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items.iter().map(|item| {
                let meta = item.get("metadata").unwrap_or(item);
                let spec = item.get("spec").unwrap_or(item);
                let tls = spec.get("tls").cloned().unwrap_or(serde_json::json!([]));
                let rules = spec.get("rules").cloned().unwrap_or(serde_json::json!([]));

                serde_json::json!({
                    "name": meta.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    "namespace": meta.get("namespace").and_then(|v| v.as_str()).unwrap_or(""),
                    "rules": rules,
                    "tls": tls,
                    "created_at": meta.get("creationTimestamp").and_then(|v| v.as_str()).unwrap_or(""),
                })
            }).collect()
        })
        .unwrap_or_default();

    let total = routes.len();
    Json(serde_json::json!({ "routes": routes, "total": total }))
}

// ── IPAM Pools ─────────────────────────────────────────────

pub async fn ipam_pools(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    // Query CiliumNode resources for IPAM pool info
    let data = state.k8s.kubectl_json(&[
        "get", "ciliumnodes", "-o", "json",
    ]).await;

    let pools: Vec<serde_json::Value> = data
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items.iter().filter_map(|item| {
                let meta = item.get("metadata")?;
                let spec = item.get("spec")?;
                let ipam = spec.get("ipam")?;
                let name = meta.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let cidrs: Vec<String> = ipam.get("podCIDRs")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                Some(serde_json::json!({
                    "node": name,
                    "pod_cidrs": cidrs,
                    "ipam": ipam,
                }))
            }).collect()
        })
        .unwrap_or_default();

    Json(serde_json::json!({ "pools": pools, "total": pools.len() }))
}

// ── IP Allocations ─────────────────────────────────────────

pub async fn ip_allocations(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    // Get pod IPs from running pods
    let data = state.k8s.kubectl_json(&[
        "get", "pods", "--all-namespaces", "-o", "json",
    ]).await;

    let allocations: Vec<serde_json::Value> = data
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items.iter().filter_map(|item| {
                let meta = item.get("metadata")?;
                let status = item.get("status")?;
                let pod_ip = status.get("podIP").and_then(|v| v.as_str())?;
                if pod_ip.is_empty() { return None; }
                Some(serde_json::json!({
                    "ip": pod_ip,
                    "pod": meta.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    "namespace": meta.get("namespace").and_then(|v| v.as_str()).unwrap_or(""),
                    "node": status.get("hostIP").and_then(|v| v.as_str()).unwrap_or(""),
                    "phase": status.get("phase").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                }))
            }).collect()
        })
        .unwrap_or_default();

    let total = allocations.len();
    Json(serde_json::json!({ "allocations": allocations, "total": total }))
}

// ── Latency Analysis ───────────────────────────────────────

pub async fn latency_analysis(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    // Derive per-service traffic volume from Hubble flows
    // Note: L4 flows don't include latency; we report flow counts as a traffic volume proxy
    let flows = state.hubble.get_flows(500, None).await.unwrap_or_default();

    let mut svc_flows: std::collections::HashMap<(String, String), (u64, u64)> = std::collections::HashMap::new();
    for flow in &flows {
        let svc = if !flow.destination.pod.is_empty() {
            flow.destination.pod.split('-').take(2).collect::<Vec<_>>().join("-")
        } else { continue; };
        let ns = if flow.destination.namespace.is_empty() { "unknown" } else { &flow.destination.namespace };
        let entry = svc_flows.entry((svc, ns.to_string())).or_default();
        entry.0 += 1; // total
        if flow.verdict == "DROPPED" { entry.1 += 1; } // errors
    }

    let services: Vec<serde_json::Value> = svc_flows.into_iter().map(|((svc, ns), (total, errors))| {
        serde_json::json!({
            "service": svc,
            "namespace": ns,
            "sample_count": total,
            "error_count": errors,
            "error_rate": if total > 0 { errors as f64 / total as f64 * 100.0 } else { 0.0 },
            "note": "Latency requires L7/Prometheus metrics; showing flow volume",
        })
    }).collect();

    Json(serde_json::json!({ "services": services, "source": "hubble L4 flow counts" }))
}

// ── Traffic Mirror Rules ───────────────────────────────────

const MIRROR_RULES_PREFIX: &str = "cv:mirror_rules:";

pub async fn mirror_rules(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let items = state.cache.list_values(MIRROR_RULES_PREFIX).await.unwrap_or_default();
    Json(serde_json::json!({ "rules": items, "total": items.len() }))
}

pub async fn create_mirror_rule(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(body): Json<CreateMirrorRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;

    let id = format!("mirror-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("000"));
    let rule = serde_json::json!({
        "id": id,
        "name": body.name,
        "enabled": true,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    let _ = state.cache.set_persistent(&format!("{}{}", MIRROR_RULES_PREFIX, id), &rule).await;

    Ok(Json(serde_json::json!({
        "id": id,
        "status": "created",
        "message": "Mirror rule created",
    })))
}

pub async fn delete_mirror_rule(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;

    let _ = state.cache.delete(&format!("{}{}", MIRROR_RULES_PREFIX, id)).await;
    Ok(Json(serde_json::json!({
        "id": id,
        "status": "deleted",
        "message": "Mirror rule deleted"
    })))
}

// ── Cluster Health ─────────────────────────────────────────

pub async fn cluster_health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    // Query real component statuses
    let cilium_pods = state.k8s.kubectl_json(&[
        "get", "pods", "-n", "kube-system", "-l", "k8s-app=cilium", "-o", "json",
    ]).await;

    let components: Vec<serde_json::Value> = {
        let mut comps = Vec::new();
        // Check cilium agents
        let agents = cilium_pods.get("items").and_then(|v| v.as_array());
        let (total, ready) = agents.map(|arr| {
            let t = arr.len();
            let r = arr.iter().filter(|p| {
                p.get("status")
                    .and_then(|s| s.get("containerStatuses"))
                    .and_then(|v| v.as_array())
                    .map(|cs| cs.iter().all(|c| c.get("ready").and_then(|v| v.as_bool()).unwrap_or(false)))
                    .unwrap_or(false)
            }).count();
            (t, r)
        }).unwrap_or((0, 0));

        comps.push(serde_json::json!({
            "name": "cilium-agent",
            "status": if total == ready && total > 0 { "healthy" } else { "degraded" },
            "instances": total,
            "ready": ready,
        }));

        // Check hubble-relay
        let relay = state.k8s.kubectl_json(&[
            "get", "pods", "-n", "kube-system", "-l", "k8s-app=hubble-relay", "-o", "json",
        ]).await;
        let relay_items = relay.get("items").and_then(|v| v.as_array());
        let (rt, rr) = relay_items.map(|arr| (arr.len(), arr.iter().filter(|p| {
            p.get("status").and_then(|s| s.get("phase")).and_then(|v| v.as_str()) == Some("Running")
        }).count())).unwrap_or((0, 0));
        comps.push(serde_json::json!({
            "name": "hubble-relay", "instances": rt, "ready": rr,
            "status": if rt == rr && rt > 0 { "healthy" } else if rt > 0 { "degraded" } else { "not_found" },
        }));

        comps
    };

    // K8s health
    let k8s_healthy = state.k8s.is_healthy().await;

    // Node status
    let nodes = state.k8s.kubectl_json(&["get", "nodes", "-o", "json"]).await;
    let node_items = nodes.get("items").and_then(|v| v.as_array());
    let nodes_total = node_items.map(|a| a.len()).unwrap_or(0);
    let nodes_ready = node_items.map(|arr| {
        arr.iter().filter(|n| {
            n.get("status").and_then(|s| s.get("conditions")).and_then(|v| v.as_array())
                .and_then(|conds| conds.iter().find(|c| c.get("type").and_then(|v| v.as_str()) == Some("Ready")))
                .and_then(|c| c.get("status")).and_then(|v| v.as_str()) == Some("True")
        }).count()
    }).unwrap_or(0);

    let overall = if nodes_ready == nodes_total && nodes_total > 0 && k8s_healthy { "healthy" } else { "degraded" };

    Json(serde_json::json!({
        "status": overall,
        "components": components,
        "kubernetes": {
            "healthy": k8s_healthy,
            "nodes_total": nodes_total,
            "nodes_ready": nodes_ready,
        },
        "last_check": chrono::Utc::now().to_rfc3339(),
    }))
}

// ── RBAC Bindings ──────────────────────────────────────────

pub async fn rbac_bindings(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;

    let data = state.k8s.kubectl_json(&[
        "get", "clusterrolebindings", "-o", "json",
        "-l", "app.kubernetes.io/part-of=cilium",
    ]).await;

    let bindings: Vec<serde_json::Value> = data
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items.iter().map(|item| {
                let meta = item.get("metadata").unwrap_or(item);
                let role_ref = item.get("roleRef").unwrap_or(item);
                let subjects: Vec<serde_json::Value> = item
                    .get("subjects")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                serde_json::json!({
                    "name": meta.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    "type": "ClusterRoleBinding",
                    "role": role_ref.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    "role_kind": role_ref.get("kind").and_then(|v| v.as_str()).unwrap_or(""),
                    "subjects": subjects,
                    "created_at": meta.get("creationTimestamp").and_then(|v| v.as_str()).unwrap_or(""),
                })
            }).collect()
        })
        .unwrap_or_default();

    // If no cilium-specific bindings found, get all
    if bindings.is_empty() {
        let all_data = state.k8s.kubectl_json(&[
            "get", "clusterrolebindings", "-o", "json",
        ]).await;
        let all_bindings: Vec<serde_json::Value> = all_data
            .get("items")
            .and_then(|v| v.as_array())
            .map(|items| {
                items.iter().take(20).map(|item| {
                    let meta = item.get("metadata").unwrap_or(item);
                    let role_ref = item.get("roleRef").unwrap_or(item);
                    serde_json::json!({
                        "name": meta.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                        "type": "ClusterRoleBinding",
                        "role": role_ref.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    })
                }).collect()
            })
            .unwrap_or_default();
        return Ok(Json(serde_json::json!({ "bindings": all_bindings, "total": all_bindings.len() })));
    }

    let total = bindings.len();
    Ok(Json(serde_json::json!({ "bindings": bindings, "total": total })))
}

// ── Network Interfaces ─────────────────────────────────────

pub async fn net_interfaces(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;

    // Get network interfaces from local system via /proc/net/dev
    use crate::services::k8s::K8sService;
    let proc_net = K8sService::run_cmd("sh", &["-c", "cat /proc/net/dev"]).await;

    let mut interfaces = Vec::new();
    for line in proc_net.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 11 { continue; }
        let name = parts[0].trim_end_matches(':');
        interfaces.push(serde_json::json!({
            "name": name,
            "rx_bytes": parts[1].parse::<u64>().unwrap_or(0),
            "rx_packets": parts[2].parse::<u64>().unwrap_or(0),
            "rx_errors": parts[3].parse::<u64>().unwrap_or(0),
            "rx_dropped": parts[4].parse::<u64>().unwrap_or(0),
            "tx_bytes": parts[9].parse::<u64>().unwrap_or(0),
            "tx_packets": parts[10].parse::<u64>().unwrap_or(0),
            "tx_errors": parts.get(11).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0),
            "tx_dropped": parts.get(12).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0),
        }));
    }

    Ok(Json(serde_json::json!({ "interfaces": interfaces, "total": interfaces.len(), "source": "local /proc/net/dev" })))
}

// ── Troubleshoot ───────────────────────────────────────────

pub async fn run_troubleshoot(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(body): Json<TroubleshootRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;
    let target = &body.target;

    use crate::services::k8s::K8sService;
    let mut steps = Vec::new();
    let mut passed = 0u32;
    let mut warnings = 0u32;
    let mut failed = 0u32;

    // Step 1: K8s API
    let k8s_ok = state.k8s.is_healthy().await;
    if k8s_ok { passed += 1; } else { failed += 1; }
    steps.push(serde_json::json!({
        "step": 1, "name": "Kubernetes API", "status": if k8s_ok { "pass" } else { "fail" },
        "output": if k8s_ok { "API server reachable" } else { "API server unreachable" },
    }));

    // Step 2: Cilium agents
    let agent_pods = state.k8s.kubectl_json(&["get", "pods", "-n", "kube-system", "-l", "k8s-app=cilium", "-o", "json"]).await;
    let agent_count = agent_pods.get("items").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    if agent_count > 0 { passed += 1; } else { failed += 1; }
    steps.push(serde_json::json!({
        "step": 2, "name": "Cilium Agents", "status": if agent_count > 0 { "pass" } else { "fail" },
        "output": format!("{} cilium agent pods found", agent_count),
    }));

    // Step 3: Hubble
    let hubble_ok = state.hubble.is_healthy().await;
    if hubble_ok { passed += 1; } else { warnings += 1; }
    steps.push(serde_json::json!({
        "step": 3, "name": "Hubble Relay", "status": if hubble_ok { "pass" } else { "warn" },
        "output": if hubble_ok { format!("Connected to {}", state.hubble.address()) } else { "Relay unreachable".to_string() },
    }));

    // Step 4: Network policies
    let policies = state.k8s.list_policies().await.unwrap_or_default();
    if !policies.is_empty() { passed += 1; } else { warnings += 1; }
    steps.push(serde_json::json!({
        "step": 4, "name": "Network Policies", "status": if !policies.is_empty() { "pass" } else { "warn" },
        "output": format!("{} CiliumNetworkPolicies active", policies.len()),
    }));

    // Step 5: BPF filesystem
    let bpf_mount = K8sService::run_cmd("sh", &["-c", "mountpoint -q /sys/fs/bpf && echo yes || echo no"]).await;
    let bpf_ok = bpf_mount.trim() == "yes";
    if bpf_ok { passed += 1; } else { warnings += 1; }
    steps.push(serde_json::json!({
        "step": 5, "name": "BPF Filesystem", "status": if bpf_ok { "pass" } else { "warn" },
        "output": if bpf_ok { "/sys/fs/bpf mounted" } else { "/sys/fs/bpf not available" },
    }));

    // Step 6: Flows check
    let flows = state.hubble.get_flows(100, None).await.unwrap_or_default();
    let drops = flows.iter().filter(|f| f.verdict == "DROPPED").count();
    if drops == 0 { passed += 1; } else { warnings += 1; }
    steps.push(serde_json::json!({
        "step": 6, "name": "Recent Traffic Health", "status": if drops == 0 { "pass" } else { "warn" },
        "output": format!("{} flows observed, {} drops", flows.len(), drops),
    }));

    let total_steps = steps.len() as u32;
    Ok(Json(serde_json::json!({
        "target": target,
        "started_at": chrono::Utc::now().to_rfc3339(),
        "status": "completed",
        "steps": steps,
        "summary": {
            "total_steps": total_steps,
            "passed": passed,
            "warnings": warnings,
            "failed": failed,
        }
    })))
}
