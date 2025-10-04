/// Query cache implementation using Moka and Blake3
/// Provides sub-millisecond cache lookups for repeated queries

use anyhow::Result;
use blake3::Hasher;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedResult {
    pub results: Vec<SearchResult>,
    pub query: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub content: String,
    pub score: f32,
    pub start_line: u32,
    pub end_line: u32,
}

pub struct QueryCache {
    cache: Cache<String, CachedResult>,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
}

impl QueryCache {
    pub fn new(max_capacity: usize, ttl_seconds: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity as u64)
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();
            
        Self {
            cache,
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
        }
    }
    
    /// Generate cache key using Blake3 hash
    pub fn compute_key(&self, query: &str, filters: Option<&SearchFilters>) -> String {
        let mut hasher = Hasher::new();
        hasher.update(query.as_bytes());
        
        if let Some(filters) = filters {
            if let Some(lang) = &filters.language {
                hasher.update(lang.as_bytes());
            }
            if let Some(path) = &filters.path_pattern {
                hasher.update(path.as_bytes());
            }
            hasher.update(&filters.min_score.to_le_bytes());
        }
        
        hasher.finalize().to_hex().to_string()
    }
    
    /// Get cached result if available
    pub async fn get(&self, key: &str) -> Option<CachedResult> {
        let result = self.cache.get(key).await;
        
        if result.is_some() {
            self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            debug!("Cache hit for key: {}", key);
        } else {
            self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            debug!("Cache miss for key: {}", key);
        }
        
        result
    }
    
    /// Store result in cache
    pub async fn insert(&self, key: String, result: CachedResult) {
        self.cache.insert(key.clone(), result).await;
        debug!("Cached result for key: {}", key);
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        
        CacheStats {
            hits,
            misses,
            hit_rate,
            size: self.cache.entry_count(),
        }
    }
    
    /// Clear cache
    pub async fn clear(&self) {
        self.cache.invalidate_all().await;
        self.hits.store(0, std::sync::atomic::Ordering::Relaxed);
        self.misses.store(0, std::sync::atomic::Ordering::Relaxed);
        info!("Cache cleared");
    }
}

#[derive(Debug, Clone)]
pub struct SearchFilters {
    pub language: Option<String>,
    pub path_pattern: Option<String>,
    pub min_score: f32,
}

#[derive(Debug)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub size: u64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache Stats: {} hits, {} misses, {:.1}% hit rate, {} entries",
            self.hits, self.misses, self.hit_rate, self.size
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache_operations() {
        let cache = QueryCache::new(100, 60);
        
        let result = CachedResult {
            results: vec![],
            query: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };
        
        let key = cache.compute_key("test query", None);
        cache.insert(key.clone(), result.clone()).await;
        
        let cached = cache.get(&key).await;
        assert!(cached.is_some());
        
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }
}
