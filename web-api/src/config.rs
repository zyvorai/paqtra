// Configuration management
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub redis_url: String,
    pub jwt_secret: String,
    pub hubble_address: String,
    pub k8s_context: Option<String>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let jwt_secret = env::var("JWT_SECRET").map_err(|_| {
            anyhow::anyhow!(
                "JWT_SECRET environment variable is required. \
                 Set it to a strong random secret (at least 32 characters)."
            )
        })?;

        if jwt_secret.len() < 32 {
            anyhow::bail!(
                "JWT_SECRET must be at least 32 characters long for security. \
                 Current length: {}",
                jwt_secret.len()
            );
        }

        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "0".to_string())
                .parse()?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            jwt_secret,
            hubble_address: env::var("HUBBLE_ADDRESS")
                .unwrap_or_else(|_| "localhost:4245".to_string()),
            k8s_context: env::var("K8S_CONTEXT").ok(),
        })
    }
}
