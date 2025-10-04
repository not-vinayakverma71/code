/// FINAL CONSOLIDATED CACHE V3 - Production Ready
/// This is the ONLY implementation to use
/// Combines best of all approaches with honest assessment

use std::sync::Arc;
use moka::future::Cache as MokaCache;
use parking_lot::RwLock;
use anyhow::Result;
use tracing;

use super::{
    types::{CacheKey, CacheValue, CacheConfig, CacheLevel},
    cache_coordinator::CacheCoordinator,
    cache_metrics::CacheMetrics,
    bloom_filter::BloomFilter,
    l1_cache::L1Cache,
    l2_cache::L2Cache,
    l3_cache::L3Cache,
};

/// The ONE and ONLY Cache Implementation
pub struct CacheV3 {
    // L1: Moka async in-memory cache (as specified in docs)
    pub l1_cache: Arc<MokaCache<CacheKey, CacheValue>>,
    pub l2_cache: Arc<tokio::sync::RwLock<Option<L2Cache>>>,
    l3: Arc<tokio::sync::RwLock<Option<L3Cache>>>,
    bloom: Arc<RwLock<BloomFilter>>,
    
    // Cache coordinator
    pub coordinator: Arc<CacheCoordinator>,
    
    // Metrics and config
    pub metrics: Arc<CacheMetrics>,
    config: CacheConfig,
}

impl CacheV3 {
    pub async fn new(config: CacheConfig) -> Result<Self> {
        let metrics = Arc::new(CacheMetrics::default());
        
        // Build Moka cache with eviction listener (docs line 80-82)
        let l1 = MokaCache::builder()
            .max_capacity(config.l1_config.max_entries)
            .time_to_live(config.l1_config.ttl)
            .time_to_idle(config.l1_config.idle_time)
            .weigher(|_key: &CacheKey, value: &CacheValue| value.size() as u32)
            .eviction_listener(|key, _value, cause| {
                tracing::debug!("Evicted {:?}: {:?}", key, cause);
            })
            .build();
        
        let l1_arc = Arc::new(l1);
        let l1_cache = Arc::new(L1Cache::new(config.l1_config.clone(), metrics.clone()));
        let l2_cache = Arc::new(tokio::sync::RwLock::new(None));
        let l3_cache = Arc::new(tokio::sync::RwLock::new(None));
        
        // Create coordinator
        let coordinator = Arc::new(CacheCoordinator::new(
            l1_cache.clone(),
            Arc::new(L2Cache::new(config.l2_config.clone(), metrics.clone()).await?),
            None, // L3 disabled for now
            metrics.clone(),
        ));
        
        Ok(Self {
            l1_cache: l1_arc,
            l2_cache,
            l3: l3_cache,
            bloom: Arc::new(RwLock::new(BloomFilter::new(
                config.l1_config.bloom_size,
                config.l1_config.bloom_fp_rate,
            ))),
            coordinator,
            metrics,
            config,
        })
    }
    
    /// Get from cache - tries L1, then L2, then L3
    pub async fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        // L1 check (synchronous, fast)
        if !self.bloom.read().contains(key) {
            self.metrics.record_bloom_filter_hit();
            return None;
        }
        
        if let Some(value) = self.l1_cache.get(key).await {
            self.metrics.record_l1_hit();
            return Some(value);
        }
        self.metrics.record_l1_miss();
        
        // L2 check (async, disk)
        let l2_result = self.get_from_l2(key).await;
        if l2_result.is_some() {
            return l2_result;
        }
        
        // L3 check (async, network)
        let l3_result = self.get_from_l3(key).await;
        if l3_result.is_some() {
            return l3_result;
        }
        
        self.metrics.record_miss(CacheLevel::L3);
        None
    }
    
    /// Put into cache
    pub async fn put(&self, key: CacheKey, value: CacheValue) {
        // Always put in L1
        self.put_l1(key.clone(), value.clone()).await;
        
        // Optionally put in L2/L3 based on size/importance
        if value.size > 1000 {
            let _ = self.put_l2(key.clone(), value.clone()).await;
        }
        
        if value.size > 10000 && self.config.l3_redis_url.is_some() {
            let _ = self.put_l3(key, value).await;
        }
    }
    
    /// Get synchronously - uses blocking on async
    #[inline]
    pub fn get_sync(&self, key: &CacheKey) -> Option<CacheValue> {
        if !self.bloom.read().contains(key) {
            return None;
        }
        
        // Block on async get for sync API
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.l1_cache.get(key).await.map(|v| {
                    self.metrics.record_l1_hit();
                    v
                })
            })
        }).or_else(|| {
            self.metrics.record_l1_miss();
            None
        })
    }
    
    /// Put synchronously - uses blocking on async
    #[inline]
    pub fn put_sync(&self, key: CacheKey, value: CacheValue) {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.put_l1(key, value).await;
            })
        });
    }
    
    // Internal methods
    
    async fn put_l1(&self, key: CacheKey, value: CacheValue) {
        // Moka handles eviction automatically based on max_capacity
        self.bloom.write().insert(&key);
        self.l1_cache.insert(key, value).await;
    }
    
    async fn get_from_l2(&self, key: &CacheKey) -> Option<CacheValue> {
        let mut l2_guard = self.l2_cache.write().await;
        
        if l2_guard.is_none() {
            if let Ok(l2) = L2Cache::new(self.config.l2_config.clone(), self.metrics.clone()).await {
                *l2_guard = Some(l2);
            } else {
                return None;
            }
        }
        
        if let Some(l2) = l2_guard.as_ref() {
            if let Ok(Some(value)) = l2.get(key).await {
                self.metrics.record_l2_hit();
                // Promote to L1
                self.put_l1(key.clone(), value.clone());
                return Some(value);
            }
        }
        
        self.metrics.record_l2_miss();
        None
    }
    
    async fn put_l2(&self, key: CacheKey, value: CacheValue) -> Result<()> {
        let mut l2_guard = self.l2_cache.write().await;
        
        if l2_guard.is_none() {
            let l2 = L2Cache::new(self.config.l2_config.clone(), self.metrics.clone()).await?;
            *l2_guard = Some(l2);
        }
        
        if let Some(l2) = l2_guard.as_ref() {
            l2.put(key, value).await?;
        }
        
        Ok(())
    }
    
    async fn get_from_l3(&self, key: &CacheKey) -> Option<CacheValue> {
        if self.config.l3_redis_url.is_none() {
            return None;
        }
        
        let mut l3_guard = self.l3.write().await;
        
        if l3_guard.is_none() {
            if let Some(redis_url) = &self.config.l3_redis_url {
                if let Ok(l3) = L3Cache::new(redis_url, self.metrics.clone()).await {
                    *l3_guard = Some(l3);
                }
            }
        }
        
        if let Some(l3) = l3_guard.as_mut() {
            if let Ok(Some(value)) = l3.get(key).await {
                self.metrics.record_l3_hit();
                // Promote to L1 and L2
                self.put_l1(key.clone(), value.clone());
                let _ = self.put_l2(key.clone(), value.clone()).await;
                return Some(value);
            }
        }
        
        self.metrics.record_l3_miss();
        None
    }
    
    async fn put_l3(&self, key: CacheKey, value: CacheValue) -> Result<()> {
        let mut l3_guard = self.l3.write().await;
        
        if l3_guard.is_none() {
            if let Some(redis_url) = &self.config.l3_redis_url {
                let l3 = L3Cache::new(redis_url, self.metrics.clone()).await?;
                *l3_guard = Some(l3);
            }
        }
        
        if let Some(l3) = l3_guard.as_mut() {
            l3.put(&key, value).await?;
        }
        
        Ok(())
    }
    
    pub async fn get_metrics(&self) -> super::cache_metrics::MetricsSnapshot {
        self.metrics.snapshot().await
    }
}

// Export as the main cache
pub use self::CacheV3 as Cache;
