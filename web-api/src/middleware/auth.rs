// Authentication middleware
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
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
}

/// JWT authentication middleware that validates Bearer tokens.
/// Skips authentication for health and metrics endpoints.
/// Injects decoded Claims into request extensions for downstream RBAC checks.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Skip auth for health/metrics endpoints
    let path = request.uri().path();
    if path == "/health" || path == "/ready" || path == "/metrics" {
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

/// Check if the request has admin role (for destructive operations).
/// Returns None if authorized, or Some(Response) with 403 if not.
pub fn require_admin(claims: Option<&Claims>) -> Option<Response> {
    match claims {
        Some(c) if c.role == "admin" => None,
        Some(_) => Some(
            (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "Admin role required for this operation"})),
            )
                .into_response(),
        ),
        None => Some(
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authentication required"})),
            )
                .into_response(),
        ),
    }
}
