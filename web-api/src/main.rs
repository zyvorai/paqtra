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
use tower_http::{
    trace::TraceLayer,
    compression::CompressionLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;

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

    // Initialize services
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis_conn = redis_client.get_connection_manager().await?;
    tracing::info!("Connected to Redis");

    // Build shared application state
    let app_state = std::sync::Arc::new(AppState {
        config: config.clone(),
        redis: redis_conn,
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

// Application state shared across handlers
pub struct AppState {
    pub config: Config,
    pub redis: redis::aio::ConnectionManager,
}
