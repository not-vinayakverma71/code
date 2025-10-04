/// Real L1 Cache Implementation with Moka
/// Production-ready with TTL, LRU eviction, and metrics

use std::sync::Arc;
use std::time::{Duration, Instant};
use moka::future::Cache as MokaCache;
use anyhow::Result;
use async_trait::async_trait;

use super::types::{CacheKey, CacheValue, L1Config};
use super::cache_metrics::CacheMetrics;
use std::collections::HashMap;

/// Real L1 in-memory cache using Moka
pub struct RealL1Cache {
    cache: Arc<MokaCache<CacheKey, CacheValue>>,
    metrics: Arc<CacheMetrics>,
    config: L1Config,
}

impl RealL1Cache {
    /// Create new L1 cache with TTL and LRU eviction
    pub fn new(config: L1Config, metrics: Arc<CacheMetrics>) -> Self {
        let cache = MokaCache::builder()
            .max_capacity(config.max_entries)
            .time_to_live(config.ttl)
            .time_to_idle(config.idle_time)
            .weigher(|_key: &CacheKey, value: &CacheValue| -> u32 {
                (value.size / 1024) as u32 // Weight in KB
            })
            .eviction_listener(|key, value, cause| {
                tracing::debug!(
                    "L1 eviction: key={:?}, size={}, cause={:?}",
                    key, value.size, cause
                );
            })
            .build();

        Self {
            cache: Arc::new(cache),
            metrics,
            config,
        }
    }

    /// Get value from L1 cache
    pub async fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        let start = Instant::now();
        
        let result = self.cache.get(key).await;
        
        // Record metrics
        let latency = start.elapsed();
        if result.is_some() {
            self.metrics.record_l1_hit();
            self.metrics.record_l1_latency(latency);
        } else {
            self.metrics.record_l1_miss();
        }
        
        result
    }

    /// Put value into L1 cache
    pub async fn put(&self, key: CacheKey, value: CacheValue) {
        let start = Instant::now();
        
        // Insert with automatic TTL and eviction handling
        self.cache.insert(key, value).await;
        
        // Record metrics
        let latency = start.elapsed();
        self.metrics.record_l1_write_latency(latency);
    }

    /// Invalidate specific key
    pub async fn invalidate(&self, key: &CacheKey) {
        self.cache.remove(key).await;
    }

    /// Invalidate multiple keys matching pattern
    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<usize> {
        // Note: Moka doesn't expose iteration over keys
        // In production, we'd maintain a separate key index
        // For now, return 0 as we can't iterate
        tracing::warn!("Pattern invalidation not supported in Moka, use individual invalidation");
        Ok(0)
    }

    /// Get cache statistics
    pub fn stats(&self) -> L1Stats {
        let entry_count = self.cache.entry_count() as usize;
        let weighted_size = self.cache.weighted_size() as usize;
        
        L1Stats {
            entry_count,
            weighted_size_kb: weighted_size,
            hit_rate: self.metrics.hit_rate(),
            avg_latency_us: self.metrics.avg_l1_latency_us() as u64,
        }
    }

    /// Run maintenance tasks (called periodically)
    pub async fn run_maintenance(&self) {
        // Moka handles maintenance automatically in the background
        // This method is for compatibility and future extensions
        self.cache.run_pending_tasks().await;
    }
}

#[derive(Debug, Clone)]
pub struct L1Stats {
    pub entry_count: usize,
    pub weighted_size_kb: usize,
    pub hit_rate: f64,
    pub avg_latency_us: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, Duration};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_l1_cache_basic() {
        let config = L1Config {
            max_entries: 100,
            ttl: Duration::from_secs(60),
            idle_time: Duration::from_secs(30),
            bloom_size: 1000,
            bloom_fp_rate: 0.01,
        };
        
        let metrics = Arc::new(CacheMetrics::default());
        let cache = RealL1Cache::new(config, metrics);
        
        // Test put and get
        let key = CacheKey("test_key".to_string());
        let value = CacheValue {
            data: vec![1, 2, 3, 4, 5],
            size: 5,
            created_at: SystemTime::now(),
            access_count: 0,
            last_accessed: SystemTime::now(),
            metadata: Some(HashMap::new()),
            ttl: None,
        };
        
        cache.put(key.clone(), value.clone()).await;
        
        let retrieved = cache.get(&key).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data, value.data);
    }

    #[tokio::test]
    async fn test_l1_cache_eviction() {
        let config = L1Config {
            max_entries: 2, // Very small cache
            ttl: Duration::from_secs(60),
            idle_time: Duration::from_secs(30),
            bloom_size: 100,
            bloom_fp_rate: 0.01,
        };
        
        let metrics = Arc::new(CacheMetrics::default());
        let cache = RealL1Cache::new(config, metrics);
        
        // Fill cache beyond capacity
        for i in 0..5 {
            let key = CacheKey(format!("key_{}", i));
            let value = CacheValue {
                data: vec![i as u8; 100],
                size: 100,
            created_at: SystemTime::now(),
            access_count: 0,
            last_accessed: SystemTime::now(),
            metadata: Some(HashMap::new()),
            ttl: None,
            };
            cache.put(key, value).await;
        }
        
        // Allow eviction to happen
        tokio::time::sleep(Duration::from_millis(100)).await;
        cache.run_maintenance().await;
        
        // Check that cache respects max_entries
        let stats = cache.stats();
        assert!(stats.entry_count <= 2);
    }

    #[tokio::test]
    async fn test_l1_cache_ttl() {
        let config = L1Config {
            max_entries: 100,
            ttl: Duration::from_millis(100), // Very short TTL
            idle_time: Duration::from_secs(30),
            bloom_size: 1000,
            bloom_fp_rate: 0.01,
        };
        
        let metrics = Arc::new(CacheMetrics::default());
        let cache = RealL1Cache::new(config, metrics);
        
        let key = CacheKey("ttl_key".to_string());
        let value = CacheValue {
            data: vec![1, 2, 3],
            size: 3,
            created_at: SystemTime::now(),
            access_count: 0,
            last_accessed: SystemTime::now(),
            metadata: Some(HashMap::new()),
            ttl: None,
        };
        
        cache.put(key.clone(), value).await;
        
        // Should exist immediately
        assert!(cache.get(&key).await.is_some());
        
        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(150)).await;
        cache.run_maintenance().await;
        
        // Should be expired
        assert!(cache.get(&key).await.is_none());
    }
}
