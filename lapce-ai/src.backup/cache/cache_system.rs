/// CacheSystem - EXACT implementation from docs lines 34-49
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

use super::{
    l1_cache::L1Cache,
    l2_cache::L2Cache,
    l3_cache::L3Cache,
    cache_coordinator::CacheCoordinator,
    cache_metrics::CacheMetrics,
    types::{CacheConfig, CacheKey, CacheValue},
};

pub struct CacheSystem {
    // Three-tier cache hierarchy (docs line 39-41)
    pub l1_cache: Arc<L1Cache>,
    pub l2_cache: Arc<L2Cache>,
    pub l3_cache: Option<Arc<RwLock<L3Cache>>>,
    
    // Cache coordinator
    pub coordinator: Arc<CacheCoordinator>,
    
    // Metrics
    pub metrics: Arc<CacheMetrics>,
}

impl CacheSystem {
    pub async fn new(config: CacheConfig) -> Result<Self> {
        let metrics = Arc::new(CacheMetrics::default());
        
        // Initialize L1 cache
        let l1_cache = Arc::new(L1Cache::new(config.l1_config.clone(), metrics.clone()));
        
        // Initialize L2 cache
        let l2_cache = Arc::new(L2Cache::new(config.l2_config.clone(), metrics.clone()).await?);
        
        // Initialize L3 cache if Redis URL provided
        let l3_cache = if let Some(redis_url) = &config.l3_redis_url {
            Some(Arc::new(RwLock::new(L3Cache::new(redis_url, metrics.clone()).await?)))
        } else {
            None
        };
        
        // Initialize coordinator
        let coordinator = Arc::new(CacheCoordinator::new(
            l1_cache.clone(),
            l2_cache.clone(),
            l3_cache.clone(),
            metrics.clone(),
        ));
        
        Ok(Self {
            l1_cache,
            l2_cache,
            l3_cache,
            coordinator,
            metrics,
        })
    }
    
    pub async fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        self.coordinator.get(key).await
    }
    
    pub async fn put(&self, key: CacheKey, value: CacheValue) {
        self.coordinator.put(key, value).await
    }
}
