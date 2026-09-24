//! Durable flow index (SQLite when PAQTRA_DATA_DIR is set, else in-memory).
//!
//! Stores Hubble flow metadata for investigation and policy preview.
//! Never stores payloads, argv, or secrets.

use anyhow::{Context, Result};
use chrono::{Duration as ChronoDuration, Utc};
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

impl StoredFlow {
    pub fn from_flow(flow: &Flow, source: FlowSource, drop_reason: &str) -> Self {
        Self {
            id: flow.id.clone(),
            ts: if flow.timestamp.is_empty() {
                Utc::now().to_rfc3339()
            } else {
                flow.timestamp.clone()
            },
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
    pub port: Option<u16>,
    pub verdict: Option<String>,
    pub since_rfc3339: Option<String>,
    pub limit: usize,
    pub offset: usize,
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
        let conn = Connection::open(&path)
            .with_context(|| format!("open {}", path.display()))?;
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
        let cutoff = (Utc::now() - ChronoDuration::days(self.retention_days)).to_rfc3339();
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
            let mut sql = String::from(
                "SELECT id, ts, cluster, verdict, drop_reason, protocol, port,
                        src_namespace, src_pod, src_ip, src_identity,
                        dst_namespace, dst_pod, dst_ip, dst_identity, source
                 FROM flows WHERE 1=1",
            );
            let mut vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            if let Some(v) = &q.src_namespace {
                sql.push_str(" AND src_namespace = ?");
                vals.push(Box::new(v.clone()));
            }
            if let Some(v) = &q.src_pod {
                sql.push_str(" AND src_pod LIKE ?");
                vals.push(Box::new(format!("%{v}%")));
            }
            if let Some(v) = &q.dst_namespace {
                sql.push_str(" AND dst_namespace = ?");
                vals.push(Box::new(v.clone()));
            }
            if let Some(v) = &q.dst_pod {
                sql.push_str(" AND dst_pod LIKE ?");
                vals.push(Box::new(format!("%{v}%")));
            }
            if let Some(p) = q.port {
                sql.push_str(" AND port = ?");
                vals.push(Box::new(p as i64));
            }
            if let Some(v) = &q.verdict {
                sql.push_str(" AND verdict = ?");
                vals.push(Box::new(v.clone()));
            }
            if let Some(v) = &q.since_rfc3339 {
                sql.push_str(" AND ts >= ?");
                vals.push(Box::new(v.clone()));
            }
            sql.push_str(" ORDER BY ts DESC LIMIT ? OFFSET ?");
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
        let mut filtered: Vec<_> = mem
            .iter()
            .filter(|f| {
                q.src_namespace
                    .as_ref()
                    .map(|n| &f.src_namespace == n)
                    .unwrap_or(true)
                    && q.src_pod
                        .as_ref()
                        .map(|p| f.src_pod.contains(p))
                        .unwrap_or(true)
                    && q.dst_namespace
                        .as_ref()
                        .map(|n| &f.dst_namespace == n)
                        .unwrap_or(true)
                    && q.dst_pod
                        .as_ref()
                        .map(|p| f.dst_pod.contains(p))
                        .unwrap_or(true)
                    && q.port.map(|p| f.port == p).unwrap_or(true)
                    && q.verdict
                        .as_ref()
                        .map(|v| &f.verdict == v)
                        .unwrap_or(true)
                    && q.since_rfc3339
                        .as_ref()
                        .map(|s| f.ts.as_str() >= s.as_str())
                        .unwrap_or(true)
            })
            .cloned()
            .collect();
        filtered.sort_by(|a, b| b.ts.cmp(&a.ts));
        Ok(filtered.into_iter().skip(offset).take(limit).collect())
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
}
