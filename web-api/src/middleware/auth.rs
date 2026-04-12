// Authentication middleware
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub role: String,
    #[serde(default)]
    pub namespaces: Vec<String>, // Empty = all namespaces (for backwards compat)
}

/// JWT authentication middleware that validates Bearer tokens.
/// Skips authentication for health and metrics endpoints.
/// Injects decoded Claims into request extensions for downstream RBAC checks.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Skip auth for health/metrics endpoints and WebSocket upgrades
    // (WebSocket handlers validate tokens via query parameter instead)
    let path = request.uri().path();
    if path == "/health"
        || path == "/ready"
        || path == "/metrics"
        || path.starts_with("/api/v1/ws/")
        || path.starts_with("/api-docs/")
        || path == "/swagger-ui"
    {
        return next.run(request).await;
    }

    // Skip auth when disabled at startup (dev/demo mode only)
    if state.config.auth_disabled {
        return next.run(request).await;
    }

    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header[7..],
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing or invalid Authorization header"})),
            )
                .into_response();
        }
    };

    let decoding_key = DecodingKey::from_secret(state.config.jwt_secret.as_bytes());
    let validation = Validation::new(Algorithm::HS256);

    match decode::<Claims>(token, &decoding_key, &validation) {
        Ok(token_data) => {
            // Inject claims into request extensions for RBAC checks
            request.extensions_mut().insert(token_data.claims);
            next.run(request).await
        }
        Err(e) => {
            tracing::warn!("JWT validation failed: {}", e);
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid or expired token"})),
            )
                .into_response()
        }
    }
}
