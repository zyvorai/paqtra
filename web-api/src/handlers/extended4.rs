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

pub async fn wireguard_peers(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
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
    }))
}

// ── Cilium Status ─────────────────────────────────────────

pub async fn cilium_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
        "agents": [
            {
                "node": "cilium-node-1",
                "status": "OK",
                "version": "1.16.1",
                "uptime": "14d 6h 32m",
                "controllers_failing": 0,
                "controllers_total": 42,
                "endpoint_count": 58,
                "policy_revision": 124,
                "proxy_redirects": 12,
                "identity_count": 87,
                "datapath": "vxlan",
                "masquerading": "BPF",
                "encryption": "WireGuard",
                "kube_proxy_replacement": "True"
            },
            {
                "node": "cilium-node-2",
                "status": "OK",
                "version": "1.16.1",
                "uptime": "14d 6h 30m",
                "controllers_failing": 0,
                "controllers_total": 42,
                "endpoint_count": 46,
                "policy_revision": 124,
                "proxy_redirects": 8,
                "identity_count": 87,
                "datapath": "vxlan",
                "masquerading": "BPF",
                "encryption": "WireGuard",
                "kube_proxy_replacement": "True"
            },
            {
                "node": "cilium-node-3",
                "status": "OK",
                "version": "1.16.1",
                "uptime": "14d 6h 28m",
                "controllers_failing": 1,
                "controllers_total": 42,
                "endpoint_count": 38,
                "policy_revision": 123,
                "proxy_redirects": 6,
                "identity_count": 87,
                "datapath": "vxlan",
                "masquerading": "BPF",
                "encryption": "WireGuard",
                "kube_proxy_replacement": "True"
            }
        ]
    }))
}

// ── Policy Validation ─────────────────────────────────────

pub async fn validate_policy(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ValidatePolicyRequest>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let yaml = &body.yaml;
    let valid = !yaml.is_empty();
    let errors: Vec<String> = if valid {
        vec![]
    } else {
        vec!["Empty policy YAML provided".to_string()]
    };
    Json(serde_json::json!({
        "valid": valid,
        "errors": errors
    }))
}

// ── Flow Export Configs ───────────────────────────────────

pub async fn list_export_configs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let items: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "id": "exp-001",
            "name": "production-s3-export",
            "format": "json",
            "destination": "s3://cilium-flows-prod/daily/",
            "filter": "namespace=production",
            "status": "active",
            "exported_count": 1_842_500,
            "last_export": "2026-04-03T11:55:00Z"
        }),
        serde_json::json!({
            "id": "exp-002",
            "name": "security-siem-feed",
            "format": "cef",
            "destination": "syslog://siem.internal:514",
            "filter": "verdict=DROPPED",
            "status": "active",
            "exported_count": 34_210,
            "last_export": "2026-04-03T11:59:30Z"
        }),
        serde_json::json!({
            "id": "exp-003",
            "name": "staging-debug",
            "format": "csv",
            "destination": "s3://cilium-flows-staging/debug/",
            "filter": "namespace=staging",
            "status": "paused",
            "exported_count": 520_000,
            "last_export": "2026-04-02T18:00:00Z"
        }),
    ];
    Json(paginate_json(items, &params, "configs"))
}

pub async fn create_export_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateExportRequest>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let name = &body.name;
    let format = &body.format;
    let destination = &body.destination;
    Json(serde_json::json!({
        "id": "exp-004",
        "name": name,
        "format": format,
        "destination": destination,
        "status": "active",
        "message": "Export configuration created successfully"
    }))
}

pub async fn delete_export_config(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
        "id": id,
        "status": "deleted",
        "message": "Export configuration deleted successfully"
    }))
}

// ── SLO Targets ───────────────────────────────────────────

pub async fn list_slos(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
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
    Json(paginate_json(items, &params, "slos"))
}

// ── Incidents ─────────────────────────────────────────────

pub async fn list_incidents(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
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
    Json(serde_json::json!({
        "nodes": [
            {
                "node": "cilium-node-1",
                "status": "ready",
                "pods_evicted": 0,
                "pods_remaining": 58,
                "started_at": null,
                "cordon": false
            },
            {
                "node": "cilium-node-2",
                "status": "ready",
                "pods_evicted": 0,
                "pods_remaining": 46,
                "started_at": null,
                "cordon": false
            },
            {
                "node": "cilium-node-3",
                "status": "cordoned",
                "pods_evicted": 0,
                "pods_remaining": 38,
                "started_at": null,
                "cordon": true
            }
        ]
    }))
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
    Json(serde_json::json!({
        "reports": [
            {
                "namespace": "production",
                "enforce_level": "restricted",
                "audit_level": "restricted",
                "warn_level": "restricted",
                "total_pods": 42,
                "compliant_pods": 42,
                "violations": []
            },
            {
                "namespace": "staging",
                "enforce_level": "baseline",
                "audit_level": "restricted",
                "warn_level": "restricted",
                "total_pods": 18,
                "compliant_pods": 15,
                "violations": [
                    { "pod": "debug-tools-7f8a2b-xk4mn", "violation": "Container runs as root (UID 0)", "severity": "high" },
                    { "pod": "legacy-worker-3c9d1e-pl2qr", "violation": "Privileged container detected", "severity": "critical" },
                    { "pod": "test-runner-5a6b7c-ws4jk", "violation": "Host network namespace enabled", "severity": "high" }
                ]
            },
            {
                "namespace": "kube-system",
                "enforce_level": "privileged",
                "audit_level": "baseline",
                "warn_level": "baseline",
                "total_pods": 24,
                "compliant_pods": 24,
                "violations": []
            }
        ]
    }))
}

// ── Egress Gateway ────────────────────────────────────────

pub async fn egress_policies(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
        "policies": [
            {
                "name": "external-api-egress",
                "namespace": "production",
                "gateway_node": "cilium-node-1",
                "egress_ip": "203.0.113.10",
                "destination_cidrs": ["198.51.100.0/24", "203.0.113.0/24"],
                "selectors": ["app=api-gateway", "role=backend"],
                "status": "active"
            },
            {
                "name": "db-egress",
                "namespace": "production",
                "gateway_node": "cilium-node-2",
                "egress_ip": "203.0.113.11",
                "destination_cidrs": ["10.100.0.0/16"],
                "selectors": ["app=data-service"],
                "status": "active"
            },
            {
                "name": "monitoring-egress",
                "namespace": "monitoring",
                "gateway_node": "cilium-node-1",
                "egress_ip": "203.0.113.12",
                "destination_cidrs": ["0.0.0.0/0"],
                "selectors": ["app=prometheus", "app=grafana"],
                "status": "active"
            }
        ]
    }))
}

// ── Service Mesh ──────────────────────────────────────────

pub async fn mesh_services(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
        "services": [
            {
                "name": "api-gateway",
                "namespace": "production",
                "protocol": "HTTP/2",
                "mtls": true,
                "retries": 3,
                "timeout_ms": 5000,
                "circuit_breaker": true,
                "traffic_policy": "round-robin"
            },
            {
                "name": "frontend",
                "namespace": "production",
                "protocol": "HTTP/1.1",
                "mtls": true,
                "retries": 2,
                "timeout_ms": 3000,
                "circuit_breaker": false,
                "traffic_policy": "least-connections"
            },
            {
                "name": "grpc-backend",
                "namespace": "staging",
                "protocol": "gRPC",
                "mtls": true,
                "retries": 3,
                "timeout_ms": 10000,
                "circuit_breaker": true,
                "traffic_policy": "round-robin"
            },
            {
                "name": "redis-cache",
                "namespace": "production",
                "protocol": "TCP",
                "mtls": false,
                "retries": 1,
                "timeout_ms": 1000,
                "circuit_breaker": false,
                "traffic_policy": "random"
            }
        ]
    }))
}

// ── KubeProxy Replacement Status ──────────────────────────

pub async fn kpr_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
        "enabled": true,
        "mode": "strict",
        "device": "eth0",
        "dsr_mode": "geneve",
        "session_affinity": true,
        "graceful_termination": true,
        "node_port_range": "30000-32767",
        "services": 42,
        "backends": 128,
        "nat_entries": 8_452,
        "ct_entries": 24_310
    }))
}
