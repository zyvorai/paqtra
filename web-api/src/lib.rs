// Library root for integration tests and external consumers.
// The binary entry point remains in main.rs.
//
// Only modules that do not depend on AppState are re-exported here,
// keeping the library target self-contained and free of runtime
// dependencies like Redis, Hubble, or Kubernetes.

pub mod config;
pub mod error;
pub mod models;

// Re-export the Claims struct from the auth middleware.
// The full middleware module is not exported because the
// auth_middleware function depends on AppState.
pub mod auth_types {
    use serde::{Deserialize, Serialize};

    /// JWT claims - mirrors the definition in middleware/auth.rs
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Claims {
        pub sub: String,
        pub exp: usize,
        pub iat: usize,
        pub role: String,
    }
}
