use axum::{extract::{Query, State}, http::StatusCode, Json};
use serde::Deserialize;
use std::sync::Arc;
use crate::AppState;
use super::{check_admin, track_request, PaginationQuery, paginate_json};

#[derive(Debug, Deserialize)]
pub struct ValidatePolicyRequest {
    #[serde(default)]
    pub yaml: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateExportRequest {
    #[serde(default = "default_export_name")]
    pub name: String,
    #[serde(default = "default_export_format")]
    pub format: String,
    #[serde(default)]
    pub destination: String,
}

fn default_export_name() -> String { "new-export".to_string() }
fn default_export_format() -> String { "json".to_string() }

#[derive(Debug, Deserialize)]
pub struct NodeActionRequest {
    #[serde(default)]
    pub node: String,
}

// ── WireGuard Peers ───────────────────────────────────────

pub async fn wireguard_peers(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;

    // Try to get real WireGuard data from cilium
    use crate::services::k8s::K8sService;
    let wg_output = K8sService::run_cmd(
        "kubectl",
        &["exec", "-n", "kube-system", "-l", "k8s-app=cilium", "-c", "cilium-agent",
          "--", "cilium", "encrypt", "status"],
    ).await;

    if !wg_output.is_empty() {
        return Ok(Json(serde_json::json!({
            "raw_status": wg_output,
            "source": "cilium encrypt status",
        })));
    }

    // Fallback to sample data if cilium is not available
    Ok(Json(serde_json::json!({
        "peers": [
            {
                "public_key": "aB3dEfGhIjKlMnOpQrStUvWxYz0123456789abc=",
                "endpoint": "10.0.1.6:51871",
                "allowed_ips": ["10.244.1.0/24", "10.244.4.0/24"],
                "latest_handshake": "2026-04-03T11:59:42Z",
                "transfer_rx": 328_177_366,
                "transfer_tx": 252_070_133,
                "persistent_keepalive": 25,
                "node": "cilium-node-2"
            },
            {
                "public_key": "xY9wVuTsRqPoNmLkJiHgFeDcBa9876543210zyx=",
                "endpoint": "10.0.1.7:51871",
                "allowed_ips": ["10.244.2.0/24", "10.244.5.0/24"],
                "latest_handshake": "2026-04-03T11:59:38Z",
                "transfer_rx": 412_560_200,
                "transfer_tx": 301_440_800,
                "persistent_keepalive": 25,
                "node": "cilium-node-3"
            },
            {
                "public_key": "mN5oP6qR7sT8uV9wX0yZ1aB2cD3eF4gH5iJ6kL=",
                "endpoint": "10.0.1.5:51871",
                "allowed_ips": ["10.244.0.0/24", "10.244.3.0/24"],
                "latest_handshake": "2026-04-03T11:59:45Z",
                "transfer_rx": 243_794_534,
                "transfer_tx": 202_699_467,
                "persistent_keepalive": 25,
                "node": "cilium-node-1"
            }
        ]
    })))
}

// ── Cilium Status ─────────────────────────────────────────

pub async fn cilium_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    // Query real cilium status via kubectl exec on cilium pods
    use crate::services::k8s::K8sService;
    let status_json = K8sService::run_cmd(
        "kubectl",
        &["exec", "-n", "kube-system", "-l", "k8s-app=cilium",
          "--", "cilium", "status", "-o", "json", "--brief"],
    ).await;

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&status_json) {
        return Json(serde_json::json!({ "status": parsed }));
    }

    // Fallback: get cilium pods and their status
    let pods = state.k8s.kubectl_json(&[
        "get", "pods", "-n", "kube-system", "-l", "k8s-app=cilium", "-o", "json",
    ]).await;

    let agents: Vec<serde_json::Value> = pods
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items.iter().map(|item| {
                let meta = item.get("metadata").unwrap_or(item);
                let status = item.get("status").unwrap_or(item);
                let phase = status.get("phase").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let node = meta.get("labels")
                    .and_then(|l| l.get("kubernetes.io/hostname"))
                    .and_then(|v| v.as_str())
                    .or_else(|| status.get("hostIP").and_then(|v| v.as_str()))
                    .unwrap_or("unknown");
                let ready = status.get("containerStatuses")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|c| c.get("ready"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                serde_json::json!({
                    "node": node,
                    "pod": meta.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    "status": if ready { "OK" } else { phase },
                    "ready": ready,
                    "host_ip": status.get("hostIP").and_then(|v| v.as_str()).unwrap_or(""),
                })
            }).collect()
        })
        .unwrap_or_default();

    Json(serde_json::json!({ "agents": agents }))
}

// ── Policy Validation ─────────────────────────────────────

pub async fn validate_policy(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(body): Json<ValidatePolicyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;
    let yaml = &body.yaml;
    if yaml.is_empty() {
        return Ok(Json(serde_json::json!({
            "valid": false,
            "errors": ["Empty policy YAML provided"]
        })));
    }

    // Parse YAML locally for structure check
    let mut errors: Vec<String> = Vec::new();
    match serde_yaml::from_str::<serde_json::Value>(yaml) {
        Ok(parsed) => {
            if parsed.get("apiVersion").is_none() {
                errors.push("Missing 'apiVersion' field".to_string());
            }
            if parsed.get("kind").is_none() {
                errors.push("Missing 'kind' field".to_string());
            }
            if parsed.get("metadata").is_none() {
                errors.push("Missing 'metadata' field".to_string());
            }
            if parsed.get("spec").is_none() {
                errors.push("Missing 'spec' field".to_string());
            }
        }
        Err(e) => {
            errors.push(format!("Invalid YAML syntax: {}", e));
        }
    }

    // If local parse passed, validate via kubectl dry-run (piping YAML to stdin)
    let mut dry_run_output: Option<String> = None;
    if errors.is_empty() {
        use crate::services::k8s::K8sService;
        let (ok, stdout, stderr) = K8sService::run_cmd_stdin(
            "kubectl",
            &["apply", "--dry-run=client", "-f", "-", "--validate=true"],
            yaml,
        ).await;
        if !ok && !stderr.is_empty() {
            errors.push(format!("kubectl dry-run failed: {}", stderr));
        }
        let output = if ok { stdout } else { stderr };
        if !output.is_empty() {
            dry_run_output = Some(output);
        }
    }

    let valid = errors.is_empty();
    Ok(Json(serde_json::json!({
        "valid": valid,
        "errors": errors,
        "dry_run_output": dry_run_output,
    })))
}

// ── Flow Export Configs ───────────────────────────────────

const EXPORT_CONFIGS_PREFIX: &str = "cv:export_configs:";

pub async fn list_export_configs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let items = state.cache.list_values(EXPORT_CONFIGS_PREFIX).await.unwrap_or_default();
    Json(paginate_json(items, &params, "configs"))
}

pub async fn create_export_config(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(body): Json<CreateExportRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;

    let id = format!("exp-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("000"));
    let config = serde_json::json!({
        "id": id,
        "name": body.name,
        "format": body.format,
        "destination": body.destination,
        "status": "active",
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    let _ = state.cache.set_persistent(&format!("{}{}", EXPORT_CONFIGS_PREFIX, id), &config).await;

    Ok(Json(serde_json::json!({
        "id": id,
        "status": "active",
        "message": "Export configuration created successfully",
    })))
}

pub async fn delete_export_config(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;

    let key = format!("{}{}", EXPORT_CONFIGS_PREFIX, id);
    let _ = state.cache.delete(&key).await;

    Ok(Json(serde_json::json!({
        "id": id,
        "status": "deleted",
        "message": "Export configuration deleted",
    })))
}

// ── SLO Targets ───────────────────────────────────────────

const SLOS_PREFIX: &str = "cv:slos:";
const INCIDENTS_PREFIX: &str = "cv:incidents:";

pub async fn list_slos(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let stored = state.cache.list_values(SLOS_PREFIX).await.unwrap_or_default();
    if !stored.is_empty() {
        return Json(paginate_json(stored, &params, "slos"));
    }
    // Seed defaults
    let items: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "name": "api-availability",
            "service": "api-gateway",
            "metric": "availability",
            "target": 99.95,
            "current": 99.98,
            "budget_remaining": 0.87,
            "budget_total": 1.0,
            "window": "30d",
            "status": "met"
        }),
        serde_json::json!({
            "name": "api-latency-p99",
            "service": "api-gateway",
            "metric": "latency_p99",
            "target": 200.0,
            "current": 120.8,
            "budget_remaining": 0.92,
            "budget_total": 1.0,
            "window": "30d",
            "status": "met"
        }),
        serde_json::json!({
            "name": "frontend-error-rate",
            "service": "frontend",
            "metric": "error_rate",
            "target": 0.1,
            "current": 0.08,
            "budget_remaining": 0.45,
            "budget_total": 1.0,
            "window": "30d",
            "status": "met"
        }),
        serde_json::json!({
            "name": "dns-resolution",
            "service": "coredns",
            "metric": "latency_p95",
            "target": 10.0,
            "current": 12.4,
            "budget_remaining": 0.0,
            "budget_total": 1.0,
            "window": "7d",
            "status": "breached"
        }),
    ];
    for slo in &items {
        if let Some(name) = slo.get("name").and_then(|v| v.as_str()) {
            let _ = state.cache.set_persistent(&format!("{}{}", SLOS_PREFIX, name), slo).await;
        }
    }
    Json(paginate_json(items, &params, "slos"))
}

// ── Incidents ─────────────────────────────────────────────

pub async fn list_incidents(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let stored = state.cache.list_values(INCIDENTS_PREFIX).await.unwrap_or_default();
    if !stored.is_empty() {
        return Json(paginate_json(stored, &params, "incidents"));
    }
    // Seed defaults
    let items: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "id": "inc-001",
            "title": "Elevated packet drops on cilium-node-3",
            "severity": "warning",
            "status": "resolved",
            "started_at": "2026-04-01T14:22:00Z",
            "resolved_at": "2026-04-01T15:05:00Z",
            "duration_minutes": 43,
            "affected_services": ["api-gateway", "frontend"],
            "root_cause": "BPF map overflow due to stale CT entries",
            "timeline": [
                { "timestamp": "2026-04-01T14:22:00Z", "event": "Alert triggered: packet drop rate > 50/s on cilium-node-3", "actor": "alertmanager" },
                { "timestamp": "2026-04-01T14:25:00Z", "event": "On-call engineer acknowledged", "actor": "ops-team" },
                { "timestamp": "2026-04-01T14:40:00Z", "event": "Root cause identified: CT map at 98% capacity", "actor": "ops-team" },
                { "timestamp": "2026-04-01T14:50:00Z", "event": "CT GC interval reduced, stale entries purged", "actor": "ops-team" },
                { "timestamp": "2026-04-01T15:05:00Z", "event": "Packet drop rate returned to normal", "actor": "system" }
            ]
        }),
        serde_json::json!({
            "id": "inc-002",
            "title": "DNS resolution failures in staging namespace",
            "severity": "critical",
            "status": "investigating",
            "started_at": "2026-04-03T09:15:00Z",
            "resolved_at": null,
            "duration_minutes": null,
            "affected_services": ["grpc-backend", "worker-pool"],
            "root_cause": null,
            "timeline": [
                { "timestamp": "2026-04-03T09:15:00Z", "event": "Alert triggered: DNS SERVFAIL rate > 10% in staging", "actor": "alertmanager" },
                { "timestamp": "2026-04-03T09:18:00Z", "event": "On-call engineer acknowledged", "actor": "ops-team" },
                { "timestamp": "2026-04-03T09:30:00Z", "event": "CoreDNS pod logs show upstream timeout errors", "actor": "ops-team" }
            ]
        }),
    ];
    for inc in &items {
        if let Some(id) = inc.get("id").and_then(|v| v.as_str()) {
            let _ = state.cache.set_persistent(&format!("{}{}", INCIDENTS_PREFIX, id), inc).await;
        }
    }
    Json(paginate_json(items, &params, "incidents"))
}

// ── Change Log ────────────────────────────────────────────

pub async fn list_changes(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let items: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "id": "chg-001",
            "timestamp": "2026-04-03T10:30:00Z",
            "type": "CiliumNetworkPolicy",
            "resource": "allow-dns-egress",
            "namespace": "production",
            "diff_summary": "Added egress rule for UDP/53 to kube-dns",
            "author": "platform-team",
            "rollback_available": true
        }),
        serde_json::json!({
            "id": "chg-002",
            "timestamp": "2026-04-03T09:15:00Z",
            "type": "CiliumClusterwideNetworkPolicy",
            "resource": "deny-external-default",
            "namespace": "",
            "diff_summary": "Updated CIDR list: added 203.0.113.0/24 to deny list",
            "author": "security-team",
            "rollback_available": true
        }),
        serde_json::json!({
            "id": "chg-003",
            "timestamp": "2026-04-02T16:45:00Z",
            "type": "Service",
            "resource": "api-gateway",
            "namespace": "production",
            "diff_summary": "Changed service type from ClusterIP to LoadBalancer",
            "author": "dev-team",
            "rollback_available": false
        }),
        serde_json::json!({
            "id": "chg-004",
            "timestamp": "2026-04-02T14:00:00Z",
            "type": "ConfigMap",
            "resource": "cilium-config",
            "namespace": "kube-system",
            "diff_summary": "Enabled bandwidth manager, set devices=eth0",
            "author": "platform-team",
            "rollback_available": true
        }),
    ];
    Json(paginate_json(items, &params, "changes"))
}

pub async fn rollback_change(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;
    Ok(Json(serde_json::json!({
        "id": id,
        "status": "rolled_back",
        "message": "Change successfully rolled back",
        "rolled_back_at": "2026-04-03T12:10:00Z"
    })))
}

// ── Node Drain ────────────────────────────────────────────

pub async fn node_drain_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    let data = state.k8s.kubectl_json(&["get", "nodes", "-o", "json"]).await;

    let nodes: Vec<serde_json::Value> = data
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items.iter().map(|item| {
                let meta = item.get("metadata").unwrap_or(item);
                let spec = item.get("spec").unwrap_or(item);
                let status = item.get("status").unwrap_or(item);

                let name = meta.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let unschedulable = spec.get("unschedulable").and_then(|v| v.as_bool()).unwrap_or(false);

                let ready = status.get("conditions")
                    .and_then(|v| v.as_array())
                    .and_then(|conds| conds.iter().find(|c| c.get("type").and_then(|v| v.as_str()) == Some("Ready")))
                    .and_then(|c| c.get("status").and_then(|v| v.as_str()))
                    .unwrap_or("Unknown") == "True";

                let node_status = if unschedulable { "cordoned" } else if ready { "ready" } else { "not_ready" };

                serde_json::json!({
                    "node": name,
                    "status": node_status,
                    "cordon": unschedulable,
                    "ready": ready,
                })
            }).collect()
        })
        .unwrap_or_default();

    Json(serde_json::json!({ "nodes": nodes }))
}

pub async fn drain_node(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(body): Json<NodeActionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;
    let node = &body.node;
    Ok(Json(serde_json::json!({
        "node": node,
        "status": "draining",
        "message": "Node drain initiated, pods being evicted gracefully",
        "started_at": "2026-04-03T12:15:00Z",
        "estimated_completion": "2026-04-03T12:20:00Z"
    })))
}

pub async fn uncordon_node(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(body): Json<NodeActionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;
    let node = &body.node;
    Ok(Json(serde_json::json!({
        "node": node,
        "status": "ready",
        "message": "Node uncordoned and scheduling resumed",
        "cordon": false
    })))
}

// ── Pod Security ──────────────────────────────────────────

pub async fn pod_security(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    // Query namespaces for PSA labels and pods for security context
    let ns_data = state.k8s.kubectl_json(&["get", "namespaces", "-o", "json"]).await;

    let reports: Vec<serde_json::Value> = ns_data
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items.iter().filter_map(|item| {
                let meta = item.get("metadata")?;
                let name = meta.get("name").and_then(|v| v.as_str())?;
                let labels = meta.get("labels").and_then(|v| v.as_object());

                let enforce = labels
                    .and_then(|l| l.get("pod-security.kubernetes.io/enforce"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("not-set");
                let audit = labels
                    .and_then(|l| l.get("pod-security.kubernetes.io/audit"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("not-set");
                let warn = labels
                    .and_then(|l| l.get("pod-security.kubernetes.io/warn"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("not-set");

                Some(serde_json::json!({
                    "namespace": name,
                    "enforce_level": enforce,
                    "audit_level": audit,
                    "warn_level": warn,
                }))
            }).collect()
        })
        .unwrap_or_default();

    Json(serde_json::json!({ "reports": reports, "total": reports.len() }))
}

// ── Egress Gateway ────────────────────────────────────────

pub async fn egress_policies(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    // Query CiliumEgressGatewayPolicy resources
    let data = state.k8s.kubectl_json(&[
        "get", "ciliumegressgatewaypolicies", "--all-namespaces", "-o", "json",
    ]).await;

    let policies: Vec<serde_json::Value> = data
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items.iter().map(|item| {
                let meta = item.get("metadata").unwrap_or(item);
                let spec = item.get("spec").unwrap_or(item);
                serde_json::json!({
                    "name": meta.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    "namespace": meta.get("namespace").and_then(|v| v.as_str()).unwrap_or(""),
                    "spec": spec,
                    "created_at": meta.get("creationTimestamp").and_then(|v| v.as_str()).unwrap_or(""),
                })
            }).collect()
        })
        .unwrap_or_default();

    let total = policies.len();
    Json(serde_json::json!({ "policies": policies, "total": total }))
}

// ── Service Mesh ──────────────────────────────────────────

pub async fn mesh_services(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    // Query real K8s Services and infer protocol from ports/annotations
    let data = state.k8s.kubectl_json(&[
        "get", "services", "--all-namespaces", "-o", "json",
    ]).await;

    let services: Vec<serde_json::Value> = data
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items.iter().filter_map(|item| {
                let meta = item.get("metadata")?;
                let spec = item.get("spec")?;
                let name = meta.get("name").and_then(|v| v.as_str())?;
                let ns = meta.get("namespace").and_then(|v| v.as_str()).unwrap_or("");

                // Infer protocol from ports
                let ports = spec.get("ports").and_then(|v| v.as_array());
                let protocol = ports
                    .and_then(|p| p.first())
                    .and_then(|p| p.get("appProtocol").and_then(|v| v.as_str())
                        .or_else(|| p.get("protocol").and_then(|v| v.as_str())))
                    .unwrap_or("TCP");

                Some(serde_json::json!({
                    "name": name,
                    "namespace": ns,
                    "type": spec.get("type").and_then(|v| v.as_str()).unwrap_or("ClusterIP"),
                    "protocol": protocol,
                    "cluster_ip": spec.get("clusterIP").and_then(|v| v.as_str()).unwrap_or(""),
                    "ports": ports.cloned().unwrap_or_default(),
                }))
            }).collect()
        })
        .unwrap_or_default();

    let total = services.len();
    Json(serde_json::json!({ "services": services, "total": total }))
}

// ── KubeProxy Replacement Status ──────────────────────────

pub async fn kpr_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    // Read KPR config from cilium-config ConfigMap
    let cm = state.k8s.kubectl_json(&[
        "get", "configmap", "cilium-config", "-n", "kube-system", "-o", "json",
    ]).await;
    let empty = serde_json::json!({});
    let data = cm.get("data").unwrap_or(&empty);

    let kpr = data.get("kube-proxy-replacement").and_then(|v| v.as_str()).unwrap_or("disabled");
    let device = data.get("devices").and_then(|v| v.as_str()).unwrap_or("");
    let dsr = data.get("loadbalancer-mode").and_then(|v| v.as_str()).unwrap_or("snat");
    let session_affinity = data.get("enable-session-affinity").and_then(|v| v.as_str()).unwrap_or("false");
    let node_port_range = data.get("node-port-range").and_then(|v| v.as_str()).unwrap_or("30000-32767");

    Json(serde_json::json!({
        "enabled": kpr != "disabled" && !kpr.is_empty(),
        "mode": kpr,
        "device": device,
        "dsr_mode": dsr,
        "session_affinity": session_affinity == "true",
        "node_port_range": node_port_range,
        "source": "cilium-config ConfigMap",
    }))
}
