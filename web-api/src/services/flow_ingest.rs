//! Background Hubble → FlowStore ingestion.

use crate::models::flow::Flow;
use crate::services::flow_store::{normalize_ts, FlowSource, StoredFlow};
use crate::AppState;
use std::sync::Arc;
use std::time::Duration;

/// Flows requested from Hubble per capture. Public because history responses
/// tell readers how much of the traffic this can capture.
pub const INGEST_BATCH: usize = 500;

/// The rows to store for a batch of flows, and how many flows were left out.
///
/// A flow whose timestamp is missing or unreadable is left out. Stamping it with
/// the time it was polled would put it on the timeline at the wrong moment, and
/// because polls overlap, the same flow would be stored again on every cycle.
/// Real Hubble always sends a time, so this only matters for a broken source.
pub fn rows_for_store(flows: &[Flow], source: FlowSource) -> (Vec<StoredFlow>, usize) {
    let mut skipped = 0;
    let rows = flows
        .iter()
        .filter(|f| {
            let usable = normalize_ts(&f.timestamp).is_some();
            if !usable {
                skipped += 1;
            }
            usable
        })
        .map(|f| {
            let reason = if f.verdict.eq_ignore_ascii_case("DROPPED") {
                "dropped"
            } else {
                ""
            };
            StoredFlow::from_flow(f, source, reason)
        })
        .collect();
    (rows, skipped)
}

/// Spawn detached ingest loop.
pub fn spawn_flow_ingest(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_secs(state.config.flow_ingest_interval_secs));
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
    let (rows, skipped) = rows_for_store(&flows, source);
    if skipped > 0 && state.flow_store.note_skipped_no_time(skipped as u64) == 0 {
        // Once, not every cycle: a persistent condition should not flood the log.
        tracing::warn!(
            "Hubble returned {skipped} flow(s) with no usable timestamp; these cannot be placed on a timeline and are not stored"
        );
    }
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
    let (rows, skipped) = rows_for_store(&flows, source);
    state.flow_store.note_skipped_no_time(skipped as u64);
    let n = state.flow_store.insert_batch(&rows)?;
    state.flow_store.record_ingest(
        !rows.is_empty() || source != FlowSource::Unavailable,
        n as u64,
        source,
    );
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::flow::FlowEndpoint;
    use crate::services::flow_store::{FlowQuery, FlowStore};
    use crate::services::hubble::parse_hubble_output;

    fn flow(id: &str, ts: &str, verdict: &str) -> Flow {
        let ep = |ns: &str| FlowEndpoint {
            namespace: ns.into(),
            pod: "p".into(),
            ip: String::new(),
        };
        Flow {
            id: id.into(),
            timestamp: ts.into(),
            source: ep("a"),
            destination: ep("b"),
            verdict: verdict.into(),
            protocol: "TCP".into(),
            port: 80,
            http_method: None,
            http_url: None,
            http_code: None,
            cluster: None,
        }
    }

    #[test]
    fn flows_without_a_usable_time_are_left_out_and_counted() {
        let flows = vec![
            flow("ok", "2026-09-24T04:00:00.123456789Z", "FORWARDED"),
            flow("empty", "", "FORWARDED"),
            flow("blank", "   ", "FORWARDED"),
            flow("junk", "yesterday", "DROPPED"),
            flow("date-only", "2026-09-24", "DROPPED"),
            flow("offset", "2026-09-24T09:30:00+05:30", "DROPPED"),
        ];
        let (rows, skipped) = rows_for_store(&flows, FlowSource::HubbleCli);
        assert_eq!(skipped, 4);
        assert_eq!(
            rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["ok", "offset"]
        );
        // Timestamps are stored in the one comparable form.
        assert_eq!(rows[0].ts, "2026-09-24T04:00:00.123456Z");
        assert_eq!(rows[1].ts, "2026-09-24T04:00:00.000000Z");
    }

    #[test]
    fn only_dropped_flows_carry_a_drop_reason() {
        let flows = vec![
            flow("f", "2026-09-24T04:00:00Z", "FORWARDED"),
            flow("d", "2026-09-24T04:00:01Z", "dropped"),
        ];
        let (rows, _) = rows_for_store(&flows, FlowSource::HubbleCli);
        assert_eq!(
            (rows[0].drop_reason.as_str(), rows[1].drop_reason.as_str()),
            ("", "dropped")
        );
    }

    #[test]
    fn nothing_skipped_when_every_flow_has_a_time() {
        let (rows, skipped) = rows_for_store(
            &[flow("a", "2026-09-24T04:00:00Z", "FORWARDED")],
            FlowSource::HubbleCli,
        );
        assert_eq!((rows.len(), skipped), (1, 0));
        let (none, skipped) = rows_for_store(&[], FlowSource::HubbleCli);
        assert_eq!((none.len(), skipped), (0, 0));
    }

    #[test]
    fn overlapping_polls_of_the_same_flows_never_duplicate_history() {
        // Real Hubble output with no uuid (older Cilium): ids are derived from
        // content, so each poll of the sliding window agrees on them.
        let line = |t: &str, pod: &str| {
            serde_json::json!({"flow": {"time": t, "verdict": "FORWARDED",
                "source": {"namespace": "a", "pod_name": pod}, "destination": {"namespace": "b", "pod_name": "q"},
                "l4": {"TCP": {"destination_port": 80}}}, "node_name": "n"})
            .to_string()
        };
        let (l1, l2, l3) = (
            line("2026-09-24T04:00:01Z", "p1"),
            line("2026-09-24T04:00:02Z", "p2"),
            line("2026-09-24T04:00:03Z", "p3"),
        );
        let poll_1 = format!("{l1}\n{l2}\n");
        let poll_2 = format!("{l2}\n{l3}\n"); // the window slid: l2 is now first, not second
        let store = FlowStore::memory_only();
        for out in [&poll_1, &poll_2, &poll_1, &poll_2] {
            let (rows, _) = rows_for_store(&parse_hubble_output(out, None), FlowSource::HubbleCli);
            store.insert_batch(&rows).unwrap();
        }
        assert_eq!(
            store.count(&FlowQuery::default()).unwrap(),
            3,
            "three distinct flows, however often they are polled"
        );
    }

    #[test]
    fn the_store_counts_skipped_flows_and_reports_the_first_time() {
        let store = FlowStore::memory_only();
        assert_eq!(store.stats().skipped_no_time, 0);
        assert_eq!(
            store.note_skipped_no_time(3),
            0,
            "first report: caller may log"
        );
        assert_eq!(
            store.note_skipped_no_time(2),
            3,
            "later reports: caller stays quiet"
        );
        assert_eq!(store.stats().skipped_no_time, 5);
    }
}
