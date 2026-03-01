// Redis cache service
//
// Wraps Redis connection manager for caching flow data, metrics, etc.
// Currently a stub - implement caching logic as needed.

use redis::aio::ConnectionManager;

pub struct CacheService {
    conn: ConnectionManager,
}

impl CacheService {
    pub fn new(conn: ConnectionManager) -> Self {
        Self { conn }
    }

    /// Check if Redis is reachable
    pub async fn is_healthy(&self) -> bool {
        redis::cmd("PING")
            .query_async::<String>(&mut self.conn.clone())
            .await
            .is_ok()
    }
}
