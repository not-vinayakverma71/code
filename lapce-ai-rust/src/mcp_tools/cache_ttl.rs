/// Tool Result Caching with TTL
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use serde_json::Value;

pub struct CacheSystem {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    config: CacheConfig,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub default_ttl: Duration,
    pub max_entries: usize,
    pub max_size_bytes: usize,
}

#[derive(Clone)]
struct CacheEntry {
    key: String,
    value: Value,
    created_at: Instant,
    ttl: Duration,
    hit_count: u64,
    size_bytes: usize,
}

impl CacheSystem {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    pub async fn get(&self, key: &str) -> Option<Value> {
        if !self.config.enabled {
            return None;
        }
        
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(key) {
            if entry.created_at.elapsed() < entry.ttl {
                entry.hit_count += 1;
                return Some(entry.value.clone());
            } else {
                // Entry expired
                cache.remove(key);
            }
        }
        
        None
    }
    
    pub async fn set(&self, key: String, value: Value, ttl: Option<Duration>) {
        if !self.config.enabled {
            return;
        }
        
        let size_bytes = serde_json::to_vec(&value).map(|v| v.len()).unwrap_or(0);
        
        // Check size limit
        if size_bytes > self.config.max_size_bytes {
            return;
        }
        
        let mut cache = self.cache.write().await;
        
        // Evict if at capacity
        if cache.len() >= self.config.max_entries {
            self.evict_lru(&mut cache);
        }
        
        cache.insert(key.clone(), CacheEntry {
            key,
            value,
            created_at: Instant::now(),
            ttl: ttl.unwrap_or(self.config.default_ttl),
            hit_count: 0,
            size_bytes,
        });
    }
    
    fn evict_lru(&self, cache: &mut HashMap<String, CacheEntry>) {
        if let Some(key) = cache.values()
            .min_by_key(|e| e.hit_count)
            .map(|e| e.key.clone()) 
        {
            cache.remove(&key);
        }
    }
    
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
    
    pub async fn get_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        
        let total_size = cache.values().map(|e| e.size_bytes).sum();
        let total_hits = cache.values().map(|e| e.hit_count).sum();
        
        CacheStats {
            entry_count: cache.len(),
            total_size_bytes: total_size,
            total_hits,
            hit_rate: 0.0, // Would need to track misses
        }
    }
}

#[derive(Serialize)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_size_bytes: usize,
    pub total_hits: u64,
    pub hit_rate: f64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_ttl: Duration::from_secs(300), // 5 minutes
            max_entries: 1000,
            max_size_bytes: 10 * 1024 * 1024, // 10MB
        }
    }
}
