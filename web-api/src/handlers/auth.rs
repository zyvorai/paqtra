//! Username/password login → JWT.

use axum::{extract::State, http::StatusCode, Json};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::middleware::auth::Claims;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub role: String,
    pub username: String,
    pub expires_in: u64,
}

/// Token lifetime: 24 hours.
const TOKEN_TTL_SECS: u64 = 24 * 60 * 60;

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<Value>)> {
    let user = body.username.trim();
    let pass = body.password;

    if user.is_empty() || pass.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Wrong username or password."})),
        ));
    }

    let ok_user = user == state.config.admin_username;
    // Accept either the configured admin password or the API key as password
    // (Netra-style: operators can sign in with the deploy key as password).
    let ok_pass = pass == state.config.admin_password || pass == state.config.api_key;

    if !(ok_user && ok_pass) {
        tracing::warn!(username = %user, "login failed");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Wrong username or password."})),
        ));
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let claims = Claims {
        sub: user.to_string(),
        exp: (now + TOKEN_TTL_SECS) as usize,
        iat: now as usize,
        role: "admin".to_string(),
        namespaces: vec![],
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|e| {
        tracing::error!("JWT encode failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Could not issue session token"})),
        )
    })?;

    Ok(Json(LoginResponse {
        token,
        role: "admin".to_string(),
        username: user.to_string(),
        expires_in: TOKEN_TTL_SECS,
    }))
}
