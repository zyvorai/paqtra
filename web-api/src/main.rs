// Cilium Vision Web API Server
mod config;
mod routes;
pub mod handlers;
mod models;
mod services;
mod middleware;
mod websocket;
mod error;

use axum::{
    Router,
    routing::{get, post},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{
    trace::TraceLayer,
    compression::CompressionLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::services::hubble::HubbleService;
use crate::services::k8s::K8sService;
use crate::services::cache::CacheService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cilium_vision_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // Load configuration
    let config = Config::load()?;
    tracing::info!("Configuration loaded");

    // Initialize Redis
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis_conn = redis_client.get_connection_manager().await?;
    tracing::info!("Connected to Redis");

    // Initialize services
    let hubble = HubbleService::new(&config.hubble_address);
    tracing::info!("HubbleService initialized (relay: {})", config.hubble_address);

    let k8s = K8sService::new(config.k8s_context.clone());
    tracing::info!("K8sService initialized (context: {:?})", config.k8s_context);

    let cache = CacheService::new(redis_conn.clone());
    tracing::info!("CacheService initialized");

    let metrics = Arc::new(RwLock::new(AppMetrics::default()));

    // Build shared application state
    let app_state = Arc::new(AppState {
        config: config.clone(),
        redis: redis_conn,
        hubble,
        k8s,
        cache,
        metrics,
    });

    // Build API router
    let api_routes = Router::new()
        // Health checks (no auth required - handled by middleware)
        .route("/health", get(handlers::health::health_check))
        .route("/ready", get(handlers::health::readiness_check))

        // Flow monitoring
        .route("/api/v1/flows", get(handlers::flows::list_flows))
        .route("/api/v1/flows/{id}", get(handlers::flows::get_flow))
        .route("/api/v1/flows/stats", get(handlers::flows::flow_stats))

        // Policy management
        .route("/api/v1/policies", get(handlers::policies::list_policies))
        .route("/api/v1/policies", post(handlers::policies::create_policy))
        .route("/api/v1/policies/{id}", get(handlers::policies::get_policy))
        .route("/api/v1/policies/{id}", axum::routing::put(handlers::policies::update_policy))
        .route("/api/v1/policies/{id}", axum::routing::delete(handlers::policies::delete_policy))
        .route("/api/v1/policies/simulate", post(handlers::policies::simulate_policy))

        // Anomaly detection
        .route("/api/v1/anomalies", get(handlers::anomalies::list_anomalies))
        .route("/api/v1/anomalies/{id}", get(handlers::anomalies::get_anomaly))
        .route("/api/v1/anomalies/{id}/remediate", post(handlers::anomalies::remediate_anomaly))

        // Compliance
        .route("/api/v1/compliance/frameworks", get(handlers::compliance::list_frameworks))
        .route("/api/v1/compliance/audit", post(handlers::compliance::run_audit))
        .route("/api/v1/security/posture", get(handlers::compliance::security_posture))

        // Intelligence modules
        .route("/api/v1/modules/autopolicy/generate", post(handlers::modules::generate_autopolicy))
        .route("/api/v1/modules/chaos/experiments", get(handlers::modules::list_chaos_experiments))
        .route("/api/v1/modules/chaos/run", post(handlers::modules::run_chaos_experiment))
        .route("/api/v1/modules/canary/{id}", get(handlers::modules::canary_status))

        // Events, Endpoints, Nodes
        .route("/api/v1/events", get(handlers::events::list_events))
        .route("/api/v1/endpoints", get(handlers::endpoints::list_endpoints))
        .route("/api/v1/nodes", get(handlers::nodes::list_nodes))

        // Replay
        .route("/api/v1/modules/replay/recordings", get(handlers::extended::list_recordings))
        .route("/api/v1/modules/replay/start", post(handlers::extended::start_recording))
        .route("/api/v1/modules/replay/{id}/stop", post(handlers::extended::stop_recording))

        // Healer
        .route("/api/v1/modules/healer/problems", get(handlers::extended::list_healer_problems))
        .route("/api/v1/modules/healer/{id}/fix", post(handlers::extended::apply_healer_fix))

        // RootCause
        .route("/api/v1/modules/rootcause/drops", get(handlers::extended::list_packet_drops))
        .route("/api/v1/modules/rootcause/analyze", post(handlers::extended::analyze_drops))

        // MultiCluster
        .route("/api/v1/modules/multicluster/clusters", get(handlers::extended::list_clusters))
        .route("/api/v1/modules/multicluster/{name}/sync", post(handlers::extended::sync_cluster))

        // Heatmap & Dependencies
        .route("/api/v1/heatmap", get(handlers::extended::heatmap_data))
        .route("/api/v1/dependencies", get(handlers::extended::list_dependencies))

        // Security Dashboard
        .route("/api/v1/security/findings", get(handlers::extended::list_security_findings))
        .route("/api/v1/security/zero-trust", get(handlers::extended::zero_trust_score))

        // eBPF Profiler
        .route("/api/v1/modules/ebpf/programs", get(handlers::extended::list_ebpf_programs))
        .route("/api/v1/modules/ebpf/maps", get(handlers::extended::list_ebpf_maps))

        // Metrics summary
        .route("/api/v1/metrics/summary", get(handlers::extended::metrics_summary))

        // Host Info
        .route("/api/v1/host/info", get(handlers::extended2::host_info))

        // Policy Templates
        .route("/api/v1/policies/templates", get(handlers::extended2::list_policy_templates))
        .route("/api/v1/policies/templates/{id}/apply", post(handlers::extended2::apply_template))

        // Diagnostics
        .route("/api/v1/diagnostics/run", post(handlers::extended2::run_diagnostics))
        .route("/api/v1/diagnostics/connectivity", post(handlers::extended2::connectivity_test))

        // Audit Log
        .route("/api/v1/audit/log", get(handlers::extended2::audit_log))

        // Alerts
        .route("/api/v1/alerts/rules", get(handlers::extended2::list_alert_rules))
        .route("/api/v1/alerts/history", get(handlers::extended2::alert_history))
        .route("/api/v1/alerts/rules/{id}", axum::routing::put(handlers::extended2::toggle_alert_rule))

        // Service Map
        .route("/api/v1/servicemap", get(handlers::extended2::service_map))

        // Packet Capture
        .route("/api/v1/modules/capture/sessions", get(handlers::extended2::list_capture_sessions))
        .route("/api/v1/modules/capture/start", post(handlers::extended2::start_capture))
        .route("/api/v1/modules/capture/{id}/stop", post(handlers::extended2::stop_capture))

        // DNS Monitor
        .route("/api/v1/dns/queries", get(handlers::extended2::dns_queries))
        .route("/api/v1/dns/stats", get(handlers::extended2::dns_stats))

        // Identities
        .route("/api/v1/identities", get(handlers::extended2::list_identities))

        // Cluster Mesh
        .route("/api/v1/clustermesh/peers", get(handlers::extended2::list_mesh_peers))
        .route("/api/v1/clustermesh/connect", post(handlers::extended2::connect_mesh_peer))

        // BGP Peering
        .route("/api/v1/bgp/peers", get(handlers::extended2::list_bgp_peers))

        // Bandwidth
        .route("/api/v1/bandwidth", get(handlers::extended2::bandwidth_data))

        // Cost & Forecasting
        .route("/api/v1/costs/breakdown", get(handlers::extended3::cost_breakdown))
        .route("/api/v1/forecast/metrics", get(handlers::extended3::forecast_metrics))
        .route("/api/v1/forecast/{metric}", get(handlers::extended3::forecast_data))

        // Encryption
        .route("/api/v1/encryption/status", get(handlers::extended3::encryption_status))

        // Load Balancer & Ingress
        .route("/api/v1/loadbalancer/services", get(handlers::extended3::lb_services))
        .route("/api/v1/ingress/routes", get(handlers::extended3::ingress_routes))

        // IPAM
        .route("/api/v1/ipam/pools", get(handlers::extended3::ipam_pools))
        .route("/api/v1/ipam/allocations", get(handlers::extended3::ip_allocations))

        // Latency
        .route("/api/v1/latency/analysis", get(handlers::extended3::latency_analysis))

        // Traffic Mirroring
        .route("/api/v1/modules/mirror/rules", get(handlers::extended3::mirror_rules).post(handlers::extended3::create_mirror_rule))
        .route("/api/v1/modules/mirror/rules/{id}", axum::routing::delete(handlers::extended3::delete_mirror_rule))

        // Cluster Health & RBAC
        .route("/api/v1/cluster/health", get(handlers::extended3::cluster_health))
        .route("/api/v1/rbac/bindings", get(handlers::extended3::rbac_bindings))

        // Network Interfaces
        .route("/api/v1/network/interfaces", get(handlers::extended3::net_interfaces))

        // Troubleshoot
        .route("/api/v1/troubleshoot/run", post(handlers::extended3::run_troubleshoot))

        // WireGuard
        .route("/api/v1/wireguard/peers", get(handlers::extended4::wireguard_peers))
        // Cilium Status
        .route("/api/v1/cilium/status", get(handlers::extended4::cilium_status))
        // Policy Validation
        .route("/api/v1/policies/validate", post(handlers::extended4::validate_policy))
        // Flow Exports
        .route("/api/v1/flows/exports", get(handlers::extended4::list_export_configs).post(handlers::extended4::create_export_config))
        .route("/api/v1/flows/exports/{id}", axum::routing::delete(handlers::extended4::delete_export_config))
        // SLOs
        .route("/api/v1/slo/targets", get(handlers::extended4::list_slos))
        // Incidents
        .route("/api/v1/incidents", get(handlers::extended4::list_incidents))
        // Changes
        .route("/api/v1/changes", get(handlers::extended4::list_changes))
        .route("/api/v1/changes/{id}/rollback", post(handlers::extended4::rollback_change))
        // Node Drain
        .route("/api/v1/nodes/drain/status", get(handlers::extended4::node_drain_status))
        .route("/api/v1/nodes/drain", post(handlers::extended4::drain_node))
        .route("/api/v1/nodes/uncordon", post(handlers::extended4::uncordon_node))
        // Pod Security
        .route("/api/v1/security/pods", get(handlers::extended4::pod_security))
        // Egress Gateway
        .route("/api/v1/egress/policies", get(handlers::extended4::egress_policies))
        // Service Mesh
        .route("/api/v1/servicemesh/services", get(handlers::extended4::mesh_services))
        // KubeProxy Replacement
        .route("/api/v1/kpr/status", get(handlers::extended4::kpr_status))

        // WebSocket endpoints
        .route("/api/v1/ws/flows", get(websocket::flows_websocket))
        .route("/api/v1/ws/metrics", get(websocket::metrics_websocket))

        // Metrics endpoint for Prometheus (no auth required - handled by middleware)
        .route("/metrics", get(handlers::metrics::prometheus_metrics))

        // State
        .with_state(app_state.clone());

    // Configure middleware
    let app = api_routes
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn_with_state(
            app_state,
            middleware::auth::auth_middleware,
        ))
        .layer(middleware::cors::cors_layer())
        .layer(TraceLayer::new_for_http());

    // Start server (port 0 = OS-assigned random port)
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_addr = listener.local_addr()?;
    tracing::info!("Starting Cilium Vision API server on {}", actual_addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Application-level metrics tracked in memory
#[derive(Debug, Clone)]
pub struct AppMetrics {
    pub total_requests: u64,
    pub total_errors: u64,
    pub flows_fetched: u64,
    pub policies_created: u64,
    pub policies_deleted: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hubble_queries: u64,
    pub k8s_queries: u64,
}

impl Default for AppMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            total_errors: 0,
            flows_fetched: 0,
            policies_created: 0,
            policies_deleted: 0,
            cache_hits: 0,
            cache_misses: 0,
            hubble_queries: 0,
            k8s_queries: 0,
        }
    }
}

// Application state shared across handlers
pub struct AppState {
    pub config: Config,
    pub redis: redis::aio::ConnectionManager,
    pub hubble: HubbleService,
    pub k8s: K8sService,
    pub cache: CacheService,
    pub metrics: Arc<RwLock<AppMetrics>>,
}
