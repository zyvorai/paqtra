// Authentication middleware
use axum::{
    extract::{Request, State},
    http::{header, Method, StatusCode},
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

/// Authenticated users of any role may change their own password.
const SELF_SERVICE_PATH: &str = "/api/v1/auth/password";

/// Turn a validly signed token into the caller's *current* identity.
///
/// The configured admin account (ADMIN_USERNAME) is trusted as issued. Any other
/// subject must be an existing, enabled local user whose password has not been
/// changed since the token was issued; the role comes from the store, not the
/// token, so a role change or a disabled account applies immediately.
pub async fn resolve_claims(state: &AppState, mut claims: Claims) -> Result<Claims, &'static str> {
    if claims.sub == state.config.admin_username {
        return Ok(claims);
    }
    match crate::services::users::get(state, &claims.sub).await {
        Some(user) if user.accepts_token_issued_at(claims.iat as i64) => {
            claims.role = user.role.as_str().to_string();
            Ok(claims)
        }
        _ => Err("Account is disabled, changed or no longer exists"),
    }
}

/// Anything but `admin` is read-only. Unknown roles fail closed.
fn may_write(role: &str) -> bool {
    role == "admin"
}

fn is_read_method(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
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
        || path == "/api/v1/auth/login"
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
            let claims = match resolve_claims(&state, token_data.claims).await {
                Ok(c) => c,
                Err(msg) => {
                    return (StatusCode::UNAUTHORIZED, Json(json!({ "error": msg }))).into_response();
                }
            };
            if !may_write(&claims.role)
                && !is_read_method(request.method())
                && request.uri().path() != SELF_SERVICE_PATH
            {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({ "error": "Your role is read-only" })),
                )
                    .into_response();
            }
            // Inject claims into request extensions for RBAC checks
            request.extensions_mut().insert(claims);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_admin_may_write() {
        assert!(may_write("admin"));
        for r in ["viewer", "editor", "", "Admin", "root"] {
            assert!(!may_write(r), "{r:?}");
        }
    }

    #[test]
    fn read_methods() {
        for m in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert!(is_read_method(&m));
        }
        for m in [Method::POST, Method::PUT, Method::DELETE, Method::PATCH] {
            assert!(!is_read_method(&m));
        }
    }
}
