// Prometheus metrics endpoint
use axum::{http::StatusCode, response::IntoResponse};

pub async fn prometheus_metrics() -> impl IntoResponse {
    // TODO: Implement Prometheus metrics collection
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4")],
        "# HELP http_requests_total Total HTTP requests\n# TYPE http_requests_total counter\nhttp_requests_total 0\n",
    )
}
