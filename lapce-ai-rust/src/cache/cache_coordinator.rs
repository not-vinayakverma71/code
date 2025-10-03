/// CacheCoordinator - EXACT implementation from docs lines 313-371
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::RwLock;

use super::{
    l1_cache::L1Cache,
    l2_cache::L2Cache,
    l3_cache::L3Cache,
    promotion_policy::PromotionPolicy,
    cache_metrics::CacheMetrics,
    types::{CacheKey, CacheValue, CacheLevel},
};

pub struct CacheCoordinator {
    pub l1: Arc<L1Cache>,
    pub l2: Arc<L2Cache>,
    pub l3: Option<Arc<RwLock<L3Cache>>>,
    pub promotion_policy: PromotionPolicy,
    pub metrics: Arc<CacheMetrics>,
}

impl CacheCoordinator {
    pub fn new(
        l1: Arc<L1Cache>,
        l2: Arc<L2Cache>,
        l3: Option<Arc<RwLock<L3Cache>>>,
        metrics: Arc<CacheMetrics>,
    ) -> Self {
        Self {
            l1,
            l2,
            l3,
            promotion_policy: PromotionPolicy::new(),
            metrics,
        }
    }
    
    pub async fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        // Try L1 first
        if let Some(value) = self.l1.get(key).await {
            self.l1.metrics.record_l1_hit();
            return Some(value);
        }
        self.l1.metrics.record_l1_miss();
        
        // Try L2
        if let Ok(Some(value)) = self.l2.get(key).await {
            self.l2.metrics.record_l2_hit();
            // Promote to L1 if hot
            if self.promotion_policy.should_promote_to_l1(key, &value) {
                self.l1.put(key.clone(), value.clone()).await;
            }
            return Some(value);
        }
        self.l2.metrics.record_l2_miss();
        
        // Try L3 if available
        if let Some(l3) = &self.l3 {
            let mut l3_guard = l3.write().await;
            if let Ok(Some(value)) = l3_guard.get(key).await {
                self.metrics.record_l3_hit();
                // Promote through levels
                if self.promotion_policy.should_promote_to_l2(key, &value) {
                    let _ = self.l2.put(key.clone(), value.clone()).await;
                }
                if self.promotion_policy.should_promote_to_l1(key, &value) {
                    self.l1.put(key.clone(), value.clone()).await;
                }
                return Some(value);
            }
        }    
        self.metrics.record_l3_miss();
        
        None
    }
    
    pub async fn put(&self, key: CacheKey, value: CacheValue) {
        let levels = self.promotion_policy.determine_levels(&key, &value);
        
        // Write to L1 synchronously for immediate availability
        let mut l1_written = false;
        for level in &levels {
            if matches!(level, CacheLevel::L1) {
                self.l1.put(key.clone(), value.clone()).await;
                l1_written = true;
                break;
            }
        }
        
        // Spawn async tasks for L2/L3 writes to avoid blocking
        for level in levels {
            match level {
                CacheLevel::L1 => {
                    // Already handled above
                }
                CacheLevel::L2 => {
                    // Write L2 synchronously for now to ensure it works
                    let _ = self.l2.put(key.clone(), value.clone()).await;
                }
                CacheLevel::L3 => {
                    if let Some(ref l3) = self.l3 {
                        let l3 = l3.clone();
                        let key = key.clone();
                        let value = value.clone();
                        tokio::spawn(async move {
                            let mut l3_guard = l3.write().await;
                            let _ = l3_guard.put(&key, value).await;
                        });
                    }
                }
            }
        }
    }
}
