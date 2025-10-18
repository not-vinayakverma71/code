/// L1Cache - EXACT implementation from docs lines 51-137
use std::sync::Arc;
use moka::future::Cache as MokaCache;
use parking_lot::RwLock;
use tracing;

use super::{
    bloom_filter::BloomFilter,
    cache_metrics::CacheMetrics,
    access_counter::AccessCounter,
    types::{CacheKey, CacheValue, L1Config},
};

pub struct L1Cache {
    pub cache: MokaCache<CacheKey, CacheValue>,
    pub bloom_filter: Arc<RwLock<BloomFilter>>,
    pub access_counter: Arc<AccessCounter>,
    pub metrics: Arc<CacheMetrics>,
    cache_threshold: f64,
}

impl L1Cache {
    pub fn new(config: L1Config, metrics: Arc<CacheMetrics>) -> Self {
        let cache = MokaCache::builder()
            .max_capacity(config.max_entries)
            .time_to_live(config.ttl)
            .time_to_idle(config.idle_time)
            .weigher(|_key: &CacheKey, value: &CacheValue| value.size() as u32)
            .eviction_listener(|key, value, cause| {
                tracing::debug!("Evicted {:?}: {:?}", key, cause);
            })
            .build();
            
        let bloom_filter = Arc::new(RwLock::new(BloomFilter::new(
            config.bloom_size,
            config.bloom_fp_rate,
        )));
        
        Self {
            cache,
            bloom_filter,
            access_counter: Arc::new(AccessCounter::new()),
            metrics,
            cache_threshold: 0.5, // Default threshold
        }
    }
    
    pub async fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        // Check bloom filter first
        if !self.bloom_filter.read().contains(key) {
            self.metrics.record_bloom_filter_hit();
            return None;
        }
        
        // Get from cache
        if let Some(value) = self.cache.get(key).await {
            self.access_counter.record(key);
            self.metrics.record_l1_hit();
            Some(value)
        } else {
            self.metrics.record_l1_miss();
            None
        }
    }
    
    pub async fn put(&self, key: CacheKey, value: CacheValue) {
        // Update bloom filter
        self.bloom_filter.write().insert(&key);
        
        // Check if we should cache based on access patterns
        if self.should_cache(&key, &value).await {
            self.cache.insert(key.clone(), value).await;
            self.access_counter.record(&key);
        }
    }
    
    async fn should_cache(&self, key: &CacheKey, value: &CacheValue) -> bool {
        // Adaptive caching based on access frequency and value size
        let frequency = self.access_counter.frequency(key);
        let size = value.size();
        
        // Always cache new items (frequency 0) or use frequency-based scoring
        if frequency == 0 {
            return true; // Cache new items on first access
        }
        
        // Use logarithmic decay for size penalty
        let size_factor = 1.0 / (1.0 + (size as f64).ln());
        let frequency_factor = (frequency as f64).sqrt();
        
        let score = frequency_factor * size_factor;
        score > self.cache_threshold()
    }
    
    fn cache_threshold(&self) -> f64 {
        self.cache_threshold
    }
}
