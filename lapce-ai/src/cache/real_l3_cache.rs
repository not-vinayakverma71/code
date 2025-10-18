/// Real L3 Cache Implementation with Redis
/// Production-ready distributed cache with connection pooling

use std::sync::Arc;
use std::time::Instant;
use redis::{Client, AsyncCommands};
// ConnectionManager is not available in current redis version
type ConnectionManager = redis::aio::MultiplexedConnection;
use anyhow::{Result, Context};
use tokio::sync::RwLock;

use super::types::{CacheKey, CacheValue};
use super::cache_metrics::CacheMetrics;

/// Real L3 distributed cache using Redis
pub struct RealL3Cache {
    client: Arc<Client>,
    conn_manager: Arc<RwLock<Option<ConnectionManager>>>,
    metrics: Arc<CacheMetrics>,
    ttl_seconds: u64,
    key_prefix: String,
}

impl RealL3Cache {
    /// Create new L3 cache with Redis backend
    pub async fn new(
        redis_url: &str,
        metrics: Arc<CacheMetrics>,
    ) -> Result<Self> {
        let client = Client::open(redis_url)
            .context("Failed to create Redis client")?;
        
        // Create multiplexed connection for pooling
        let conn_manager = match client.get_multiplexed_async_connection().await {
            Ok(mgr) => Some(mgr),
            Err(e) => {
                tracing::warn!("Failed to connect to Redis: {}", e);
                None
            }
        };
        
        Ok(Self {
            client: Arc::new(client),
            conn_manager: Arc::new(RwLock::new(conn_manager)),
            metrics,
            ttl_seconds: 3600, // 1 hour default TTL
            key_prefix: "lapce:cache:".to_string(),
        })
    }

    /// Get connection with auto-reconnect
    async fn get_connection(&self) -> Result<ConnectionManager> {
        let mut guard = self.conn_manager.write().await;
        
        match &*guard {
            Some(conn) => {
                // Test connection with PING
                let mut conn_clone = conn.clone();
                match redis::cmd("PING").query_async::<_, String>(&mut conn_clone).await {
                    Ok(_) => return Ok(conn.clone()),
                    Err(e) => {
                        tracing::warn!("Redis connection lost: {}", e);
                        *guard = None;
                    }
                }
            }
            None => {}
        }
        
        // Reconnect
        tracing::info!("Attempting Redis reconnection...");
        match self.client.get_multiplexed_async_connection().await {
            Ok(new_conn) => {
                *guard = Some(new_conn.clone());
                tracing::info!("Redis reconnected successfully");
                Ok(new_conn)
            }
            Err(e) => {
                tracing::error!("Redis reconnection failed: {}", e);
                Err(e.into())
            }
        }
    }

    /// Build full Redis key with prefix
    fn build_key(&self, key: &CacheKey) -> String {
        format!("{}{}", self.key_prefix, key.0)
    }

    /// Get value from L3 cache
    pub async fn get(&self, key: &CacheKey) -> Result<Option<CacheValue>> {
        let start = Instant::now();
        
        let mut conn = match self.get_connection().await {
            Ok(c) => c,
            Err(_) => {
                self.metrics.record_l3_miss();
                return Ok(None);
            }
        };
        
        let redis_key = self.build_key(key);
        
        // Get from Redis
        let data: Option<Vec<u8>> = conn.get(&redis_key).await?;
        
        let value = match data {
            Some(bytes) => {
                let value: CacheValue = bincode::deserialize(&bytes)?;
                self.metrics.record_l3_hit();
                Some(value)
            }
            None => {
                self.metrics.record_l3_miss();
                None
            }
        };
        
        // Record latency
        let latency = start.elapsed();
        self.metrics.record_l3_latency(latency);
        
        Ok(value)
    }

    /// Put value into L3 cache with TTL
    pub async fn put(&self, key: &CacheKey, value: CacheValue) -> Result<()> {
        let start = Instant::now();
        
        let mut conn = self.get_connection().await?;
        let redis_key = self.build_key(key);
        
        // Serialize value
        let data = bincode::serialize(&value)?;
        
        // Use default TTL
        let ttl = self.ttl_seconds;
        
        // Set with expiration
        conn.set_ex::<_, _, ()>(&redis_key, data, ttl).await?;
        
        // Record metrics
        let latency = start.elapsed();
        self.metrics.record_l3_write_latency(latency);
        
        Ok(())
    }

    /// Put value with pipelining for batch operations
    pub async fn put_batch(&self, entries: Vec<(CacheKey, CacheValue)>) -> Result<()> {
        let start = Instant::now();
        
        let mut conn = self.get_connection().await?;
        let mut pipe = redis::pipe();
        
        for (key, value) in entries {
            let redis_key = self.build_key(&key);
            let data = bincode::serialize(&value)?;
            let ttl = self.ttl_seconds;
            
            pipe.set_ex(&redis_key, data, ttl);
        }
        
        // Execute pipeline
        pipe.query_async::<_, ()>(&mut conn).await?;
        
        // Record metrics
        let latency = start.elapsed();
        self.metrics.record_l3_batch_latency(latency);
        
        Ok(())
    }

    /// Invalidate specific key
    pub async fn invalidate(&self, key: &CacheKey) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let redis_key = self.build_key(key);
        
        conn.del::<_, ()>(&redis_key).await?;
        
        Ok(())
    }

    /// Invalidate keys matching pattern
    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<usize> {
        let mut conn = self.get_connection().await?;
        let redis_pattern = format!("{}{}", self.key_prefix, pattern);
        
        // Use SCAN for production-safe iteration
        let mut cursor = 0u64;
        let mut total_deleted = 0usize;
        
        loop {
            let (new_cursor, keys): (u64, Vec<String>) = 
                redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(&redis_pattern)
                    .arg("COUNT")
                    .arg(100)
                    .query_async(&mut conn)
                    .await?;
            
            if !keys.is_empty() {
                let deleted: usize = conn.del(&keys).await?;
                total_deleted += deleted;
            }
            
            cursor = new_cursor;
            if cursor == 0 {
                break;
            }
        }
        
        Ok(total_deleted)
    }

    /// Get cache statistics from Redis INFO
    pub async fn stats(&self) -> Result<L3Stats> {
        let mut conn = self.get_connection().await?;
        
        // Get Redis INFO stats
        let info: String = redis::cmd("INFO")
            .arg("stats")
            .query_async(&mut conn)
            .await?;
        
        // Parse relevant metrics
        let mut keyspace_hits = 0u64;
        let mut keyspace_misses = 0u64;
        let mut total_connections = 0u64;
        
        for line in info.lines() {
            if let Some(value) = line.strip_prefix("keyspace_hits:") {
                keyspace_hits = value.parse().unwrap_or(0);
            } else if let Some(value) = line.strip_prefix("keyspace_misses:") {
                keyspace_misses = value.parse().unwrap_or(0);
            } else if let Some(value) = line.strip_prefix("total_connections_received:") {
                total_connections = value.parse().unwrap_or(0);
            }
        }
        
        let hit_rate = if keyspace_hits + keyspace_misses > 0 {
            keyspace_hits as f64 / (keyspace_hits + keyspace_misses) as f64
        } else {
            0.0
        };
        
        Ok(L3Stats {
            keyspace_hits,
            keyspace_misses,
            hit_rate,
            total_connections,
            avg_latency_ms: self.metrics.avg_l3_latency_ms(),
        })
    }

    /// Clear entire cache namespace
    pub async fn clear(&self) -> Result<()> {
        let pattern = format!("{}*", self.key_prefix);
        self.invalidate_pattern(&pattern).await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct L3Stats {
    pub keyspace_hits: u64,
    pub keyspace_misses: u64,
    pub hit_rate: f64,
    pub total_connections: u64,
    pub avg_latency_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use std::collections::HashMap;

    // These tests require Redis to be running
    // Run with: docker run -p 6379:6379 redis:alpine
    
    #[tokio::test]
    #[ignore] // Ignore by default as it requires Redis
    async fn test_l3_cache_basic() {
        let metrics = Arc::new(CacheMetrics::default());
        let cache = RealL3Cache::new(
            "redis://localhost:6379",
            metrics
        ).await.unwrap();
        
        let key = CacheKey("test_key".to_string());
        let value = CacheValue {
            data: vec![1, 2, 3, 4, 5],
            size: 5,
            created_at: SystemTime::now(),
            access_count: 0,
            last_accessed: SystemTime::now(),
            metadata: Some(HashMap::new()),
            ttl: Some(60), // seconds
        };
        
        cache.put(&key, value.clone()).await.unwrap();
        
        let retrieved = cache.get(&key).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data, value.data);
        
        // Clean up
        cache.invalidate(&key).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_l3_cache_batch() {
        let metrics = Arc::new(CacheMetrics::default());
        let cache = RealL3Cache::new(
            "redis://localhost:6379",
            metrics
        ).await.unwrap();
        
        // Prepare batch
        let mut batch = Vec::new();
        for i in 0..10 {
            let key = CacheKey(format!("batch_key_{}", i));
            let value = CacheValue {
                data: vec![i as u8; 100],
                size: 100,
            created_at: SystemTime::now(),
            access_count: 0,
            last_accessed: SystemTime::now(),
            metadata: Some(HashMap::new()),
            ttl: Some(60), // seconds
            };
            batch.push((key, value));
        }
        
        // Put batch
        cache.put_batch(batch).await.unwrap();
        
        // Verify all entries exist
        for i in 0..10 {
            let key = CacheKey(format!("batch_key_{}", i));
            let retrieved = cache.get(&key).await.unwrap();
            assert!(retrieved.is_some());
        }
        
        // Clean up
        cache.invalidate_pattern("batch_key_*").await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_l3_cache_auto_reconnect() {
        let metrics = Arc::new(CacheMetrics::default());
        
        // Try with invalid URL first
        let cache = RealL3Cache::new(
            "redis://localhost:9999", // Wrong port
            metrics.clone()
        ).await;
        
        // Should create cache but operations will fail gracefully
        assert!(cache.is_ok());
        let cache = cache.unwrap();
        
        let key = CacheKey("reconnect_test".to_string());
        let value = CacheValue {
            data: vec![1, 2, 3],
            size: 3,
            created_at: SystemTime::now(),
            access_count: 0,
            last_accessed: SystemTime::now(),
            metadata: Some(HashMap::new()),
            ttl: None,
        };
        
        // Put should fail but not panic
        let result = cache.put(&key, value).await;
        assert!(result.is_err());
        
        // Get should return None without panicking
        let result = cache.get(&key).await;
        assert!(result.unwrap().is_none());
    }
}
