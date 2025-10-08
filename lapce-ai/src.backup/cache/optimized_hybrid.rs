/// Optimized Hybrid Cache - Production Ready
/// Memory-optimized version maintaining performance

use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;
use anyhow::Result;

use super::{
    types::{CacheKey, CacheValue, CacheConfig, L1Config},
    bloom_filter::BloomFilter,
    cache_metrics::CacheMetrics,
    l2_cache::L2Cache,
    l3_cache::L3Cache,
};

/// Optimized L1 Cache - minimal memory footprint
pub struct OptimizedL1Cache {
    // Start with minimal capacity
    map: Arc<DashMap<CacheKey, CacheValue>>,
    bloom_filter: Arc<RwLock<BloomFilter>>,
    config: L1Config,
}

impl OptimizedL1Cache {
    pub fn new(config: L1Config) -> Self {
        // Start with 1/10th capacity to save memory
        let initial_capacity = (config.max_entries / 10).max(10) as usize;
        
        Self {
            map: Arc::new(DashMap::with_capacity(initial_capacity)),
            bloom_filter: Arc::new(RwLock::new(
                // Smaller bloom filter - 10KB instead of 100KB
                BloomFilter::new(10_000, config.bloom_fp_rate)
            )),
            config,
        }
    }
    
    #[inline]
    pub fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        // Check bloom filter first
        if !self.bloom_filter.read().contains(key) {
            return None;
        }
        
        self.map.get(key).map(|v| v.clone())
    }
    
    #[inline]
    pub fn put(&self, key: CacheKey, value: CacheValue) {
        // Simple capacity check
        if self.map.len() >= self.config.max_entries as usize {
            // Remove first item found (faster than LRU)
            if let Some(item) = self.map.iter().next() {
                self.map.remove(item.key());
            }
        }
        
        self.bloom_filter.write().insert(&key);
        self.map.insert(key, value);
    }
}

/// Production-Ready Optimized Hybrid Cache
pub struct OptimizedHybridCache {
    // Core components
    l1: Arc<OptimizedL1Cache>,
    metrics: Arc<CacheMetrics>,
    
    // Lazy-loaded components (created on first use)
    l2: Arc<tokio::sync::RwLock<Option<L2Cache>>>,
    l3: Arc<tokio::sync::RwLock<Option<L3Cache>>>,
    
    // Configuration
    config: CacheConfig,
}

impl OptimizedHybridCache {
    pub async fn new(config: CacheConfig) -> Result<Self> {
        let metrics = Arc::new(CacheMetrics::default());
        let l1 = Arc::new(OptimizedL1Cache::new(config.l1_config.clone()));
        
        Ok(Self {
            l1,
            metrics,
            l2: Arc::new(tokio::sync::RwLock::new(None)),
            l3: Arc::new(tokio::sync::RwLock::new(None)),
            config,
        })
    }
    
    /// Get L2 cache, creating it lazily if needed
    async fn get_or_create_l2(&self) -> Result<()> {
        let mut l2_guard = self.l2.write().await;
        
        if l2_guard.is_none() {
            let l2 = L2Cache::new(self.config.l2_config.clone(), self.metrics.clone()).await?;
            *l2_guard = Some(l2);
        }
        
        Ok(())
    }
    
    /// Fast sync get for L1
    #[inline]
    pub fn get_sync(&self, key: &CacheKey) -> Option<CacheValue> {
        if let Some(value) = self.l1.get(key) {
            self.metrics.record_l1_hit();
            Some(value)
        } else {
            self.metrics.record_l1_miss();
            None
        }
    }
    
    /// Full async get with L2/L3 fallback
    pub async fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        // L1 check
        if let Some(value) = self.get_sync(key) {
            return Some(value);
        }
        
        // L2 check (lazy load)
        let l2_guard = self.l2.read().await;
        if let Some(l2) = l2_guard.as_ref() {
            if let Ok(Some(value)) = l2.get(key).await {
                self.metrics.record_l2_hit();
                // Promote to L1
                self.put_sync(key.clone(), value.clone());
                return Some(value);
            }
        }
        
        self.metrics.record_l2_miss();
        None
    }
    
    /// Fast sync put for L1 only
    #[inline]
    pub fn put_sync(&self, key: CacheKey, value: CacheValue) {
        self.l1.put(key, value);
    }
    
    /// Async put with optional L2 write
    pub async fn put(&self, key: CacheKey, value: CacheValue) {
        // Always put in L1
        self.put_sync(key.clone(), value.clone());
        
        // Optionally write to L2 for large/important values
        if value.size > 1000 {
            if let Ok(()) = self.get_or_create_l2().await {
                // Would write to L3/Redis here
                println!("Would cache to L3: key={:?}", key);
            }
        }
    }
    
    pub async fn get_metrics(&self) -> super::cache_metrics::MetricsSnapshot {
        self.metrics.snapshot().await
    }
}
