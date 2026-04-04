use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;
use crate::AppState;
use super::track_request;

// ── Replay ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ReplayRecording {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub start_time: String,
    pub end_time: String,
    pub flow_count: u64,
    pub status: String,
    pub size_bytes: u64,
}

pub async fn list_recordings(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |m| m.total_requests += 0).await;
    let recordings = vec![
        ReplayRecording { id: "rec-001".into(), name: "debug-dns-issue".into(), namespace: "default".into(),
            start_time: "2026-04-03T08:00:00Z".into(), end_time: "2026-04-03T08:15:00Z".into(),
            flow_count: 15420, status: "completed".into(), size_bytes: 2_500_000 },
        ReplayRecording { id: "rec-002".into(), name: "prod-outage-capture".into(), namespace: "production".into(),
            start_time: "2026-04-02T14:30:00Z".into(), end_time: "2026-04-02T15:00:00Z".into(),
            flow_count: 89500, status: "completed".into(), size_bytes: 12_000_000 },
    ];
    Json(serde_json::json!({ "recordings": recordings }))
}

pub async fn start_recording(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("recording");
    Json(serde_json::json!({
        "id": "rec-003",
        "name": name,
        "status": "recording",
        "message": "Recording started"
    }))
}

pub async fn stop_recording(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({ "status": "stopped", "message": "Recording stopped" }))
}

// ── Healer ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct HealerProblem {
    pub id: String,
    #[serde(rename = "type")]
    pub problem_type: String,
    pub severity: String,
    pub description: String,
    pub affected_pods: Vec<String>,
    pub namespace: String,
    pub detected_at: String,
    pub status: String,
    pub proposed_fix: String,
}

pub async fn list_healer_problems(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let problems = vec![
        HealerProblem {
            id: "heal-001".into(), problem_type: "policy_denial".into(), severity: "high".into(),
            description: "Pods in namespace 'default' denied egress to kube-dns".into(),
            affected_pods: vec!["frontend-abc123".into(), "backend-xyz789".into()],
            namespace: "default".into(), detected_at: "2026-04-03T09:30:00Z".into(),
            status: "open".into(), proposed_fix: "Add CiliumNetworkPolicy allowing UDP/53 egress to kube-system".into(),
        },
        HealerProblem {
            id: "heal-002".into(), problem_type: "unreachable_backend".into(), severity: "medium".into(),
            description: "Service 'redis-master' has no healthy endpoints".into(),
            affected_pods: vec!["redis-master-0".into()],
            namespace: "default".into(), detected_at: "2026-04-03T09:45:00Z".into(),
            status: "investigating".into(), proposed_fix: "Restart pod redis-master-0 and verify readiness probe".into(),
        },
        HealerProblem {
            id: "heal-003".into(), problem_type: "dns_failure".into(), severity: "critical".into(),
            description: "CoreDNS pods failing to resolve external domains".into(),
            affected_pods: vec!["coredns-abc".into(), "coredns-def".into()],
            namespace: "kube-system".into(), detected_at: "2026-04-03T10:00:00Z".into(),
            status: "open".into(), proposed_fix: "Check upstream DNS servers in CoreDNS ConfigMap".into(),
        },
    ];
    Json(serde_json::json!({ "problems": problems }))
}

pub async fn apply_healer_fix(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({ "status": "applied", "message": "Fix applied successfully" }))
}

// ── RootCause ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct PacketDrop {
    pub id: String,
    pub timestamp: String,
    pub source: String,
    pub destination: String,
    pub protocol: String,
    pub drop_reason: String,
    pub root_cause: String,
    pub remediation: String,
    pub namespace: String,
    pub count: u32,
}

pub async fn list_packet_drops(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let drops = vec![
        PacketDrop { id: "drop-001".into(), timestamp: "2026-04-03T10:01:00Z".into(),
            source: "default/frontend-abc123".into(), destination: "kube-system/coredns".into(),
            protocol: "UDP".into(), drop_reason: "POLICY_DENIED".into(),
            root_cause: "Missing egress policy for DNS traffic".into(),
            remediation: "Apply allow-dns CiliumNetworkPolicy".into(),
            namespace: "default".into(), count: 142 },
        PacketDrop { id: "drop-002".into(), timestamp: "2026-04-03T10:02:00Z".into(),
            source: "default/backend-xyz789".into(), destination: "10.96.0.1:443".into(),
            protocol: "TCP".into(), drop_reason: "CT_MAP_INSERTION_FAILED".into(),
            root_cause: "Conntrack table full on node-2".into(),
            remediation: "Increase bpf-ct-global-tcp-max in Cilium ConfigMap".into(),
            namespace: "default".into(), count: 28 },
        PacketDrop { id: "drop-003".into(), timestamp: "2026-04-03T10:03:00Z".into(),
            source: "monitoring/prometheus-0".into(), destination: "default/api-gateway:8080".into(),
            protocol: "TCP".into(), drop_reason: "POLICY_DENIED".into(),
            root_cause: "Ingress policy on api-gateway blocks monitoring namespace".into(),
            remediation: "Add ingress rule allowing monitoring namespace on port 8080".into(),
            namespace: "monitoring".into(), count: 56 },
    ];
    Json(serde_json::json!({ "drops": drops, "total": drops.len() }))
}

pub async fn analyze_drops(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
        "analysis": {
            "total_drops": 226,
            "unique_reasons": 2,
            "top_reason": "POLICY_DENIED",
            "top_source_namespace": "default",
            "recommendation": "Review CiliumNetworkPolicies in default namespace"
        }
    }))
}

// ── MultiCluster ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ClusterInfo {
    pub name: String,
    pub status: String,
    pub endpoint: String,
    pub region: String,
    pub nodes: u32,
    pub pods: u32,
    pub latency_ms: f64,
    pub last_sync: String,
    pub cilium_version: String,
}

pub async fn list_clusters(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let clusters = vec![
        ClusterInfo { name: "us-east-prod".into(), status: "connected".into(), endpoint: "https://10.1.0.1:6443".into(),
            region: "us-east-1".into(), nodes: 12, pods: 450, latency_ms: 2.1,
            last_sync: "2026-04-03T10:00:00Z".into(), cilium_version: "1.15.3".into() },
        ClusterInfo { name: "eu-west-prod".into(), status: "connected".into(), endpoint: "https://10.2.0.1:6443".into(),
            region: "eu-west-1".into(), nodes: 8, pods: 280, latency_ms: 45.3,
            last_sync: "2026-04-03T09:59:00Z".into(), cilium_version: "1.15.3".into() },
        ClusterInfo { name: "ap-south-staging".into(), status: "degraded".into(), endpoint: "https://10.3.0.1:6443".into(),
            region: "ap-south-1".into(), nodes: 4, pods: 95, latency_ms: 120.8,
            last_sync: "2026-04-03T09:45:00Z".into(), cilium_version: "1.15.2".into() },
    ];
    Json(serde_json::json!({ "clusters": clusters }))
}

pub async fn sync_cluster(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({ "status": "syncing", "message": "Policy sync initiated" }))
}

// ── Heatmap ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct HeatmapCell {
    pub source_namespace: String,
    pub destination_namespace: String,
    pub flow_count: u64,
    pub dropped_count: u64,
    pub avg_latency_ms: f64,
}

pub async fn heatmap_data(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let namespaces = vec!["default", "kube-system", "monitoring", "cilium", "production"];
    let cells = vec![
        HeatmapCell { source_namespace: "default".into(), destination_namespace: "kube-system".into(), flow_count: 8500, dropped_count: 12, avg_latency_ms: 1.2 },
        HeatmapCell { source_namespace: "default".into(), destination_namespace: "monitoring".into(), flow_count: 3200, dropped_count: 0, avg_latency_ms: 2.1 },
        HeatmapCell { source_namespace: "default".into(), destination_namespace: "default".into(), flow_count: 45000, dropped_count: 85, avg_latency_ms: 0.8 },
        HeatmapCell { source_namespace: "monitoring".into(), destination_namespace: "default".into(), flow_count: 12000, dropped_count: 56, avg_latency_ms: 1.5 },
        HeatmapCell { source_namespace: "monitoring".into(), destination_namespace: "kube-system".into(), flow_count: 5600, dropped_count: 0, avg_latency_ms: 1.1 },
        HeatmapCell { source_namespace: "kube-system".into(), destination_namespace: "default".into(), flow_count: 2100, dropped_count: 3, avg_latency_ms: 0.9 },
        HeatmapCell { source_namespace: "production".into(), destination_namespace: "default".into(), flow_count: 28000, dropped_count: 200, avg_latency_ms: 3.2 },
        HeatmapCell { source_namespace: "production".into(), destination_namespace: "kube-system".into(), flow_count: 4500, dropped_count: 8, avg_latency_ms: 1.3 },
        HeatmapCell { source_namespace: "cilium".into(), destination_namespace: "kube-system".into(), flow_count: 9200, dropped_count: 0, avg_latency_ms: 0.5 },
    ];
    Json(serde_json::json!({ "cells": cells, "namespaces": namespaces }))
}

// ── Service Dependencies ────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ServiceDep {
    pub source: String,
    pub destination: String,
    pub protocol: String,
    pub port: u16,
    pub request_rate: f64,
    pub error_rate: f64,
    pub latency_p50: f64,
    pub latency_p99: f64,
}

pub async fn list_dependencies(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let deps = vec![
        ServiceDep { source: "frontend".into(), destination: "api-gateway".into(), protocol: "HTTP".into(), port: 8080, request_rate: 1200.0, error_rate: 0.5, latency_p50: 12.0, latency_p99: 85.0 },
        ServiceDep { source: "api-gateway".into(), destination: "backend".into(), protocol: "gRPC".into(), port: 9090, request_rate: 800.0, error_rate: 0.2, latency_p50: 8.0, latency_p99: 45.0 },
        ServiceDep { source: "backend".into(), destination: "redis".into(), protocol: "TCP".into(), port: 6379, request_rate: 3500.0, error_rate: 0.0, latency_p50: 0.5, latency_p99: 2.0 },
        ServiceDep { source: "backend".into(), destination: "postgres".into(), protocol: "TCP".into(), port: 5432, request_rate: 600.0, error_rate: 0.1, latency_p50: 3.0, latency_p99: 25.0 },
        ServiceDep { source: "api-gateway".into(), destination: "auth-service".into(), protocol: "HTTP".into(), port: 8081, request_rate: 400.0, error_rate: 1.2, latency_p50: 15.0, latency_p99: 120.0 },
        ServiceDep { source: "frontend".into(), destination: "cdn".into(), protocol: "HTTPS".into(), port: 443, request_rate: 5000.0, error_rate: 0.0, latency_p50: 2.0, latency_p99: 10.0 },
    ];
    Json(serde_json::json!({ "dependencies": deps }))
}

// ── Security Dashboard ──────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SecurityFinding {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub resource: String,
    pub namespace: String,
    pub remediation: String,
    pub status: String,
}

pub async fn list_security_findings(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let findings = vec![
        SecurityFinding { id: "sec-001".into(), category: "network_policy".into(), severity: "high".into(),
            title: "Namespace without network policies".into(), description: "Namespace 'production' has no CiliumNetworkPolicy applied".into(),
            resource: "namespace/production".into(), namespace: "production".into(),
            remediation: "Apply default-deny ingress/egress policy".into(), status: "open".into() },
        SecurityFinding { id: "sec-002".into(), category: "encryption".into(), severity: "medium".into(),
            title: "Unencrypted pod-to-pod traffic".into(), description: "WireGuard encryption not enabled for inter-node traffic".into(),
            resource: "ciliumconfig/cilium".into(), namespace: "kube-system".into(),
            remediation: "Enable encryption.type=wireguard in Cilium Helm values".into(), status: "open".into() },
        SecurityFinding { id: "sec-003".into(), category: "identity".into(), severity: "low".into(),
            title: "Pods using default service account".into(), description: "12 pods in default namespace use default ServiceAccount".into(),
            resource: "serviceaccount/default".into(), namespace: "default".into(),
            remediation: "Create dedicated ServiceAccounts with RBAC bindings".into(), status: "acknowledged".into() },
        SecurityFinding { id: "sec-004".into(), category: "least_privilege".into(), severity: "critical".into(),
            title: "Allow-all egress policy detected".into(), description: "Policy 'legacy-allow-all' permits all egress from namespace default".into(),
            resource: "ciliumnetworkpolicy/legacy-allow-all".into(), namespace: "default".into(),
            remediation: "Replace with specific egress rules per service".into(), status: "open".into() },
    ];
    Json(serde_json::json!({ "findings": findings }))
}

pub async fn zero_trust_score(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    Json(serde_json::json!({
        "overall": 68,
        "network_segmentation": 75,
        "identity_verification": 82,
        "encryption": 45,
        "least_privilege": 55,
        "monitoring": 88
    }))
}

// ── eBPF Profiler ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct EbpfProgram {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub prog_type: String,
    pub attach_point: String,
    pub run_count: u64,
    pub run_time_ns: u64,
    pub avg_run_time_ns: u64,
    pub map_count: u32,
    pub loaded_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EbpfMapInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub map_type: String,
    pub key_size: u32,
    pub value_size: u32,
    pub max_entries: u32,
    pub current_entries: u32,
}

pub async fn list_ebpf_programs(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let programs = vec![
        EbpfProgram { id: "prog-1".into(), name: "cil_from_container".into(), prog_type: "tc".into(),
            attach_point: "eth0 ingress".into(), run_count: 15_000_000, run_time_ns: 450_000_000,
            avg_run_time_ns: 30, map_count: 8, loaded_at: "2026-04-01T00:00:00Z".into() },
        EbpfProgram { id: "prog-2".into(), name: "cil_to_container".into(), prog_type: "tc".into(),
            attach_point: "eth0 egress".into(), run_count: 12_000_000, run_time_ns: 360_000_000,
            avg_run_time_ns: 30, map_count: 6, loaded_at: "2026-04-01T00:00:00Z".into() },
        EbpfProgram { id: "prog-3".into(), name: "cil_from_host".into(), prog_type: "xdp".into(),
            attach_point: "eth0".into(), run_count: 50_000_000, run_time_ns: 500_000_000,
            avg_run_time_ns: 10, map_count: 4, loaded_at: "2026-04-01T00:00:00Z".into() },
        EbpfProgram { id: "prog-4".into(), name: "cil_sock_ops".into(), prog_type: "sock_ops".into(),
            attach_point: "cgroup/sock_ops".into(), run_count: 8_000_000, run_time_ns: 160_000_000,
            avg_run_time_ns: 20, map_count: 3, loaded_at: "2026-04-01T00:00:00Z".into() },
    ];
    Json(serde_json::json!({ "programs": programs }))
}

pub async fn list_ebpf_maps(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    track_request(&state, |_| {}).await;
    let maps = vec![
        EbpfMapInfo { id: "map-1".into(), name: "cilium_ct_tcp4_global".into(), map_type: "hash".into(), key_size: 36, value_size: 64, max_entries: 524288, current_entries: 12850 },
        EbpfMapInfo { id: "map-2".into(), name: "cilium_ct_any4_global".into(), map_type: "hash".into(), key_size: 36, value_size: 64, max_entries: 262144, current_entries: 4200 },
        EbpfMapInfo { id: "map-3".into(), name: "cilium_ipcache".into(), map_type: "lpm_trie".into(), key_size: 24, value_size: 32, max_entries: 512000, current_entries: 256 },
        EbpfMapInfo { id: "map-4".into(), name: "cilium_policy".into(), map_type: "hash".into(), key_size: 48, value_size: 24, max_entries: 65536, current_entries: 180 },
        EbpfMapInfo { id: "map-5".into(), name: "cilium_lxc".into(), map_type: "hash".into(), key_size: 16, value_size: 96, max_entries: 65536, current_entries: 42 },
        EbpfMapInfo { id: "map-6".into(), name: "cilium_metrics".into(), map_type: "percpu_hash".into(), key_size: 8, value_size: 16, max_entries: 1024, current_entries: 64 },
    ];
    Json(serde_json::json!({ "maps": maps }))
}

// ── Metrics Summary ─────────────────────────────────────────

pub async fn metrics_summary(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let m = state.metrics.read().await;
    Json(serde_json::json!({
        "total_requests": m.total_requests,
        "total_errors": m.total_errors,
        "total_queries": m.hubble_queries + m.k8s_queries,
        "cache_hits": m.cache_hits,
        "cache_misses": m.cache_misses,
        "uptime_seconds": 86400
    }))
}
