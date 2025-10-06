use std::time::Duration;
use dashmap::DashMap;

use super::{
    count_min_sketch::CountMinSketch,
    types::CacheKey,
};

pub struct AccessCounter {
    pub counts: DashMap<CacheKey, CountMinSketch>,
    pub window: Duration,
}

impl AccessCounter {
    pub fn new() -> Self {
        Self {
            counts: DashMap::new(),
            window: Duration::from_secs(3600), // 1 hour window
        }
    }
    
    pub fn record(&self, key: &CacheKey) {
        self.counts.entry(key.clone())
            .or_insert_with(|| CountMinSketch::new(0.01, 0.99))
            .increment();
    }
    
    pub fn frequency(&self, key: &CacheKey) -> u32 {
        self.counts.get(key)
            .map(|sketch| sketch.estimate())
            .unwrap_or(0)
    }
    
    pub fn decay(&self) {
        // Periodically decay counts to adapt to changing patterns
        for mut entry in self.counts.iter_mut() {
            entry.value_mut().decay(0.9);
        }
    }
}
