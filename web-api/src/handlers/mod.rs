// HTTP request handlers
pub mod health;
pub mod flows;
pub mod policies;
pub mod anomalies;
pub mod compliance;
pub mod modules;
pub mod metrics;
pub mod events;
pub mod endpoints;
pub mod nodes;
pub mod extended;
pub mod extended2;
pub mod extended3;
pub mod extended4;

use crate::{AppMetrics, AppState};
use serde::Serialize;
use serde_json::Value;

/// Increment total_requests and run an extra closure on the metrics.
pub async fn track_request(state: &AppState, f: impl FnOnce(&mut AppMetrics)) {
    let mut m = state.metrics.write().await;
    m.total_requests += 1;
    f(&mut m);
}

/// Increment total_errors.
pub async fn track_error(state: &AppState) {
    let mut m = state.metrics.write().await;
    m.total_errors += 1;
}

/// Serialize to JSON with empty-object fallback.
pub fn to_json<T: Serialize>(val: &T) -> Value {
    serde_json::to_value(val).unwrap_or_else(|_| serde_json::json!({}))
}
