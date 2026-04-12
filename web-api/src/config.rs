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
    /// When true, authentication is completely disabled (dev/demo mode only).
    /// Read once at startup from AUTH_DISABLED env var.
    pub auth_disabled: bool,
    /// Directory containing the built web UI static files (index.html, assets/, etc.)
    pub ui_dist_dir: Option<String>,
    /// Optional Prometheus server URL for querying real metrics (e.g. latency percentiles).
    pub prometheus_url: Option<String>,
    /// List of (cluster_name, hubble_address) pairs for multi-cluster aggregation.
    /// Parsed from HUBBLE_ADDRESSES env var as comma-separated `name=host:port` pairs.
    /// Falls back to a single "local" entry derived from `hubble_address`.
    pub hubble_addresses: Vec<(String, String)>,
    /// Optional OTLP HTTP endpoint for exporting trace spans (e.g. "http://localhost:4318").
    /// Read from OTEL_EXPORTER_ENDPOINT env var. When unset, tracing export is disabled.
    pub otel_endpoint: Option<String>,
    /// Service name reported in exported spans. Defaults to "cilium-vision-api".
    /// Read from OTEL_SERVICE_NAME env var.
    pub otel_service_name: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        // Only load .env file in non-production environments
        if env::var("ENVIRONMENT").unwrap_or_default() != "production" {
            dotenvy::dotenv().ok();
        }

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

        let auth_disabled = env::var("AUTH_DISABLED").unwrap_or_default() == "true";
        if auth_disabled {
            let environment = env::var("ENVIRONMENT").unwrap_or_default();
            if environment != "development" && environment != "test" && !environment.is_empty() {
                tracing::error!(
                    "FATAL: AUTH_DISABLED=true is set but ENVIRONMENT='{}' is not 'development' or 'test'. \
                     Refusing to start with authentication disabled in this environment.",
                    environment
                );
                anyhow::bail!(
                    "AUTH_DISABLED=true is not allowed when ENVIRONMENT='{}'. \
                     Set ENVIRONMENT=development or ENVIRONMENT=test, or remove AUTH_DISABLED.",
                    environment
                );
            }
            tracing::warn!(
                "!!! AUTH_DISABLED=true — authentication is completely bypassed !!! \
                 This is only safe in development/test environments."
            );
        }

        let hubble_address =
            env::var("HUBBLE_ADDRESS").unwrap_or_else(|_| "localhost:4245".to_string());

        let hubble_addresses = if let Ok(raw) = env::var("HUBBLE_ADDRESSES") {
            // Parse comma-separated name=host:port pairs
            raw.split(',')
                .filter_map(|entry| {
                    let entry = entry.trim();
                    if entry.is_empty() {
                        return None;
                    }
                    let parts: Vec<&str> = entry.splitn(2, '=').collect();
                    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                        Some((parts[0].to_string(), parts[1].to_string()))
                    } else {
                        tracing::warn!(
                            "Ignoring invalid HUBBLE_ADDRESSES entry '{}': expected name=host:port",
                            entry
                        );
                        None
                    }
                })
                .collect()
        } else {
            // Derive a single entry from the primary hubble_address
            vec![("local".to_string(), hubble_address.clone())]
        };

        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "9191".to_string())
                .parse()?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            jwt_secret,
            hubble_address,
            k8s_context: env::var("K8S_CONTEXT").ok(),
            auth_disabled,
            ui_dist_dir: env::var("UI_DIST_DIR").ok(),
            prometheus_url: env::var("PROMETHEUS_URL").ok(),
            hubble_addresses,
            otel_endpoint: env::var("OTEL_EXPORTER_ENDPOINT").ok(),
            otel_service_name: env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "cilium-vision-api".to_string()),
        })
    }
}
