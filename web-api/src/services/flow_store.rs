//! Durable flow index (SQLite when PAQTRA_DATA_DIR is set, else in-memory).
//!
//! Stores Hubble flow metadata for investigation and policy preview.
//! Never stores payloads, argv, or secrets.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::models::flow::Flow;

pub const DEFAULT_RETENTION_DAYS: i64 = 7;
const MAX_MEMORY_FLOWS: usize = 50_000;

/// Provenance of a stored flow row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowSource {
    HubbleCli,
    HubbleGrpc,
    Unavailable,
}

impl FlowSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HubbleCli => "hubble_cli",
            Self::HubbleGrpc => "hubble_grpc",
            Self::Unavailable => "unavailable",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "hubble_grpc" => Self::HubbleGrpc,
            "unavailable" => Self::Unavailable,
            _ => Self::HubbleCli,
        }
    }
}

/// Indexed flow row with investigation fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFlow {
    pub id: String,
    pub ts: String,
    pub cluster: String,
    pub verdict: String,
    pub drop_reason: String,
    pub protocol: String,
    pub port: u16,
    pub src_namespace: String,
    pub src_pod: String,
    pub src_ip: String,
    pub src_identity: i64,
    pub dst_namespace: String,
    pub dst_pod: String,
    pub dst_ip: String,
    pub dst_identity: i64,
    pub source: FlowSource,
}

/// Fixed-width UTC timestamp (`2026-09-24T04:34:47.290678Z`). Stored and queried
/// in this one form so that comparing timestamps as text is the same as
/// comparing them in time; mixed offsets and fraction lengths would not be.
pub fn format_ts(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Micros, true)
}

pub fn now_ts() -> String {
    format_ts(Utc::now())
}

/// Parse any RFC 3339 timestamp into the stored form.
pub fn normalize_ts(s: &str) -> Option<String> {
    DateTime::parse_from_rfc3339(s.trim())
        .ok()
        .map(|t| format_ts(t.with_timezone(&Utc)))
}

impl StoredFlow {
    pub fn from_flow(flow: &Flow, source: FlowSource, drop_reason: &str) -> Self {
        Self {
            id: flow.id.clone(),
            ts: normalize_ts(&flow.timestamp).unwrap_or_else(now_ts),
            cluster: flow.cluster.clone().unwrap_or_else(|| "local".into()),
            verdict: flow.verdict.clone(),
            drop_reason: drop_reason.to_string(),
            protocol: flow.protocol.clone(),
            port: flow.port,
            src_namespace: flow.source.namespace.clone(),
            src_pod: flow.source.pod.clone(),
            src_ip: flow.source.ip.clone(),
            src_identity: 0,
            dst_namespace: flow.destination.namespace.clone(),
            dst_pod: flow.destination.pod.clone(),
            dst_ip: flow.destination.ip.clone(),
            dst_identity: 0,
            source,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FlowQuery {
    pub src_namespace: Option<String>,
    pub src_pod: Option<String>,
    pub dst_namespace: Option<String>,
    pub dst_pod: Option<String>,
    /// Matches when either side is in this namespace.
    pub namespace: Option<String>,
    /// Matches when either side's pod name contains this text.
    pub pod: Option<String>,
    pub port: Option<u16>,
    pub verdict: Option<String>,
    /// Inclusive lower bound, in the stored timestamp form.
    pub since_rfc3339: Option<String>,
    /// Exclusive upper bound, in the stored timestamp form.
    pub until_rfc3339: Option<String>,
    /// When set, only flows with a side in one of these namespaces. This is a
    /// caller's namespace limit; applying it here keeps counts and paging right.
    pub scope: Option<Vec<String>>,
    pub limit: usize,
    pub offset: usize,
}

/// Escape `\`, `%` and `_` so user text matches literally in a LIKE.
fn like_contains(v: &str) -> String {
    let mut out = String::with_capacity(v.len() + 2);
    out.push('%');
    for c in v.chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

impl FlowQuery {
    /// SQL conditions (each starting with " AND") and their bound values.
    fn where_sql(&self) -> (String, Vec<Box<dyn rusqlite::types::ToSql>>) {
        let mut sql = String::new();
        let mut vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut eq = |col: &str, v: &Option<String>| {
            if let Some(v) = v {
                sql.push_str(&format!(" AND {col} = ?"));
                vals.push(Box::new(v.clone()));
            }
        };
        eq("src_namespace", &self.src_namespace);
        eq("dst_namespace", &self.dst_namespace);
        eq("verdict", &self.verdict);
        if let Some(v) = &self.src_pod {
            sql.push_str(" AND src_pod LIKE ? ESCAPE '\\'");
            vals.push(Box::new(like_contains(v)));
        }
        if let Some(v) = &self.dst_pod {
            sql.push_str(" AND dst_pod LIKE ? ESCAPE '\\'");
            vals.push(Box::new(like_contains(v)));
        }
        if let Some(v) = &self.namespace {
            sql.push_str(" AND (src_namespace = ? OR dst_namespace = ?)");
            vals.push(Box::new(v.clone()));
            vals.push(Box::new(v.clone()));
        }
        if let Some(v) = &self.pod {
            sql.push_str(" AND (src_pod LIKE ? ESCAPE '\\' OR dst_pod LIKE ? ESCAPE '\\')");
            vals.push(Box::new(like_contains(v)));
            vals.push(Box::new(like_contains(v)));
        }
        if let Some(p) = self.port {
            sql.push_str(" AND port = ?");
            vals.push(Box::new(p as i64));
        }
        if let Some(v) = &self.since_rfc3339 {
            sql.push_str(" AND ts >= ?");
            vals.push(Box::new(v.clone()));
        }
        if let Some(v) = &self.until_rfc3339 {
            sql.push_str(" AND ts < ?");
            vals.push(Box::new(v.clone()));
        }
        if let Some(scope) = &self.scope {
            if scope.is_empty() {
                // An empty scope list would mean "everything" elsewhere; here it
                // is a caller bug, so match nothing rather than everything.
                sql.push_str(" AND 0");
            } else {
                let marks = vec!["?"; scope.len()].join(",");
                sql.push_str(&format!(
                    " AND (src_namespace IN ({marks}) OR dst_namespace IN ({marks}))"
                ));
                for _ in 0..2 {
                    for ns in scope {
                        vals.push(Box::new(ns.clone()));
                    }
                }
            }
        }
        (sql, vals)
    }

    /// The same conditions for the in-memory store.
    fn matches(&self, f: &StoredFlow) -> bool {
        let eq = |want: &Option<String>, got: &str| want.as_deref().is_none_or(|w| w == got);
        let has =
            |want: &Option<String>, got: &str| want.as_deref().is_none_or(|w| got.contains(w));
        eq(&self.src_namespace, &f.src_namespace)
            && eq(&self.dst_namespace, &f.dst_namespace)
            && eq(&self.verdict, &f.verdict)
            && has(&self.src_pod, &f.src_pod)
            && has(&self.dst_pod, &f.dst_pod)
            && self
                .namespace
                .as_deref()
                .is_none_or(|n| f.src_namespace == n || f.dst_namespace == n)
            && self
                .pod
                .as_deref()
                .is_none_or(|p| f.src_pod.contains(p) || f.dst_pod.contains(p))
            && self.port.is_none_or(|p| f.port == p)
            && self
                .since_rfc3339
                .as_deref()
                .is_none_or(|s| f.ts.as_str() >= s)
            && self
                .until_rfc3339
                .as_deref()
                .is_none_or(|u| f.ts.as_str() < u)
            && self.scope.as_ref().is_none_or(|scope| {
                scope
                    .iter()
                    .any(|ns| *ns == f.src_namespace || *ns == f.dst_namespace)
            })
    }
}

/// Flows per time bucket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TimelineBucket {
    /// Start of the bucket, unix seconds.
    pub start: i64,
    pub forwarded: u64,
    pub dropped: u64,
    pub other: u64,
}

/// What the store holds, for telling a reader how far back the history goes.
#[derive(Debug, Clone, Serialize)]
pub struct Coverage {
    pub oldest: Option<String>,
    pub newest: Option<String>,
    pub total: u64,
    pub retention_days: i64,
    /// False when running in memory: history is lost on restart and capped.
    pub durable: bool,
}

#[derive(Debug, Default)]
pub struct FlowStoreStats {
    pub total: u64,
    pub last_ingest_ok: bool,
    pub last_ingest_at: Option<String>,
    pub last_ingest_count: u64,
    pub ingest_source: String,
}

pub struct FlowStore {
    db: Option<Arc<Mutex<Connection>>>,
    memory: Mutex<Vec<StoredFlow>>,
    retention_days: i64,
    pub ingested_total: AtomicU64,
    pub last_ingest_ok: AtomicU64, // 1 = ok, 0 = fail
    pub last_ingest_count: AtomicU64,
    last_ingest_at: Mutex<Option<String>>,
    ingest_source: Mutex<String>,
}

impl FlowStore {
    /// In-memory only (no PAQTRA_DATA_DIR).
    pub fn memory_only() -> Self {
        Self {
            db: None,
            memory: Mutex::new(Vec::new()),
            retention_days: DEFAULT_RETENTION_DAYS,
            ingested_total: AtomicU64::new(0),
            last_ingest_ok: AtomicU64::new(0),
            last_ingest_count: AtomicU64::new(0),
            last_ingest_at: Mutex::new(None),
            ingest_source: Mutex::new("unavailable".into()),
        }
    }

    /// Open/create flows table in the same directory as the cache DB.
    pub fn open(data_dir: &Path, retention_days: i64) -> Result<Self> {
        std::fs::create_dir_all(data_dir)
            .with_context(|| format!("create data dir {}", data_dir.display()))?;
        let path: PathBuf = data_dir.join("flows.db");
        let conn = Connection::open(&path).with_context(|| format!("open {}", path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS flows (
                id TEXT NOT NULL,
                ts TEXT NOT NULL,
                cluster TEXT NOT NULL DEFAULT 'local',
                verdict TEXT NOT NULL,
                drop_reason TEXT NOT NULL DEFAULT '',
                protocol TEXT NOT NULL,
                port INTEGER NOT NULL,
                src_namespace TEXT NOT NULL DEFAULT '',
                src_pod TEXT NOT NULL DEFAULT '',
                src_ip TEXT NOT NULL DEFAULT '',
                src_identity INTEGER NOT NULL DEFAULT 0,
                dst_namespace TEXT NOT NULL DEFAULT '',
                dst_pod TEXT NOT NULL DEFAULT '',
                dst_ip TEXT NOT NULL DEFAULT '',
                dst_identity INTEGER NOT NULL DEFAULT 0,
                source TEXT NOT NULL DEFAULT 'hubble_cli',
                PRIMARY KEY (id, ts)
            );
            CREATE INDEX IF NOT EXISTS idx_flows_ts ON flows(ts DESC);
            CREATE INDEX IF NOT EXISTS idx_flows_path ON flows(src_namespace, src_pod, dst_namespace, dst_pod, port);
            CREATE INDEX IF NOT EXISTS idx_flows_verdict ON flows(verdict);
            ",
        )?;
        tracing::info!("FlowStore ready at {}", path.display());
        let store = Self {
            db: Some(Arc::new(Mutex::new(conn))),
            memory: Mutex::new(Vec::new()),
            retention_days,
            ingested_total: AtomicU64::new(0),
            last_ingest_ok: AtomicU64::new(0),
            last_ingest_count: AtomicU64::new(0),
            last_ingest_at: Mutex::new(None),
            ingest_source: Mutex::new("unavailable".into()),
        };
        let _ = store.purge_expired();
        Ok(store)
    }

    pub fn stats(&self) -> FlowStoreStats {
        let total = if let Some(db) = &self.db {
            db.lock()
                .ok()
                .and_then(|c| {
                    c.query_row("SELECT COUNT(*) FROM flows", [], |r| r.get::<_, i64>(0))
                        .ok()
                })
                .unwrap_or(0) as u64
        } else {
            self.memory.lock().map(|m| m.len() as u64).unwrap_or(0)
        };
        FlowStoreStats {
            total,
            last_ingest_ok: self.last_ingest_ok.load(Ordering::Relaxed) == 1,
            last_ingest_at: self.last_ingest_at.lock().ok().and_then(|g| g.clone()),
            last_ingest_count: self.last_ingest_count.load(Ordering::Relaxed),
            ingest_source: self
                .ingest_source
                .lock()
                .map(|g| g.clone())
                .unwrap_or_else(|_| "unavailable".into()),
        }
    }

    pub fn record_ingest(&self, ok: bool, count: u64, source: FlowSource) {
        self.last_ingest_ok
            .store(if ok { 1 } else { 0 }, Ordering::Relaxed);
        self.last_ingest_count.store(count, Ordering::Relaxed);
        self.ingested_total.fetch_add(count, Ordering::Relaxed);
        if let Ok(mut g) = self.last_ingest_at.lock() {
            *g = Some(Utc::now().to_rfc3339());
        }
        if let Ok(mut g) = self.ingest_source.lock() {
            *g = source.as_str().to_string();
        }
    }

    pub fn insert_batch(&self, rows: &[StoredFlow]) -> Result<usize> {
        if rows.is_empty() {
            return Ok(0);
        }
        if let Some(db) = &self.db {
            let conn = db
                .lock()
                .map_err(|_| anyhow::anyhow!("flow db lock poisoned"))?;
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(
                    "INSERT OR REPLACE INTO flows (
                        id, ts, cluster, verdict, drop_reason, protocol, port,
                        src_namespace, src_pod, src_ip, src_identity,
                        dst_namespace, dst_pod, dst_ip, dst_identity, source
                    ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
                )?;
                for r in rows {
                    stmt.execute(params![
                        r.id,
                        r.ts,
                        r.cluster,
                        r.verdict,
                        r.drop_reason,
                        r.protocol,
                        r.port as i64,
                        r.src_namespace,
                        r.src_pod,
                        r.src_ip,
                        r.src_identity,
                        r.dst_namespace,
                        r.dst_pod,
                        r.dst_ip,
                        r.dst_identity,
                        r.source.as_str(),
                    ])?;
                }
            }
            tx.commit()?;
            // Drop the connection lock before purge (purge re-locks).
            drop(conn);
            let _ = self.purge_expired();
            return Ok(rows.len());
        }

        let mut mem = self
            .memory
            .lock()
            .map_err(|_| anyhow::anyhow!("flow memory lock poisoned"))?;
        for r in rows {
            if let Some(pos) = mem.iter().position(|x| x.id == r.id && x.ts == r.ts) {
                mem[pos] = r.clone();
            } else {
                mem.push(r.clone());
            }
        }
        if mem.len() > MAX_MEMORY_FLOWS {
            let drop_n = mem.len() - MAX_MEMORY_FLOWS;
            mem.drain(0..drop_n);
        }
        Ok(rows.len())
    }

    pub fn purge_expired(&self) -> Result<usize> {
        let cutoff = format_ts(Utc::now() - ChronoDuration::days(self.retention_days));
        if let Some(db) = &self.db {
            let conn = db
                .lock()
                .map_err(|_| anyhow::anyhow!("flow db lock poisoned"))?;
            let n = conn.execute("DELETE FROM flows WHERE ts < ?1", params![cutoff])?;
            return Ok(n);
        }
        let mut mem = self
            .memory
            .lock()
            .map_err(|_| anyhow::anyhow!("flow memory lock poisoned"))?;
        let before = mem.len();
        mem.retain(|f| f.ts >= cutoff);
        Ok(before.saturating_sub(mem.len()))
    }

    pub fn query(&self, q: &FlowQuery) -> Result<Vec<StoredFlow>> {
        let limit = q.limit.clamp(1, 5_000);
        let offset = q.offset;

        if let Some(db) = &self.db {
            let conn = db
                .lock()
                .map_err(|_| anyhow::anyhow!("flow db lock poisoned"))?;
            let (conds, mut vals) = q.where_sql();
            let sql = format!(
                "SELECT id, ts, cluster, verdict, drop_reason, protocol, port,
                        src_namespace, src_pod, src_ip, src_identity,
                        dst_namespace, dst_pod, dst_ip, dst_identity, source
                 FROM flows WHERE 1=1{conds} ORDER BY ts DESC LIMIT ? OFFSET ?"
            );
            vals.push(Box::new(limit as i64));
            vals.push(Box::new(offset as i64));
            let mut stmt = conn.prepare(&sql)?;
            let params_ref: Vec<&dyn rusqlite::types::ToSql> =
                vals.iter().map(|b| b.as_ref()).collect();
            let rows = stmt.query_map(params_ref.as_slice(), row_to_stored)?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            return Ok(out);
        }

        let mem = self
            .memory
            .lock()
            .map_err(|_| anyhow::anyhow!("flow memory lock poisoned"))?;
        let mut filtered: Vec<_> = mem.iter().filter(|f| q.matches(f)).cloned().collect();
        filtered.sort_by(|a, b| b.ts.cmp(&a.ts));
        Ok(filtered.into_iter().skip(offset).take(limit).collect())
    }

    /// How many stored flows match, ignoring limit and offset.
    pub fn count(&self, q: &FlowQuery) -> Result<u64> {
        if let Some(db) = &self.db {
            let conn = db
                .lock()
                .map_err(|_| anyhow::anyhow!("flow db lock poisoned"))?;
            let (conds, vals) = q.where_sql();
            let params_ref: Vec<&dyn rusqlite::types::ToSql> =
                vals.iter().map(|b| b.as_ref()).collect();
            let n: i64 = conn.query_row(
                &format!("SELECT COUNT(*) FROM flows WHERE 1=1{conds}"),
                params_ref.as_slice(),
                |r| r.get(0),
            )?;
            return Ok(n as u64);
        }
        let mem = self
            .memory
            .lock()
            .map_err(|_| anyhow::anyhow!("flow memory lock poisoned"))?;
        Ok(mem.iter().filter(|f| q.matches(f)).count() as u64)
    }

    /// Flow counts per `bucket_secs` window, oldest first. Only windows that
    /// contain flows are returned; the caller fills the gaps. Ignores limit and
    /// offset.
    pub fn timeline(&self, q: &FlowQuery, bucket_secs: i64) -> Result<Vec<TimelineBucket>> {
        let bucket_secs = bucket_secs.max(1);
        if let Some(db) = &self.db {
            let conn = db
                .lock()
                .map_err(|_| anyhow::anyhow!("flow db lock poisoned"))?;
            let (conds, mut vals) = q.where_sql();
            // strftime('%s') is unix seconds; integer division floors to the bucket.
            let sql = format!(
                "SELECT CAST(strftime('%s', ts) AS INTEGER) / ? * ? AS b,
                        SUM(verdict = 'FORWARDED'), SUM(verdict = 'DROPPED'), COUNT(*)
                 FROM flows WHERE strftime('%s', ts) IS NOT NULL{conds}
                 GROUP BY b ORDER BY b"
            );
            let mut all: Vec<Box<dyn rusqlite::types::ToSql>> =
                vec![Box::new(bucket_secs), Box::new(bucket_secs)];
            all.append(&mut vals);
            let mut stmt = conn.prepare(&sql)?;
            let params_ref: Vec<&dyn rusqlite::types::ToSql> =
                all.iter().map(|b| b.as_ref()).collect();
            let rows = stmt.query_map(params_ref.as_slice(), |r| {
                let fwd: i64 = r.get(1)?;
                let drp: i64 = r.get(2)?;
                let all: i64 = r.get(3)?;
                Ok(TimelineBucket {
                    start: r.get(0)?,
                    forwarded: fwd as u64,
                    dropped: drp as u64,
                    other: (all - fwd - drp).max(0) as u64,
                })
            })?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            return Ok(out);
        }

        let mem = self
            .memory
            .lock()
            .map_err(|_| anyhow::anyhow!("flow memory lock poisoned"))?;
        let mut buckets: std::collections::BTreeMap<i64, TimelineBucket> = Default::default();
        for f in mem.iter().filter(|f| q.matches(f)) {
            let Ok(t) = DateTime::parse_from_rfc3339(&f.ts) else {
                continue;
            };
            let start = t.timestamp().div_euclid(bucket_secs) * bucket_secs;
            let b = buckets.entry(start).or_insert(TimelineBucket {
                start,
                forwarded: 0,
                dropped: 0,
                other: 0,
            });
            match f.verdict.as_str() {
                "FORWARDED" => b.forwarded += 1,
                "DROPPED" => b.dropped += 1,
                _ => b.other += 1,
            }
        }
        Ok(buckets.into_values().collect())
    }

    /// What the store holds. With `scope`, only flows with a side in those
    /// namespaces are counted, so a limited caller learns nothing about the rest.
    pub fn coverage(&self, scope: Option<&[String]>) -> Result<Coverage> {
        let q = FlowQuery {
            scope: scope.map(|s| s.to_vec()),
            ..Default::default()
        };
        if let Some(db) = &self.db {
            let conn = db
                .lock()
                .map_err(|_| anyhow::anyhow!("flow db lock poisoned"))?;
            let (conds, vals) = q.where_sql();
            let params_ref: Vec<&dyn rusqlite::types::ToSql> =
                vals.iter().map(|b| b.as_ref()).collect();
            let (oldest, newest, total): (Option<String>, Option<String>, i64) = conn.query_row(
                &format!("SELECT MIN(ts), MAX(ts), COUNT(*) FROM flows WHERE 1=1{conds}"),
                params_ref.as_slice(),
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            return Ok(Coverage {
                oldest,
                newest,
                total: total as u64,
                retention_days: self.retention_days,
                durable: true,
            });
        }
        let mem = self
            .memory
            .lock()
            .map_err(|_| anyhow::anyhow!("flow memory lock poisoned"))?;
        let visible: Vec<&StoredFlow> = mem.iter().filter(|f| q.matches(f)).collect();
        Ok(Coverage {
            oldest: visible.iter().map(|f| f.ts.clone()).min(),
            newest: visible.iter().map(|f| f.ts.clone()).max(),
            total: visible.len() as u64,
            retention_days: self.retention_days,
            durable: false,
        })
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<StoredFlow>> {
        if let Some(db) = &self.db {
            let conn = db
                .lock()
                .map_err(|_| anyhow::anyhow!("flow db lock poisoned"))?;
            let row = conn
                .query_row(
                    "SELECT id, ts, cluster, verdict, drop_reason, protocol, port,
                            src_namespace, src_pod, src_ip, src_identity,
                            dst_namespace, dst_pod, dst_ip, dst_identity, source
                     FROM flows WHERE id = ?1 ORDER BY ts DESC LIMIT 1",
                    params![id],
                    row_to_stored,
                )
                .optional()?;
            return Ok(row);
        }
        let mem = self
            .memory
            .lock()
            .map_err(|_| anyhow::anyhow!("flow memory lock poisoned"))?;
        Ok(mem.iter().rev().find(|f| f.id == id).cloned())
    }
}

fn row_to_stored(r: &rusqlite::Row<'_>) -> rusqlite::Result<StoredFlow> {
    Ok(StoredFlow {
        id: r.get(0)?,
        ts: r.get(1)?,
        cluster: r.get(2)?,
        verdict: r.get(3)?,
        drop_reason: r.get(4)?,
        protocol: r.get(5)?,
        port: r.get::<_, i64>(6)? as u16,
        src_namespace: r.get(7)?,
        src_pod: r.get(8)?,
        src_ip: r.get(9)?,
        src_identity: r.get(10)?,
        dst_namespace: r.get(11)?,
        dst_pod: r.get(12)?,
        dst_ip: r.get(13)?,
        dst_identity: r.get(14)?,
        source: FlowSource::parse(&r.get::<_, String>(15)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::flow::FlowEndpoint;

    fn sample(id: &str) -> StoredFlow {
        StoredFlow::from_flow(
            &Flow {
                id: id.into(),
                timestamp: Utc::now().to_rfc3339(),
                source: FlowEndpoint {
                    namespace: "shop".into(),
                    pod: "checkout-1".into(),
                    ip: "10.0.0.1".into(),
                },
                destination: FlowEndpoint {
                    namespace: "shop".into(),
                    pod: "payments-1".into(),
                    ip: "10.0.0.2".into(),
                },
                verdict: "DROPPED".into(),
                protocol: "TCP".into(),
                port: 443,
                http_method: None,
                http_url: None,
                http_code: None,
                cluster: Some("local".into()),
            },
            FlowSource::HubbleCli,
            "Policy denied",
        )
    }

    #[test]
    fn memory_insert_and_query() {
        let store = FlowStore::memory_only();
        store.insert_batch(&[sample("f1")]).unwrap();
        let rows = store
            .query(&FlowQuery {
                src_namespace: Some("shop".into()),
                port: Some(443),
                limit: 10,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].verdict, "DROPPED");
    }

    #[test]
    fn sqlite_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "paqtra-flows-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = FlowStore::open(&dir, 7).unwrap();
        store.insert_batch(&[sample("f2")]).unwrap();
        let rows = store
            .query(&FlowQuery {
                dst_pod: Some("payments".into()),
                limit: 10,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(store.stats().total, 1);
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── history queries ─────────────────────────────────────────

    fn row(
        id: &str,
        ts: &str,
        src: (&str, &str),
        dst: (&str, &str),
        port: u16,
        verdict: &str,
    ) -> StoredFlow {
        StoredFlow {
            id: id.into(),
            ts: normalize_ts(ts).expect("test timestamp"),
            cluster: "local".into(),
            verdict: verdict.into(),
            drop_reason: String::new(),
            protocol: "TCP".into(),
            port,
            src_namespace: src.0.into(),
            src_pod: src.1.into(),
            src_ip: String::new(),
            src_identity: 0,
            dst_namespace: dst.0.into(),
            dst_pod: dst.1.into(),
            dst_ip: String::new(),
            dst_identity: 0,
            source: FlowSource::HubbleCli,
        }
    }

    /// The same data in a SQLite store and an in-memory store, so each check
    /// proves both implementations agree.
    fn stores(rows: &[StoredFlow]) -> Vec<(&'static str, FlowStore, Option<std::path::PathBuf>)> {
        let dir = std::env::temp_dir().join(format!(
            "paqtra-flowq-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let sqlite = FlowStore::open(&dir, 36500).unwrap();
        let memory = FlowStore::memory_only();
        sqlite.insert_batch(rows).unwrap();
        memory.insert_batch(rows).unwrap();
        vec![("sqlite", sqlite, Some(dir)), ("memory", memory, None)]
    }

    fn cleanup(all: Vec<(&'static str, FlowStore, Option<std::path::PathBuf>)>) {
        for (_, store, dir) in all {
            drop(store);
            if let Some(d) = dir {
                let _ = std::fs::remove_dir_all(d);
            }
        }
    }

    fn ids(store: &FlowStore, q: &FlowQuery) -> Vec<String> {
        let mut q = q.clone();
        q.limit = 1000;
        store.query(&q).unwrap().into_iter().map(|f| f.id).collect()
    }

    fn dataset() -> Vec<StoredFlow> {
        let t = |m: i64| (Utc::now() - ChronoDuration::minutes(m)).to_rfc3339();
        vec![
            row(
                "a",
                &t(50),
                ("shop", "web-1"),
                ("shop", "db-1"),
                5432,
                "FORWARDED",
            ),
            row(
                "b",
                &t(40),
                ("shop", "web-1"),
                ("pay", "gw-1"),
                443,
                "DROPPED",
            ),
            row(
                "c",
                &t(30),
                ("pay", "gw-1"),
                ("shop", "web-2"),
                8080,
                "FORWARDED",
            ),
            row(
                "d",
                &t(20),
                ("ops", "cron-1"),
                ("ops", "cron-2"),
                80,
                "FORWARDED",
            ),
            row("e", &t(10), ("", ""), ("pay", "gw-1"), 443, "DROPPED"),
        ]
    }

    #[test]
    fn timestamps_are_normalized_to_one_comparable_form() {
        for input in [
            "2026-09-24T04:34:47Z",
            "2026-09-24T04:34:47.290678123Z",
            "2026-09-24T10:04:47.29+05:30",
            "2026-09-24T04:34:47.290678+00:00",
        ] {
            let n = normalize_ts(input).unwrap();
            assert_eq!(
                n.len(),
                "2026-09-24T04:34:47.290678Z".len(),
                "{input} -> {n}"
            );
            assert!(n.ends_with('Z'));
        }
        assert_eq!(
            normalize_ts("2026-09-24T10:04:47.29+05:30").unwrap(),
            "2026-09-24T04:34:47.290000Z"
        );
        assert!(
            normalize_ts("").is_none()
                && normalize_ts("yesterday").is_none()
                && normalize_ts("2026-09-24").is_none()
        );
        // Text order is time order, even for inputs written with different offsets.
        let earlier = normalize_ts("2026-09-24T09:59:59+05:30").unwrap(); // 04:29:59Z
        let later = normalize_ts("2026-09-24T04:30:00Z").unwrap();
        assert!(earlier < later);
        // A flow with no usable time is stamped with the current time in the same form.
        let f = StoredFlow::from_flow(
            &sample("x").into_flow_for_test(""),
            FlowSource::HubbleCli,
            "",
        );
        assert!(normalize_ts(&f.ts).is_some());
    }

    trait IntoFlowForTest {
        fn into_flow_for_test(self, ts: &str) -> Flow;
    }
    impl IntoFlowForTest for StoredFlow {
        fn into_flow_for_test(self, ts: &str) -> Flow {
            Flow {
                id: self.id,
                timestamp: ts.into(),
                source: crate::models::flow::FlowEndpoint {
                    namespace: self.src_namespace,
                    pod: self.src_pod,
                    ip: self.src_ip,
                },
                destination: crate::models::flow::FlowEndpoint {
                    namespace: self.dst_namespace,
                    pod: self.dst_pod,
                    ip: self.dst_ip,
                },
                verdict: self.verdict,
                protocol: self.protocol,
                port: self.port,
                http_method: None,
                http_url: None,
                http_code: None,
                cluster: None,
            }
        }
    }

    #[test]
    fn time_range_is_inclusive_below_and_exclusive_above() {
        let all = stores(&dataset());
        for (name, store, _) in &all {
            let rows = store
                .query(&FlowQuery {
                    limit: 100,
                    ..Default::default()
                })
                .unwrap();
            let by = |id: &str| rows.iter().find(|r| r.id == id).unwrap().ts.clone();
            let q = |since: Option<String>, until: Option<String>| FlowQuery {
                since_rfc3339: since,
                until_rfc3339: until,
                ..Default::default()
            };
            assert_eq!(
                ids(store, &q(Some(by("b")), Some(by("d")))),
                vec!["c", "b"],
                "{name}: [b, d)"
            );
            assert_eq!(
                ids(store, &q(Some(by("e")), None)),
                vec!["e"],
                "{name}: since is inclusive"
            );
            assert!(
                ids(store, &q(None, Some(by("a")))).is_empty(),
                "{name}: until is exclusive"
            );
            assert_eq!(
                ids(store, &q(None, None)),
                vec!["e", "d", "c", "b", "a"],
                "{name}: newest first"
            );
        }
        cleanup(all);
    }

    #[test]
    fn namespace_and_pod_match_either_side() {
        let all = stores(&dataset());
        for (name, store, _) in &all {
            let ns = |n: &str| FlowQuery {
                namespace: Some(n.into()),
                ..Default::default()
            };
            assert_eq!(ids(store, &ns("pay")), vec!["e", "c", "b"], "{name}");
            assert_eq!(ids(store, &ns("shop")), vec!["c", "b", "a"], "{name}");
            let pod = |p: &str| FlowQuery {
                pod: Some(p.into()),
                ..Default::default()
            };
            assert_eq!(ids(store, &pod("gw")), vec!["e", "c", "b"], "{name}");
            assert_eq!(ids(store, &pod("web-1")), vec!["b", "a"], "{name}");
            // Filters combine with AND.
            let both = FlowQuery {
                namespace: Some("pay".into()),
                verdict: Some("DROPPED".into()),
                port: Some(443),
                ..Default::default()
            };
            assert_eq!(ids(store, &both), vec!["e", "b"], "{name}");
        }
        cleanup(all);
    }

    #[test]
    fn scope_matches_either_side_and_keeps_count_and_paging_consistent() {
        let all = stores(&dataset());
        for (name, store, _) in &all {
            let scoped = |limit: usize, offset: usize| FlowQuery {
                scope: Some(vec!["shop".into()]),
                limit,
                offset,
                ..Default::default()
            };
            assert_eq!(
                ids(store, &scoped(1000, 0)),
                vec!["c", "b", "a"],
                "{name}: to or from shop"
            );
            assert_eq!(
                store.count(&scoped(1, 0)).unwrap(),
                3,
                "{name}: count ignores paging"
            );
            assert_eq!(store.query(&scoped(2, 0)).unwrap().len(), 2, "{name}");
            assert_eq!(
                store
                    .query(&scoped(2, 2))
                    .unwrap()
                    .iter()
                    .map(|f| f.id.as_str())
                    .collect::<Vec<_>>(),
                vec!["a"],
                "{name}: second page"
            );
            let two = FlowQuery {
                scope: Some(vec!["ops".into(), "pay".into()]),
                ..Default::default()
            };
            assert_eq!(ids(store, &two), vec!["e", "d", "c", "b"], "{name}");
            // A scope combined with a filter never widens it.
            let narrowed = FlowQuery {
                scope: Some(vec!["ops".into()]),
                namespace: Some("shop".into()),
                ..Default::default()
            };
            assert!(ids(store, &narrowed).is_empty(), "{name}");
            // An empty scope list matches nothing, not everything.
            let empty = FlowQuery {
                scope: Some(vec![]),
                ..Default::default()
            };
            assert!(
                ids(store, &empty).is_empty() && store.count(&empty).unwrap() == 0,
                "{name}"
            );
        }
        cleanup(all);
    }

    #[test]
    fn a_flow_with_no_namespace_on_either_side_is_never_in_scope() {
        let rows = vec![row(
            "w",
            &Utc::now().to_rfc3339(),
            ("", ""),
            ("", ""),
            53,
            "FORWARDED",
        )];
        let all = stores(&rows);
        for (name, store, _) in &all {
            assert!(
                ids(
                    store,
                    &FlowQuery {
                        scope: Some(vec!["shop".into()]),
                        ..Default::default()
                    }
                )
                .is_empty(),
                "{name}"
            );
            assert_eq!(
                ids(store, &FlowQuery::default()),
                vec!["w"],
                "{name}: visible without a scope"
            );
        }
        cleanup(all);
    }

    #[test]
    fn like_input_matches_literally() {
        let now = Utc::now().to_rfc3339();
        let rows = vec![
            row("p1", &now, ("n", "100%-done"), ("n", "x"), 1, "FORWARDED"),
            row("p2", &now, ("n", "a_b"), ("n", "x"), 1, "FORWARDED"),
            row("p3", &now, ("n", "axb"), ("n", "x"), 1, "FORWARDED"),
            row("p4", &now, ("n", "back\\slash"), ("n", "x"), 1, "FORWARDED"),
        ];
        let all = stores(&rows);
        for (name, store, _) in &all {
            let pod = |p: &str| {
                ids(
                    store,
                    &FlowQuery {
                        pod: Some(p.into()),
                        ..Default::default()
                    },
                )
            };
            assert_eq!(pod("%"), vec!["p1"], "{name}: % is not a wildcard");
            assert_eq!(pod("a_b"), vec!["p2"], "{name}: _ is not a wildcard");
            assert_eq!(pod("\\"), vec!["p4"], "{name}: backslash is literal");
            assert_eq!(
                ids(
                    store,
                    &FlowQuery {
                        src_pod: Some("_".into()),
                        ..Default::default()
                    }
                ),
                vec!["p2"],
                "{name}: src_pod escapes too"
            );
        }
        cleanup(all);
    }

    #[test]
    fn timeline_buckets_and_counts_verdicts() {
        let base = "2026-06-01T10:00:00Z";
        let at = |sec: i64| {
            (DateTime::parse_from_rfc3339(base).unwrap() + ChronoDuration::seconds(sec))
                .to_rfc3339()
        };
        let rows = vec![
            row("t1", &at(5), ("a", "p"), ("a", "q"), 1, "FORWARDED"),
            row("t2", &at(50), ("a", "p"), ("a", "q"), 1, "DROPPED"),
            row("t3", &at(59), ("a", "p"), ("a", "q"), 1, "DROPPED"),
            row("t4", &at(60), ("a", "p"), ("a", "q"), 1, "AUDIT"),
            row("t5", &at(200), ("b", "p"), ("b", "q"), 1, "FORWARDED"),
        ];
        let start = DateTime::parse_from_rfc3339(base).unwrap().timestamp();
        let all = stores(&rows);
        for (name, store, _) in &all {
            let got = store.timeline(&FlowQuery::default(), 60).unwrap();
            assert_eq!(
                got,
                vec![
                    TimelineBucket {
                        start,
                        forwarded: 1,
                        dropped: 2,
                        other: 0
                    },
                    TimelineBucket {
                        start: start + 60,
                        forwarded: 0,
                        dropped: 0,
                        other: 1
                    },
                    TimelineBucket {
                        start: start + 180,
                        forwarded: 1,
                        dropped: 0,
                        other: 0
                    },
                ],
                "{name}"
            );
            let big = store.timeline(&FlowQuery::default(), 3600).unwrap();
            assert_eq!(big.len(), 1, "{name}");
            assert_eq!(
                (big[0].forwarded, big[0].dropped, big[0].other),
                (2, 2, 1),
                "{name}"
            );
            let scoped = store
                .timeline(
                    &FlowQuery {
                        scope: Some(vec!["b".into()]),
                        ..Default::default()
                    },
                    60,
                )
                .unwrap();
            assert_eq!(scoped.len(), 1, "{name}: filters apply to the timeline");
            assert_eq!(
                store
                    .timeline(
                        &FlowQuery {
                            namespace: Some("nowhere".into()),
                            ..Default::default()
                        },
                        60
                    )
                    .unwrap(),
                vec![],
                "{name}"
            );
        }
        cleanup(all);
    }

    #[test]
    fn timeline_reads_hubble_nanosecond_timestamps() {
        // Real Hubble timestamps have nine fractional digits. They are normalized
        // on the way in, but rows stored by older versions may still carry them.
        let dir = std::env::temp_dir().join(format!(
            "paqtra-flowq-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let store = FlowStore::open(&dir, 36500).unwrap();
        {
            let db = store.db.as_ref().unwrap().lock().unwrap();
            db.execute(
                "INSERT INTO flows (id, ts, verdict, protocol, port) VALUES ('old', '2026-06-01T10:00:30.123456789Z', 'DROPPED', 'TCP', 1)",
                [],
            )
            .unwrap();
        }
        let got = store.timeline(&FlowQuery::default(), 60).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(
            (got[0].dropped, got[0].start),
            (
                1,
                DateTime::parse_from_rfc3339("2026-06-01T10:00:00Z")
                    .unwrap()
                    .timestamp()
            )
        );
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn coverage_reports_the_span_and_durability() {
        let all = stores(&dataset());
        for (name, store, _) in &all {
            let c = store.coverage(None).unwrap();
            assert_eq!(c.total, 5, "{name}");
            assert!(
                c.oldest.as_ref().unwrap() < c.newest.as_ref().unwrap(),
                "{name}"
            );
            assert_eq!(c.durable, *name == "sqlite", "{name}");
        }
        // A scope shows only what the caller may see: no count, oldest or newest
        // from namespaces outside it.
        for (name, store, _) in &all {
            let mine = store.coverage(Some(&["ops".to_string()])).unwrap();
            assert_eq!(mine.total, 1, "{name}: only the ops flow");
            assert_eq!(mine.oldest, mine.newest, "{name}");
            let none = store.coverage(Some(&["nowhere".to_string()])).unwrap();
            assert_eq!((none.total, none.oldest), (0, None), "{name}");
            let empty_scope = store.coverage(Some(&[])).unwrap();
            assert_eq!(empty_scope.total, 0, "{name}: an empty scope shows nothing");
        }
        let empty = FlowStore::memory_only().coverage(None).unwrap();
        assert_eq!((empty.total, empty.oldest, empty.newest), (0, None, None));
        cleanup(all);
    }

    #[test]
    fn retention_drops_old_flows_and_repeat_ingest_does_not_duplicate() {
        let dir = std::env::temp_dir().join(format!(
            "paqtra-flowq-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let store = FlowStore::open(&dir, 7).unwrap();
        let old = (Utc::now() - ChronoDuration::days(10)).to_rfc3339();
        let recent = (Utc::now() - ChronoDuration::days(1)).to_rfc3339();
        let rows = vec![
            row("old", &old, ("a", "p"), ("a", "q"), 1, "FORWARDED"),
            row("new", &recent, ("a", "p"), ("a", "q"), 1, "FORWARDED"),
        ];
        store.insert_batch(&rows).unwrap();
        assert_eq!(
            ids(&store, &FlowQuery::default()),
            vec!["new"],
            "the 10-day-old flow is past retention"
        );
        // Hubble is polled every 30 s and returns overlapping windows: the same
        // flow arriving again must not become a second row.
        store.insert_batch(&rows).unwrap();
        store.insert_batch(&rows[1..]).unwrap();
        assert_eq!(store.count(&FlowQuery::default()).unwrap(), 1);
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }
}
