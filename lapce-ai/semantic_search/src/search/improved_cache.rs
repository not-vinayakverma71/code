// Improved Cache Implementation with Deterministic Key Generation
// Solves the cache hit rate problem

use crate::error::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use sha2::{Sha256, Digest};
use tracing::{info, debug};

/// Improved cache with deterministic key generation and hit tracking
pub struct ImprovedQueryCache {
    cache: Arc<RwLock<HashMap<Vec<u8>, CachedEntry>>>,
    ttl: Duration,
    hit_count: Arc<RwLock<u64>>,
    miss_count: Arc<RwLock<u64>>,
    max_size: usize,
}

#[derive(Clone)]
struct CachedEntry {
    data: Vec<u8>,
    timestamp: Instant,
    access_count: u64,
}

impl ImprovedQueryCache {
    pub fn new(ttl_seconds: u64, max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_seconds),
            hit_count: Arc::new(RwLock::new(0)),
            miss_count: Arc::new(RwLock::new(0)),
            max_size,
        }
    }
    
    /// Generate deterministic cache key from query vector
    /// Uses quantization and SHA-256 for stability
    pub fn generate_cache_key(&self, query_vector: &[f32], limit: usize) -> Vec<u8> {
        let mut hasher = Sha256::new();
        
        // Quantize floats to 2 decimal places for stability
        for &val in query_vector {
            let quantized = (val * 100.0).round() as i32;
            hasher.update(quantized.to_le_bytes());
        }
        
        // Add limit to key
        hasher.update(limit.to_le_bytes());
        
        hasher.finalize().to_vec()
    }
    
    /// Get cached result with automatic TTL check
    pub async fn get<T>(&self, key: &[u8]) -> Option<T> 
    where 
        T: serde::de::DeserializeOwned 
    {
        let mut cache = self.cache.write().await;
        
        // Clone the key for potential removal
        let key_vec = key.to_vec();
        
        if let Some(entry) = cache.get_mut(&key_vec) {
            // Check TTL
            if entry.timestamp.elapsed() > self.ttl {
                debug!("Cache entry expired");
                cache.remove(&key_vec);
                *self.miss_count.write().await += 1;
                return None;
            }
            
            // Update access count
            entry.access_count += 1;
            let access_count = entry.access_count;
            
            // Deserialize data
            if let Ok(data) = bincode::deserialize::<T>(&entry.data) {
                *self.hit_count.write().await += 1;
                info!("Cache HIT (access count: {})", access_count);
                return Some(data);
            }
        }
        
        *self.miss_count.write().await += 1;
        debug!("Cache MISS");
        None
    }
    
    /// Insert data into cache with automatic eviction
    pub async fn insert<T>(&self, key: Vec<u8>, data: T) 
    where 
        T: serde::Serialize 
    {
        self.put(key, data).await;
    }
    
    /// Put data into cache with automatic eviction
    pub async fn put<T>(&self, key: Vec<u8>, data: T) 
    where 
        T: serde::Serialize 
    {
        let serialized = match bincode::serialize(&data) {
            Ok(bytes) => bytes,
            Err(e) => {
                debug!("Failed to serialize for cache: {}", e);
                return;
            }
        };
        
        let mut cache = self.cache.write().await;
        
        // Evict oldest entries if cache is full
        if cache.len() >= self.max_size {
            self.evict_lru(&mut cache).await;
        }
        
        cache.insert(key, CachedEntry {
            data: serialized,
            timestamp: Instant::now(),
            access_count: 0,
        });
        
        debug!("Cached new entry (total: {})", cache.len());
    }
    
    /// Evict least recently used entries
    async fn evict_lru(&self, cache: &mut HashMap<Vec<u8>, CachedEntry>) {
        // Find entry with oldest timestamp and lowest access count
        if let Some(key_to_remove) = cache.iter()
            .min_by_key(|(_, entry)| (entry.timestamp, entry.access_count))
            .map(|(key, _)| key.clone())
        {
            cache.remove(&key_to_remove);
            debug!("Evicted LRU entry");
        }
    }
    
    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let hit_count = *self.hit_count.read().await;
        let miss_count = *self.miss_count.read().await;
        
        let total = hit_count + miss_count;
        let hit_rate = if total > 0 {
            hit_count as f64 / total as f64
        } else {
            0.0
        };
        
        CacheStats {
            total_requests: total,
            hits: hit_count,
            misses: miss_count,
            hit_rate,
            size: cache.len(),
        }
    }
    
    /// Compute cache key for a string query
    pub fn compute_cache_key(&self, query: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        hasher.finalize().to_vec()
    }
    
    /// Compute cache key including filters
    pub fn compute_cache_key_with_filters(&self, query: &str, filters: Option<&str>) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        if let Some(filter) = filters {
            hasher.update(b"|");
            hasher.update(filter.as_bytes());
        }
        hasher.finalize().to_vec()
    }
    
    /// Invalidate cache entries by file path
    pub async fn invalidate_by_path(&self, path: &str) {
        // For now, clear all cache when a file changes
        // In production, we'd want more granular invalidation
        let mut cache = self.cache.write().await;
        let removed = cache.len();
        cache.clear();
        if removed > 0 {
            debug!("Invalidated {} cache entries for path: {}", removed, path);
        }
    }
    
    /// Clear cache
    pub async fn clear(&self) {
        self.cache.write().await.clear();
        *self.hit_count.write().await = 0;
        *self.miss_count.write().await = 0;
        info!("Cache cleared");
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_requests: u64,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub size: usize,
}
