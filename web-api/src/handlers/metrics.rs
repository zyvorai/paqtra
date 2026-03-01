// Prometheus metrics endpoint
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use std::sync::Arc;

use crate::AppState;

pub async fn prometheus_metrics(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let m = state.metrics.read().await;

    let body = format!(
        "# HELP cilium_vision_http_requests_total Total HTTP requests handled\n\
         # TYPE cilium_vision_http_requests_total counter\n\
         cilium_vision_http_requests_total {}\n\
         \n\
         # HELP cilium_vision_http_errors_total Total HTTP errors\n\
         # TYPE cilium_vision_http_errors_total counter\n\
         cilium_vision_http_errors_total {}\n\
         \n\
         # HELP cilium_vision_flows_fetched_total Total flows fetched from Hubble\n\
         # TYPE cilium_vision_flows_fetched_total counter\n\
         cilium_vision_flows_fetched_total {}\n\
         \n\
         # HELP cilium_vision_policies_created_total Total policies created\n\
         # TYPE cilium_vision_policies_created_total counter\n\
         cilium_vision_policies_created_total {}\n\
         \n\
         # HELP cilium_vision_policies_deleted_total Total policies deleted\n\
         # TYPE cilium_vision_policies_deleted_total counter\n\
         cilium_vision_policies_deleted_total {}\n\
         \n\
         # HELP cilium_vision_cache_hits_total Cache hits\n\
         # TYPE cilium_vision_cache_hits_total counter\n\
         cilium_vision_cache_hits_total {}\n\
         \n\
         # HELP cilium_vision_cache_misses_total Cache misses\n\
         # TYPE cilium_vision_cache_misses_total counter\n\
         cilium_vision_cache_misses_total {}\n\
         \n\
         # HELP cilium_vision_hubble_queries_total Hubble queries issued\n\
         # TYPE cilium_vision_hubble_queries_total counter\n\
         cilium_vision_hubble_queries_total {}\n\
         \n\
         # HELP cilium_vision_k8s_queries_total Kubernetes API queries issued\n\
         # TYPE cilium_vision_k8s_queries_total counter\n\
         cilium_vision_k8s_queries_total {}\n",
        m.total_requests,
        m.total_errors,
        m.flows_fetched,
        m.policies_created,
        m.policies_deleted,
        m.cache_hits,
        m.cache_misses,
        m.hubble_queries,
        m.k8s_queries,
    );

    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}
