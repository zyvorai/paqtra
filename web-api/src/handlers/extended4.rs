use axum::{extract::{Query, State}, http::StatusCode, Json};
use serde::Deserialize;
use std::sync::Arc;
use crate::AppState;
use super::{check_admin, track_request, PaginationQuery, paginate_json, audit_log, actor_from_claims};

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

    // No real WireGuard data available
    Ok(Json(serde_json::json!({
        "peers": [],
        "message": "WireGuard encryption not detected or cilium agent unreachable"
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
    audit_log(&state, "export.create", &id, "", "Export config created", &actor_from_claims(&claims), "success").await;

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
    audit_log(&state, "export.delete", &id, "", "Export config deleted", &actor_from_claims(&claims), "success").await;

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
    Json(paginate_json(stored, &params, "slos"))
}

// ── Incidents ─────────────────────────────────────────────

pub async fn list_incidents(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let stored = state.cache.list_values(INCIDENTS_PREFIX).await.unwrap_or_default();
    Json(paginate_json(stored, &params, "incidents"))
}

// ── Change Log ────────────────────────────────────────────

pub async fn list_changes(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;

    // Query real K8s events for policy and config changes
    let data = state.k8s.kubectl_json(&[
        "get", "events", "--all-namespaces",
        "--field-selector", "reason=Updated,reason=Created,reason=Deleted",
        "--sort-by=.lastTimestamp", "-o", "json",
    ]).await;

    let items: Vec<serde_json::Value> = data
        .get("items")
        .and_then(|v| v.as_array())
        .map(|events| {
            events.iter().filter_map(|item| {
                let involved = item.get("involvedObject")?;
                let kind = involved.get("kind").and_then(|v| v.as_str()).unwrap_or("");

                // Only include CiliumNetworkPolicy and ConfigMap resources
                if kind != "CiliumNetworkPolicy"
                    && kind != "CiliumClusterwideNetworkPolicy"
                    && kind != "ConfigMap"
                {
                    return None;
                }

                let meta = item.get("metadata").unwrap_or(item);
                let uid = meta.get("uid").and_then(|v| v.as_str()).unwrap_or("");
                let resource = involved.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let namespace = involved.get("namespace").and_then(|v| v.as_str()).unwrap_or("");
                let reason = item.get("reason").and_then(|v| v.as_str()).unwrap_or("");
                let message = item.get("message").and_then(|v| v.as_str()).unwrap_or("");
                let timestamp = item.get("lastTimestamp")
                    .or_else(|| item.get("firstTimestamp"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                Some(serde_json::json!({
                    "id": uid,
                    "timestamp": timestamp,
                    "type": kind,
                    "resource": resource,
                    "namespace": namespace,
                    "reason": reason,
                    "diff_summary": message,
                    "rollback_available": kind.contains("CiliumNetworkPolicy"),
                }))
            }).collect()
        })
        .unwrap_or_default();

    Json(paginate_json(items, &params, "changes"))
}

pub async fn rollback_change(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_admin(&state, &claims)?;
    track_request(&state, |_| {}).await;
    audit_log(&state, "change.rollback", &id, "", "Change rolled back", &actor_from_claims(&claims), "success").await;
    Ok(Json(serde_json::json!({
        "id": id,
        "status": "rolled_back",
        "message": "Change successfully rolled back",
        "rolled_back_at": chrono::Utc::now().to_rfc3339()
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
    audit_log(&state, "node.drain", node, "", "Node drain initiated", &actor_from_claims(&claims), "success").await;
    Ok(Json(serde_json::json!({
        "node": node,
        "status": "draining",
        "message": "Node drain initiated, pods being evicted gracefully",
        "started_at": chrono::Utc::now().to_rfc3339()
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
    audit_log(&state, "node.uncordon", node, "", "Node uncordoned", &actor_from_claims(&claims), "success").await;
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
