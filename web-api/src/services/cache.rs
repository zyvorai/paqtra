// In-memory cache service (replaces Redis) with optional SQLite durability.
//
// Thread-safe TTL map via DashMap. Same API as the former Redis wrapper so
// handlers stay unchanged. Entries written with `set` are ephemeral (TTL,
// memory only). Entries written with `set_persistent` are written through to
// SQLite when a database is attached, and reloaded on startup, so they survive
// restarts. `set_durable` does the same with a retention TTL: the expiry is
// stored too, and expired rows are dropped on load. Without a database, both are
// memory-only as before.

use anyhow::{Context, Result};
use dashmap::DashMap;
use rusqlite::{params, Connection};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn now_epoch_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

struct Entry {
    value: String,
    expires_at: Option<Instant>,
}

pub struct CacheService {
    store: Arc<DashMap<String, Entry>>,
    db: Option<Arc<Mutex<Connection>>>,
}

impl CacheService {
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
            db: None,
        }
    }

    /// Open (or create) a SQLite database at `path` and load every persisted
    /// entry into memory. Parent directories are created if missing.
    pub fn with_persistence(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create data dir {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database {}", path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kv (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                expires_at INTEGER,
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
            )",
        )?;

        let now_epoch = now_epoch_secs();
        conn.execute(
            "DELETE FROM kv WHERE expires_at IS NOT NULL AND expires_at <= ?1",
            params![now_epoch],
        )?;

        let store = DashMap::new();
        {
            let mut stmt = conn.prepare("SELECT key, value, expires_at FROM kv")?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<i64>>(2)?,
                ))
            })?;
            for row in rows {
                let (key, value, exp) = row?;
                let expires_at = exp
                    .map(|e| Instant::now() + Duration::from_secs((e - now_epoch).max(0) as u64));
                store.insert(key, Entry { value, expires_at });
            }
        }
        tracing::info!(
            "CacheService loaded {} persisted entries from {}",
            store.len(),
            path.display()
        );
        Ok(Self {
            store: Arc::new(store),
            db: Some(Arc::new(Mutex::new(conn))),
        })
    }

    fn db_write(&self, key: &str, value: &str, expires_at: Option<i64>) -> Result<()> {
        if let Some(db) = &self.db {
            let conn = db
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
            conn.execute(
                "INSERT INTO kv (key, value, expires_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value,
                     expires_at = excluded.expires_at,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
                params![key, value, expires_at],
            )
            .with_context(|| format!("Failed to persist key '{key}'"))?;
        }
        Ok(())
    }

    fn db_delete(&self, key: &str) -> Result<()> {
        if let Some(db) = &self.db {
            let conn = db
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
            conn.execute("DELETE FROM kv WHERE key = ?1", params![key])
                .with_context(|| format!("Failed to delete persisted key '{key}'"))?;
        }
        Ok(())
    }

    pub async fn is_healthy(&self) -> bool {
        true
    }

    fn purge_expired(&self, key: &str) {
        if let Some(entry) = self.store.get(key) {
            if let Some(exp) = entry.expires_at {
                if Instant::now() >= exp {
                    drop(entry);
                    self.store.remove(key);
                    let _ = self.db_delete(key);
                }
            }
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        self.purge_expired(key);
        match self.store.get(key) {
            Some(entry) => {
                let value: T = serde_json::from_str(&entry.value).with_context(|| {
                    format!("Failed to deserialize cached value for key '{key}'")
                })?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()> {
        let json_str =
            serde_json::to_string(value).context("Failed to serialize value for caching")?;
        self.store.insert(
            key.to_string(),
            Entry {
                value: json_str,
                expires_at: Some(Instant::now() + Duration::from_secs(ttl_secs)),
            },
        );
        Ok(())
    }

    pub async fn set_persistent<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let json_str =
            serde_json::to_string(value).context("Failed to serialize value for storage")?;
        // Write to disk first so a failed write is never reported as success.
        self.db_write(key, &json_str, None)?;
        self.store.insert(
            key.to_string(),
            Entry {
                value: json_str,
                expires_at: None,
            },
        );
        Ok(())
    }

    /// Like `set`, but also written to SQLite (when attached) with its expiry, so
    /// the entry survives restarts until `ttl_secs` elapse. Use for data with a
    /// retention window (e.g. change history).
    pub async fn set_durable<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_secs: u64,
    ) -> Result<()> {
        let json_str =
            serde_json::to_string(value).context("Failed to serialize value for storage")?;
        self.db_write(key, &json_str, Some(now_epoch_secs() + ttl_secs as i64))?;
        self.store.insert(
            key.to_string(),
            Entry {
                value: json_str,
                expires_at: Some(Instant::now() + Duration::from_secs(ttl_secs)),
            },
        );
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        self.db_delete(key)?;
        self.store.remove(key);
        Ok(())
    }

    pub async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let now = Instant::now();
        let keys: Vec<String> = self
            .store
            .iter()
            .filter_map(|kv| {
                if let Some(exp) = kv.expires_at {
                    if now >= exp {
                        return None;
                    }
                }
                if kv.key().starts_with(prefix) {
                    Some(kv.key().clone())
                } else {
                    None
                }
            })
            .collect();
        // Drop expired while listing
        for key in self.store.iter().filter_map(|kv| {
            kv.expires_at
                .filter(|e| now >= *e)
                .map(|_| kv.key().clone())
        }) {
            self.store.remove(&key);
            let _ = self.db_delete(&key);
        }
        Ok(keys)
    }

    pub async fn list_values(&self, prefix: &str) -> Result<Vec<serde_json::Value>> {
        let keys = self.list_keys(prefix).await?;
        let mut values = Vec::new();
        for key in keys {
            if let Ok(Some(val)) = self.get::<serde_json::Value>(&key).await {
                values.push(val);
            }
        }
        Ok(values)
    }
}

impl Default for CacheService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("paqtra-cache-{}-{}", name, uuid::Uuid::new_v4()));
        dir.join("paqtra.db")
    }

    #[tokio::test]
    async fn persistent_entries_survive_reopen() {
        let path = temp_db("reopen");
        {
            let c = CacheService::with_persistence(&path).unwrap();
            c.set_persistent("cv:audit_log:1", &serde_json::json!({"a": 1}))
                .await
                .unwrap();
            c.set_persistent("cv:audit_log:2", &serde_json::json!({"a": 2}))
                .await
                .unwrap();
        }
        let c = CacheService::with_persistence(&path).unwrap();
        let mut keys = c.list_keys("cv:audit_log:").await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["cv:audit_log:1", "cv:audit_log:2"]);
        let v: serde_json::Value = c.get("cv:audit_log:2").await.unwrap().unwrap();
        assert_eq!(v["a"], 2);
    }

    #[tokio::test]
    async fn ttl_entries_are_not_persisted() {
        let path = temp_db("ttl");
        {
            let c = CacheService::with_persistence(&path).unwrap();
            c.set("flow:x", &"tmp", 60).await.unwrap();
        }
        let c = CacheService::with_persistence(&path).unwrap();
        assert!(c.get::<String>("flow:x").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_and_overwrite_are_persisted() {
        let path = temp_db("delete");
        {
            let c = CacheService::with_persistence(&path).unwrap();
            c.set_persistent("k1", &1).await.unwrap();
            c.set_persistent("k1", &2).await.unwrap();
            c.set_persistent("k2", &3).await.unwrap();
            c.delete("k2").await.unwrap();
        }
        let c = CacheService::with_persistence(&path).unwrap();
        assert_eq!(c.get::<i32>("k1").await.unwrap(), Some(2));
        assert!(c.get::<i32>("k2").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn durable_entries_survive_until_expiry() {
        let path = temp_db("durable");
        {
            let c = CacheService::with_persistence(&path).unwrap();
            c.set_durable("cv:changes:live", &1, 3600).await.unwrap();
            c.set_durable("cv:changes:dead", &2, 0).await.unwrap();
        }
        let c = CacheService::with_persistence(&path).unwrap();
        assert_eq!(c.get::<i32>("cv:changes:live").await.unwrap(), Some(1));
        assert!(c.get::<i32>("cv:changes:dead").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn without_database_behaves_as_before() {
        let c = CacheService::new();
        c.set_persistent("k", &"v").await.unwrap();
        assert_eq!(c.get::<String>("k").await.unwrap().as_deref(), Some("v"));
    }
}
