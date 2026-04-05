use axum::{extract::{Path, Query, State}, Json};
use std::sync::Arc;
use crate::AppState;
use super::{track_request, PaginationQuery, paginate_json};

// ── Host Info ───────────────────────────────────────────────

pub async fn host_info(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
        "hostname": "cilium-node-1",
        "os": "Ubuntu 22.04.4 LTS",
        "kernel": "6.5.0-35-generic",
        "arch": "x86_64",
        "cpu_model": "AMD EPYC 7763 64-Core Processor",
        "cpu_cores": 16,
        "cpu_usage": 34.2,
        "memory_total_gb": 64.0,
        "memory_used_gb": 42.8,
        "disk_total_gb": 500.0,
        "disk_used_gb": 185.3,
        "uptime_seconds": 1_296_000,
        "load_average": [2.45, 1.82, 1.56],
        "network_interfaces": [
            { "name": "eth0", "ip": "10.0.1.5", "mac": "02:42:0a:00:01:05", "speed": "25 Gbps", "status": "up" },
            { "name": "cilium_host", "ip": "10.244.0.1", "mac": "3e:9a:1c:ff:00:01", "speed": "10 Gbps", "status": "up" },
            { "name": "cilium_vxlan", "ip": "10.244.0.1", "mac": "a2:b4:c6:d8:e0:f2", "speed": "10 Gbps", "status": "up" },
            { "name": "lxc_health", "ip": "10.244.0.2", "mac": "fe:ed:ca:fe:00:01", "speed": "10 Gbps", "status": "up" }
        ]
    }))
}

// ── Policy Templates ────────────────────────────────────────

pub async fn list_policy_templates(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let items: Vec<serde_json::Value> = vec![
        serde_json::json!({ "id": "tpl-001", "name": "Default Deny All", "category": "security", "description": "Deny all ingress and egress traffic by default", "tags": ["zero-trust", "baseline"],
          "yaml": "apiVersion: cilium.io/v2\nkind: CiliumNetworkPolicy\nmetadata:\n  name: default-deny\nspec:\n  endpointSelector: {}\n  ingress: []\n  egress: []" }),
        serde_json::json!({ "id": "tpl-002", "name": "Allow DNS", "category": "connectivity", "description": "Allow DNS resolution to kube-dns", "tags": ["dns", "essential"],
          "yaml": "apiVersion: cilium.io/v2\nkind: CiliumNetworkPolicy\nmetadata:\n  name: allow-dns\nspec:\n  endpointSelector: {}\n  egress:\n  - toEndpoints:\n    - matchLabels:\n        k8s:io.kubernetes.pod.namespace: kube-system\n        k8s-app: kube-dns\n    toPorts:\n    - ports:\n      - port: \"53\"\n        protocol: UDP" }),
        serde_json::json!({ "id": "tpl-003", "name": "Allow HTTP Ingress", "category": "connectivity", "description": "Allow HTTP/HTTPS ingress from any source", "tags": ["http", "ingress"],
          "yaml": "apiVersion: cilium.io/v2\nkind: CiliumNetworkPolicy\nmetadata:\n  name: allow-http\nspec:\n  endpointSelector:\n    matchLabels:\n      app: web\n  ingress:\n  - toPorts:\n    - ports:\n      - port: \"80\"\n      - port: \"443\"" }),
        serde_json::json!({ "id": "tpl-004", "name": "Namespace Isolation", "category": "security", "description": "Restrict traffic to same namespace only", "tags": ["isolation", "namespace"],
          "yaml": "apiVersion: cilium.io/v2\nkind: CiliumNetworkPolicy\nmetadata:\n  name: namespace-isolation\nspec:\n  endpointSelector: {}\n  ingress:\n  - fromEndpoints:\n    - matchLabels:\n        io.kubernetes.pod.namespace: ${NAMESPACE}" }),
        serde_json::json!({ "id": "tpl-005", "name": "Allow Monitoring", "category": "observability", "description": "Allow Prometheus scraping on metrics port", "tags": ["prometheus", "monitoring"],
          "yaml": "apiVersion: cilium.io/v2\nkind: CiliumNetworkPolicy\nmetadata:\n  name: allow-monitoring\nspec:\n  endpointSelector: {}\n  ingress:\n  - fromEndpoints:\n    - matchLabels:\n        app: prometheus\n    toPorts:\n    - ports:\n      - port: \"9090\"" }),
        serde_json::json!({ "id": "tpl-006", "name": "L7 HTTP Policy", "category": "l7", "description": "L7 policy allowing only GET and POST methods", "tags": ["l7", "http", "advanced"],
          "yaml": "apiVersion: cilium.io/v2\nkind: CiliumNetworkPolicy\nmetadata:\n  name: l7-http\nspec:\n  endpointSelector:\n    matchLabels:\n      app: api\n  ingress:\n  - toPorts:\n    - ports:\n      - port: \"8080\"\n      rules:\n        http:\n        - method: GET\n        - method: POST" }),
    ];
    Json(paginate_json(items, &params, "templates"))
}

pub async fn apply_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({ "id": id, "status": "applied", "message": "Template applied successfully" }))
}

// ── Diagnostics ─────────────────────────────────────────────

pub async fn run_diagnostics(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let tests = serde_json::json!([
        { "name": "Cilium Agent Health", "status": "pass", "message": "All agents healthy on 3 nodes", "duration_ms": 120 },
        { "name": "Hubble Relay Connectivity", "status": "pass", "message": "Connected to relay at hubble-relay:4245", "duration_ms": 85 },
        { "name": "DNS Resolution", "status": "pass", "message": "kubernetes.default.svc resolved in 2ms", "duration_ms": 45 },
        { "name": "Pod-to-Pod Connectivity", "status": "pass", "message": "Cross-node ping successful (1.2ms)", "duration_ms": 250 },
        { "name": "Service Connectivity", "status": "pass", "message": "ClusterIP services reachable", "duration_ms": 180 },
        { "name": "Network Policy Enforcement", "status": "pass", "message": "Policies correctly enforcing on all endpoints", "duration_ms": 340 },
        { "name": "IPsec/WireGuard Encryption", "status": "warn", "message": "Encryption not enabled", "duration_ms": 50 },
        { "name": "MTU Consistency", "status": "pass", "message": "MTU 1500 consistent across all nodes", "duration_ms": 90 },
        { "name": "BPF Filesystem", "status": "pass", "message": "/sys/fs/bpf mounted correctly", "duration_ms": 15 },
        { "name": "Kernel Version", "status": "pass", "message": "Kernel 6.5.0 supports all required features", "duration_ms": 10 }
    ]);
    Json(serde_json::json!({ "tests": tests }))
}

pub async fn connectivity_test(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({ "reachable": true, "latency_ms": 1.8, "hops": 2 }))
}

// ── Audit Log ───────────────────────────────────────────────

pub async fn audit_log(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let entries = serde_json::json!([
        { "id": "aud-001", "timestamp": "2026-04-03T10:05:00Z", "action": "policy.create", "actor": "admin@cilium", "resource": "CiliumNetworkPolicy/allow-dns", "namespace": "default", "details": "Created allow-dns policy", "outcome": "success" },
        { "id": "aud-002", "timestamp": "2026-04-03T10:03:00Z", "action": "anomaly.remediate", "actor": "system/healer", "resource": "Anomaly/anom-003", "namespace": "default", "details": "Auto-remediated DNS timeout anomaly", "outcome": "success" },
        { "id": "aud-003", "timestamp": "2026-04-03T09:55:00Z", "action": "policy.delete", "actor": "admin@cilium", "resource": "CiliumNetworkPolicy/legacy-allow-all", "namespace": "default", "details": "Removed overly permissive policy", "outcome": "success" },
        { "id": "aud-004", "timestamp": "2026-04-03T09:45:00Z", "action": "chaos.run", "actor": "sre@team", "resource": "ChaosExperiment/latency-test", "namespace": "staging", "details": "Ran latency spike experiment", "outcome": "success" },
        { "id": "aud-005", "timestamp": "2026-04-03T09:30:00Z", "action": "compliance.audit", "actor": "admin@cilium", "resource": "Framework/SOC2", "namespace": "-", "details": "SOC2 compliance audit completed: 72% score", "outcome": "success" },
        { "id": "aud-006", "timestamp": "2026-04-03T09:15:00Z", "action": "policy.simulate", "actor": "dev@team", "resource": "CiliumNetworkPolicy/restrict-egress", "namespace": "production", "details": "Dry-run: 45 flows affected, risk=medium", "outcome": "success" }
    ]);
    Json(serde_json::json!({ "entries": entries, "total": 6 }))
}

// ── Alerts ──────────────────────────────────────────────────

pub async fn list_alert_rules(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let items: Vec<serde_json::Value> = vec![
        serde_json::json!({ "id": "rule-001", "name": "High Drop Rate", "condition": "drop_rate > 5% for 5m", "severity": "critical", "enabled": true, "last_triggered": "2026-04-03T09:00:00Z", "trigger_count": 3, "channels": ["slack", "pagerduty"] }),
        serde_json::json!({ "id": "rule-002", "name": "DNS Resolution Failure", "condition": "dns_servfail > 10/min", "severity": "high", "enabled": true, "last_triggered": null, "trigger_count": 0, "channels": ["slack"] }),
        serde_json::json!({ "id": "rule-003", "name": "Policy Deny Spike", "condition": "policy_denied > 100/min", "severity": "warning", "enabled": true, "last_triggered": "2026-04-02T14:30:00Z", "trigger_count": 7, "channels": ["slack", "email"] }),
        serde_json::json!({ "id": "rule-004", "name": "Endpoint Unhealthy", "condition": "endpoint_status != ready", "severity": "high", "enabled": false, "last_triggered": "2026-04-01T08:00:00Z", "trigger_count": 2, "channels": ["pagerduty"] }),
        serde_json::json!({ "id": "rule-005", "name": "CT Table Near Full", "condition": "ct_entries > 90% max", "severity": "warning", "enabled": true, "last_triggered": null, "trigger_count": 0, "channels": ["slack"] }),
    ];
    Json(paginate_json(items, &params, "rules"))
}

pub async fn alert_history(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let alerts = serde_json::json!([
        { "id": "alert-001", "rule_name": "High Drop Rate", "severity": "critical", "message": "Drop rate 8.2% in namespace default for past 5 minutes", "fired_at": "2026-04-03T09:00:00Z", "resolved_at": "2026-04-03T09:12:00Z", "status": "resolved" },
        { "id": "alert-002", "rule_name": "Policy Deny Spike", "severity": "warning", "message": "142 policy denials in past minute from default/frontend", "fired_at": "2026-04-02T14:30:00Z", "resolved_at": null, "status": "firing" }
    ]);
    Json(serde_json::json!({ "alerts": alerts }))
}

pub async fn toggle_alert_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({ "id": id, "status": "updated" }))
}

// ── Service Map ─────────────────────────────────────────────

pub async fn service_map(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let nodes = serde_json::json!([
        { "name": "frontend", "namespace": "default", "type": "Deployment", "pods": 3, "status": "healthy", "request_rate": 1200.0, "error_rate": 0.1 },
        { "name": "api-gateway", "namespace": "default", "type": "Deployment", "pods": 2, "status": "healthy", "request_rate": 800.0, "error_rate": 0.3 },
        { "name": "backend", "namespace": "default", "type": "Deployment", "pods": 4, "status": "healthy", "request_rate": 600.0, "error_rate": 0.0 },
        { "name": "auth-service", "namespace": "default", "type": "Deployment", "pods": 2, "status": "degraded", "request_rate": 400.0, "error_rate": 1.5 },
        { "name": "redis", "namespace": "default", "type": "StatefulSet", "pods": 1, "status": "healthy", "request_rate": 3500.0, "error_rate": 0.0 },
        { "name": "postgres", "namespace": "default", "type": "StatefulSet", "pods": 1, "status": "healthy", "request_rate": 600.0, "error_rate": 0.1 }
    ]);
    let edges = serde_json::json!([
        { "source": "frontend", "target": "api-gateway", "protocol": "HTTP", "request_rate": 1200.0, "latency_ms": 12.0 },
        { "source": "api-gateway", "target": "backend", "protocol": "gRPC", "request_rate": 800.0, "latency_ms": 8.0 },
        { "source": "api-gateway", "target": "auth-service", "protocol": "HTTP", "request_rate": 400.0, "latency_ms": 45.0 },
        { "source": "backend", "target": "redis", "protocol": "TCP", "request_rate": 3500.0, "latency_ms": 0.5 },
        { "source": "backend", "target": "postgres", "protocol": "TCP", "request_rate": 600.0, "latency_ms": 3.0 }
    ]);
    Json(serde_json::json!({ "nodes": nodes, "edges": edges }))
}

// ── Packet Capture ──────────────────────────────────────────

pub async fn list_capture_sessions(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let items: Vec<serde_json::Value> = vec![
        serde_json::json!({ "id": "cap-001", "name": "debug-dns", "target_pod": "coredns-abc", "namespace": "kube-system", "interface_name": "eth0", "filter": "port 53", "status": "completed", "packet_count": 4520, "size_bytes": 850_000, "started_at": "2026-04-03T09:00:00Z" }),
        serde_json::json!({ "id": "cap-002", "name": "api-traffic", "target_pod": "api-gateway-def456", "namespace": "default", "interface_name": "eth0", "filter": "port 8080", "status": "capturing", "packet_count": 12800, "size_bytes": 3_200_000, "started_at": "2026-04-03T10:00:00Z" }),
    ];
    Json(paginate_json(items, &params, "sessions"))
}

pub async fn start_capture(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({ "id": "cap-003", "status": "capturing", "message": "Capture started" }))
}

pub async fn stop_capture(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({ "id": id, "status": "completed", "message": "Capture stopped" }))
}

// ── DNS Monitor ─────────────────────────────────────────────

pub async fn dns_queries(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let queries = serde_json::json!([
        { "id": "dns-001", "timestamp": "2026-04-03T10:05:00Z", "source_pod": "frontend-abc123", "namespace": "default", "query_name": "api-gateway.default.svc.cluster.local", "query_type": "A", "response_code": "NOERROR", "response_ips": ["10.96.0.15"], "latency_ms": 1.2 },
        { "id": "dns-002", "timestamp": "2026-04-03T10:05:01Z", "source_pod": "backend-xyz789", "namespace": "default", "query_name": "redis-master.default.svc.cluster.local", "query_type": "A", "response_code": "NOERROR", "response_ips": ["10.96.0.20"], "latency_ms": 0.8 },
        { "id": "dns-003", "timestamp": "2026-04-03T10:05:02Z", "source_pod": "frontend-abc123", "namespace": "default", "query_name": "external-api.example.com", "query_type": "A", "response_code": "NOERROR", "response_ips": ["203.0.113.50"], "latency_ms": 15.3 },
        { "id": "dns-004", "timestamp": "2026-04-03T10:05:03Z", "source_pod": "backend-xyz789", "namespace": "default", "query_name": "nonexistent.svc.cluster.local", "query_type": "A", "response_code": "NXDOMAIN", "response_ips": [], "latency_ms": 2.1 }
    ]);
    Json(serde_json::json!({ "queries": queries, "total": 4 }))
}

pub async fn dns_stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
        "total_queries": 45200,
        "successful": 44800,
        "nxdomain": 350,
        "servfail": 50,
        "avg_latency_ms": 2.4,
        "top_domains": [
            { "domain": "kubernetes.default.svc.cluster.local", "count": 12000 },
            { "domain": "redis-master.default.svc.cluster.local", "count": 8500 },
            { "domain": "api-gateway.default.svc.cluster.local", "count": 6200 },
            { "domain": "coredns.kube-system.svc.cluster.local", "count": 4100 },
            { "domain": "external-api.example.com", "count": 2800 }
        ]
    }))
}

// ── Identities ──────────────────────────────────────────────

pub async fn list_identities(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let items: Vec<serde_json::Value> = vec![
        serde_json::json!({ "id": 1, "labels": ["reserved:host"], "namespace": "kube-system", "endpoints_count": 3, "policy_count": 0, "created_at": "2026-03-01T00:00:00Z" }),
        serde_json::json!({ "id": 12345, "labels": ["app=frontend", "version=v2"], "namespace": "default", "endpoints_count": 3, "policy_count": 2, "created_at": "2026-03-15T10:00:00Z" }),
        serde_json::json!({ "id": 12346, "labels": ["app=backend", "version=v1"], "namespace": "default", "endpoints_count": 4, "policy_count": 3, "created_at": "2026-03-15T10:00:00Z" }),
        serde_json::json!({ "id": 12347, "labels": ["app=redis", "role=master"], "namespace": "default", "endpoints_count": 1, "policy_count": 1, "created_at": "2026-03-15T10:00:00Z" }),
        serde_json::json!({ "id": 10001, "labels": ["k8s-app=kube-dns"], "namespace": "kube-system", "endpoints_count": 2, "policy_count": 1, "created_at": "2026-03-01T00:00:00Z" }),
        serde_json::json!({ "id": 4, "labels": ["reserved:health"], "namespace": "kube-system", "endpoints_count": 3, "policy_count": 0, "created_at": "2026-03-01T00:00:00Z" }),
    ];
    Json(paginate_json(items, &params, "identities"))
}

// ── Cluster Mesh ────────────────────────────────────────────

pub async fn list_mesh_peers(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let items: Vec<serde_json::Value> = vec![
        serde_json::json!({ "name": "us-east-prod", "endpoint": "10.1.0.1:2379", "status": "connected", "connected_since": "2026-04-01T00:00:00Z", "synced_identities": 245, "synced_endpoints": 1200, "synced_services": 85, "latency_ms": 2.1 }),
        serde_json::json!({ "name": "eu-west-prod", "endpoint": "10.2.0.1:2379", "status": "connected", "connected_since": "2026-04-01T00:00:00Z", "synced_identities": 180, "synced_endpoints": 850, "synced_services": 62, "latency_ms": 45.3 }),
    ];
    Json(paginate_json(items, &params, "peers"))
}

pub async fn connect_mesh_peer(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({ "status": "connecting", "message": "Peer connection initiated" }))
}

// ── BGP Peering ─────────────────────────────────────────────

pub async fn list_bgp_peers(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let items: Vec<serde_json::Value> = vec![
        serde_json::json!({ "name": "tor-switch-1", "peer_address": "10.0.0.1", "peer_asn": 65000, "local_asn": 65001, "state": "Established", "uptime": "15d 4h", "prefixes_received": 24, "prefixes_advertised": 12, "messages_received": 45200, "messages_sent": 44800 }),
        serde_json::json!({ "name": "tor-switch-2", "peer_address": "10.0.0.2", "peer_asn": 65000, "local_asn": 65001, "state": "Established", "uptime": "15d 4h", "prefixes_received": 24, "prefixes_advertised": 12, "messages_received": 45100, "messages_sent": 44700 }),
        serde_json::json!({ "name": "spine-switch", "peer_address": "10.0.0.254", "peer_asn": 64999, "local_asn": 65001, "state": "Idle", "uptime": "0s", "prefixes_received": 0, "prefixes_advertised": 0, "messages_received": 0, "messages_sent": 0 }),
    ];
    Json(paginate_json(items, &params, "peers"))
}

// ── Bandwidth ───────────────────────────────────────────────

pub async fn bandwidth_data(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let entries = serde_json::json!([
        { "pod": "frontend-abc123", "namespace": "default", "egress_rate_mbps": 45.2, "ingress_rate_mbps": 120.5, "egress_limit_mbps": 100, "ingress_limit_mbps": null, "total_bytes_tx": 5_400_000_000_u64, "total_bytes_rx": 14_500_000_000_u64 },
        { "pod": "backend-xyz789", "namespace": "default", "egress_rate_mbps": 85.1, "ingress_rate_mbps": 62.3, "egress_limit_mbps": null, "ingress_limit_mbps": null, "total_bytes_tx": 10_200_000_000_u64, "total_bytes_rx": 7_500_000_000_u64 },
        { "pod": "redis-master-0", "namespace": "default", "egress_rate_mbps": 210.5, "ingress_rate_mbps": 180.2, "egress_limit_mbps": 500, "ingress_limit_mbps": 500, "total_bytes_tx": 25_000_000_000_u64, "total_bytes_rx": 21_600_000_000_u64 },
        { "pod": "api-gateway-def456", "namespace": "default", "egress_rate_mbps": 95.8, "ingress_rate_mbps": 150.4, "egress_limit_mbps": 200, "ingress_limit_mbps": 200, "total_bytes_tx": 11_500_000_000_u64, "total_bytes_rx": 18_000_000_000_u64 },
        { "pod": "prometheus-0", "namespace": "monitoring", "egress_rate_mbps": 12.3, "ingress_rate_mbps": 350.8, "egress_limit_mbps": null, "ingress_limit_mbps": null, "total_bytes_tx": 1_500_000_000_u64, "total_bytes_rx": 42_000_000_000_u64 }
    ]);
    Json(serde_json::json!({ "entries": entries }))
}
