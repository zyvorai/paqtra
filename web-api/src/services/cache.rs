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
            .query_async::<_, String>(&mut conn)
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
}
