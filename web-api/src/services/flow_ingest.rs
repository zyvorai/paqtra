//! Background Hubble → FlowStore ingestion.

use crate::services::flow_store::{FlowSource, StoredFlow};
use crate::AppState;
use std::sync::Arc;
use std::time::Duration;

const INGEST_INTERVAL_SECS: u64 = 30;
const INGEST_BATCH: usize = 500;

/// Spawn detached ingest loop.
pub fn spawn_flow_ingest(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(INGEST_INTERVAL_SECS));
        // First tick fires immediately — skip so startup isn't blocked on Hubble.
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(e) = ingest_once(&state).await {
                tracing::debug!("flow ingest cycle failed: {e}");
                state
                    .flow_store
                    .record_ingest(false, 0, FlowSource::Unavailable);
            }
        }
    });
}

async fn ingest_once(state: &AppState) -> anyhow::Result<()> {
    let healthy = state.hubble.is_healthy().await;
    if !healthy {
        state
            .flow_store
            .record_ingest(false, 0, FlowSource::Unavailable);
        return Ok(());
    }

    let flows = state.hubble.get_flows(INGEST_BATCH, None).await?;
    let source = FlowSource::HubbleCli;
    let rows: Vec<StoredFlow> = flows
        .iter()
        .map(|f| {
            let reason = if f.verdict.eq_ignore_ascii_case("DROPPED") {
                "dropped"
            } else {
                ""
            };
            StoredFlow::from_flow(f, source, reason)
        })
        .collect();
    let n = state.flow_store.insert_batch(&rows)?;
    state.flow_store.record_ingest(true, n as u64, source);
    tracing::debug!("flow ingest stored {n} flows");
    Ok(())
}

/// One-shot ingest (tests / on-demand refresh).
#[allow(dead_code)]
pub async fn ingest_now(state: &AppState) -> anyhow::Result<usize> {
    let flows = state.hubble.get_flows(INGEST_BATCH, None).await?;
    let source = if state.hubble.is_healthy().await {
        FlowSource::HubbleCli
    } else {
        FlowSource::Unavailable
    };
    let rows: Vec<StoredFlow> = flows
        .iter()
        .map(|f| StoredFlow::from_flow(f, source, ""))
        .collect();
    let n = state.flow_store.insert_batch(&rows)?;
    state.flow_store.record_ingest(!rows.is_empty() || source != FlowSource::Unavailable, n as u64, source);
    Ok(n)
}
