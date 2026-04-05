// Simple in-memory rate limiter using a token bucket per IP.
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Instant;

/// Per-IP bucket state.
struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

/// Global rate limiter state shared across requests.
pub struct RateLimiter {
    buckets: Mutex<HashMap<IpAddr, Bucket>>,
    /// Maximum tokens (burst size).
    capacity: f64,
    /// Tokens added per second.
    refill_rate: f64,
}

impl RateLimiter {
    /// Create a new rate limiter.
    /// `capacity` is the burst size, `per_second` is the sustained rate.
    pub fn new(capacity: u32, per_second: u32) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            capacity: capacity as f64,
            refill_rate: per_second as f64,
        }
    }

    /// Try to consume one token for the given IP. Returns true if allowed.
    fn allow(&self, ip: IpAddr) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        let now = Instant::now();

        let bucket = buckets.entry(ip).or_insert(Bucket {
            tokens: self.capacity,
            last_refill: now,
        });

        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.refill_rate).min(self.capacity);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Evict stale entries (call periodically to prevent unbounded growth).
    pub fn cleanup(&self) {
        let mut buckets = self.buckets.lock().unwrap();
        let now = Instant::now();
        buckets.retain(|_, b| now.duration_since(b.last_refill).as_secs() < 300);
    }
}

/// Axum middleware that applies per-IP rate limiting.
pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
) -> Response {
    // Extract client IP from ConnectInfo or X-Forwarded-For
    let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    // Skip rate limiting for health/metrics endpoints
    let path = request.uri().path();
    if path == "/health" || path == "/ready" || path == "/metrics" {
        return next.run(request).await;
    }

    // Use a global static rate limiter (100 req/s burst, 50 sustained per IP)
    static LIMITER: std::sync::OnceLock<RateLimiter> = std::sync::OnceLock::new();
    let limiter = LIMITER.get_or_init(|| RateLimiter::new(100, 50));

    if !limiter.allow(ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [("retry-after", "1")],
            "Rate limit exceeded",
        )
            .into_response();
    }

    next.run(request).await
}
