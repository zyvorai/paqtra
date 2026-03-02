// CORS middleware
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;

/// Build a restrictive CORS layer.
/// In production, ALLOWED_ORIGINS should be set to the actual frontend origin(s).
pub fn cors_layer() -> CorsLayer {
    let origins_str = std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://localhost:3001".to_string());

    if origins_str.contains("localhost") {
        tracing::warn!("CORS configured with localhost origins - not suitable for production. Set ALLOWED_ORIGINS env var.");
    }

    let origins: Vec<HeaderValue> = origins_str
        .split(',')
        .filter_map(|o| o.trim().parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(3600))
}
