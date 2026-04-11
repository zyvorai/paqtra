// Flow monitoring endpoints
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::AppState;
use crate::error::ApiError;
use crate::models::flow::{Flow, FlowQueryParams};
use super::{track_request, track_error, to_json};

/// Cache key prefix for flow queries
const FLOWS_CACHE_PREFIX: &str = "flows";
/// Cache TTL for flow lists (seconds)
const FLOWS_CACHE_TTL: u64 = 10;
/// Maximum allowed limit to prevent excessively large responses
const MAX_LIMIT: usize = 1000;

pub async fn list_flows(
    State(state): State<Arc<AppState>>,
    Query(params): Query<FlowQueryParams>,
) -> Result<Json<Value>, ApiError> {
    tracing::info!("Fetching flows with params: {:?}", params);

    track_request(&state, |m| { m.hubble_queries.fetch_add(1, Ordering::Relaxed); }).await;

    let limit = params.limit.unwrap_or(100).min(MAX_LIMIT);
    // offset is usize, so it is guaranteed to be non-negative
    let offset = params.offset.unwrap_or(0);
    let cache_key = format!(
        "{}:ns={}:v={}:l={}:o={}",
        FLOWS_CACHE_PREFIX,
        params.namespace.as_deref().unwrap_or("*"),
        params.verdict.as_deref().unwrap_or("*"),
        limit,
        offset
    );

    // Try cache first
    match state.cache.get::<Vec<Flow>>(&cache_key).await {
        Ok(Some(cached_flows)) => {
            tracing::debug!("Cache hit for flows (key={})", cache_key);
            state.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
            state.metrics.flows_fetched.fetch_add(cached_flows.len() as u64, Ordering::Relaxed);

            let total = cached_flows.len();
            let page = apply_pagination(&cached_flows, offset, limit);

            return Ok(Json(json!({
                "flows": page,
                "total": total,
                "limit": limit,
                "offset": offset,
                "cached": true,
            })));
        }
        Ok(None) => {
            tracing::info!(limit = limit, offset = offset, "Cache miss for flows, fetching from Hubble");
            state.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
        Err(e) => {
            // Cache deserialization or connection errors are non-fatal; we fall
            // through to fetch fresh data from Hubble. The warning is logged so
            // operators can investigate recurring cache failures.
            tracing::warn!("Cache read error: {}", e);
        }
    }

    // Fetch from Hubble
    let mut flows = match state
        .hubble
        .get_flows(limit + offset, params.namespace.as_deref())
        .await
    {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to fetch flows from Hubble: {}", e);
            track_error(&state).await;
            return Err(ApiError::InternalError(e.to_string()));
        }
    };

    // Apply verdict filter if present
    if let Some(ref verdict) = params.verdict {
        flows.retain(|f| f.verdict.eq_ignore_ascii_case(verdict));
    }

    // Store in cache (best-effort)
    if let Err(e) = state.cache.set(&cache_key, &flows, FLOWS_CACHE_TTL).await {
        tracing::warn!("Cache write error: {}", e);
    }

    state.metrics.flows_fetched.fetch_add(flows.len() as u64, Ordering::Relaxed);

    let total = flows.len();
    let page = apply_pagination(&flows, offset, limit);

    Ok(Json(json!({
        "flows": page,
        "total": total,
        "limit": limit,
        "offset": offset,
        "cached": false,
    })))
}

pub async fn get_flow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    tracing::info!("Fetching flow: {}", id);

    track_request(&state, |m| { m.hubble_queries.fetch_add(1, Ordering::Relaxed); }).await;

    // Try cache
    let cache_key = format!("flow:{}", id);
    if let Ok(Some(flow)) = state.cache.get::<Flow>(&cache_key).await {
        state.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
        return Ok(Json(to_json(&flow)));
    }

    // Fetch a batch and find by id
    let flows = match state.hubble.get_flows(500, None).await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to fetch flows from Hubble: {}", e);
            return Err(ApiError::InternalError(e.to_string()));
        }
    };

    match flows.into_iter().find(|f| f.id == id) {
        Some(flow) => {
            // Cache the individual flow
            let _ = state.cache.set(&cache_key, &flow, 30).await;
            Ok(Json(to_json(&flow)))
        }
        None => {
            Err(ApiError::NotFound)
        }
    }
}

pub async fn flow_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    track_request(&state, |m| { m.hubble_queries.fetch_add(1, Ordering::Relaxed); }).await;

    // Try cache
    let cache_key = "flow_stats";
    if let Ok(Some(stats)) = state
        .cache
        .get::<crate::models::flow::FlowStats>(cache_key)
        .await
    {
        state.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
        return Ok(Json(to_json(&stats)));
    }

    match state.hubble.get_flow_stats().await {
        Ok(stats) => {
            // Cache stats for 5 seconds
            let _ = state.cache.set(cache_key, &stats, 5).await;
            Ok(Json(to_json(&stats)))
        }
        Err(e) => {
            tracing::error!("Failed to compute flow stats: {}", e);
            track_error(&state).await;
            Err(ApiError::InternalError(e.to_string()))
        }
    }
}

/// Apply offset/limit pagination to a slice
fn apply_pagination(flows: &[Flow], offset: usize, limit: usize) -> &[Flow] {
    let start = offset.min(flows.len());
    let end = (start + limit).min(flows.len());
    &flows[start..end]
}
