// Redis cache service
//
// Wraps Redis connection manager for caching flow data, metrics, etc.
// Provides typed get/set with TTL support.

use anyhow::{Context, Result};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct CacheService {
    conn: ConnectionManager,
}

impl CacheService {
    pub fn new(conn: ConnectionManager) -> Self {
        Self { conn }
    }

    /// Check if Redis is reachable
    pub async fn is_healthy(&self) -> bool {
        let mut conn = self.conn.clone();
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .is_ok()
    }

    /// Get a value from cache, deserializing from JSON.
    /// Returns Ok(None) if the key does not exist.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.conn.clone();
        let raw: Option<String> = conn
            .get(key)
            .await
            .context("Redis GET failed")?;

        match raw {
            Some(json_str) => {
                let value: T = serde_json::from_str(&json_str)
                    .with_context(|| format!("Failed to deserialize cached value for key '{}'", key))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Set a value in cache, serializing to JSON with a TTL in seconds.
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()> {
        let mut conn = self.conn.clone();
        let json_str = serde_json::to_string(value)
            .context("Failed to serialize value for caching")?;

        conn.set_ex::<_, _, ()>(key, json_str, ttl_secs)
            .await
            .context("Redis SETEX failed")?;

        Ok(())
    }

    /// Set a persistent value (no TTL expiry).
    pub async fn set_persistent<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let mut conn = self.conn.clone();
        let json_str = serde_json::to_string(value)
            .context("Failed to serialize value for storage")?;

        conn.set::<_, _, ()>(key, json_str)
            .await
            .context("Redis SET failed")?;

        Ok(())
    }

    /// Delete a key from Redis.
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.conn.clone();
        conn.del::<_, ()>(key)
            .await
            .context("Redis DEL failed")?;
        Ok(())
    }

    /// List all keys matching a pattern prefix (using SCAN to avoid blocking Redis).
    pub async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let mut conn = self.conn.clone();
        let pattern = format!("{}*", prefix);
        let mut keys = Vec::new();
        let mut cursor: u64 = 0;
        loop {
            let (next_cursor, batch): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .context("Redis SCAN failed")?;
            keys.extend(batch);
            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }
        Ok(keys)
    }

    /// Get all values for a key prefix as JSON values.
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
