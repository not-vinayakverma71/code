// PHASE 3.1: MULTI-LAYER CACHE IMPLEMENTATION
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Instant, Duration};
use moka::future::Cache as MokaCache;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResult {
    pub query: String,
    pub results: Vec<String>,
    #[serde(skip, default = "Instant::now")]
    pub timestamp: Instant,
    pub hit_count: usize,
}

#[derive(Debug, Clone)]
pub struct CacheMetrics {
    pub total_requests: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub evictions: usize,
    pub hit_rate: f64,
}

pub struct MultiLayerCache {
    // Layer 1: Hot cache (in-memory, fast)
    hot_cache: Arc<MokaCache<String, CachedResult>>,
    
    // Layer 2: Warm cache (in-memory, larger)
    warm_cache: Arc<RwLock<lru::LruCache<String, CachedResult>>>,
    
    // Layer 3: Cold cache (disk-based for persistence)
    cold_cache_dir: PathBuf,
    
    // Metrics tracking
    metrics: Arc<RwLock<CacheMetrics>>,
    
    // Configuration
    hot_cache_size: usize,
    warm_cache_size: usize,
    ttl: Duration,
}

impl MultiLayerCache {
    pub async fn new(hot_size: usize, warm_size: usize, cache_dir: &str) -> Result<Self> {
        // Create hot cache with Moka (automatic eviction, TTL support)
        let hot_cache = MokaCache::builder()
            .max_capacity(hot_size as u64)
            .time_to_live(Duration::from_secs(300)) // 5 minutes TTL
            .build();
        
        // Create warm cache with LRU
        let warm_cache = lru::LruCache::new(
            std::num::NonZeroUsize::new(warm_size).unwrap()
        );
        
        // Create cold cache directory
        let cold_cache_dir = PathBuf::from(cache_dir);
        std::fs::create_dir_all(&cold_cache_dir)?;
        
        Ok(Self {
            hot_cache: Arc::new(hot_cache),
            warm_cache: Arc::new(RwLock::new(warm_cache)),
            cold_cache_dir,
            metrics: Arc::new(RwLock::new(CacheMetrics {
                total_requests: 0,
                cache_hits: 0,
                cache_misses: 0,
                evictions: 0,
                hit_rate: 0.0,
            })),
            hot_cache_size: hot_size,
            warm_cache_size: warm_size,
            ttl: Duration::from_secs(300),
        })
    }
    
    pub async fn get(&self, key: &str) -> Option<CachedResult> {
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        
        // Check hot cache first
        if let Some(result) = self.hot_cache.get(key).await {
            metrics.cache_hits += 1;
            self.update_hit_rate(&mut metrics);
            return Some(result);
        }
        
        // Check warm cache
        let mut warm_cache = self.warm_cache.write().await;
        if let Some(result) = warm_cache.get_mut(key) {
            metrics.cache_hits += 1;
            self.update_hit_rate(&mut metrics);
            
            // Promote to hot cache
            let result_clone = result.clone();
            result.hit_count += 1;
            drop(warm_cache);
            
            self.hot_cache.insert(key.to_string(), result_clone.clone()).await;
            return Some(result_clone);
        }
        drop(warm_cache);
        
        // Check cold cache (disk)
        if let Ok(result) = self.load_from_disk(key).await {
            metrics.cache_hits += 1;
            self.update_hit_rate(&mut metrics);
            
            // Promote to warm cache
            let mut warm_cache = self.warm_cache.write().await;
            warm_cache.put(key.to_string(), result.clone());
            
            return Some(result);
        }
        
        metrics.cache_misses += 1;
        self.update_hit_rate(&mut metrics);
        None
    }
    
    pub async fn put(&self, key: String, results: Vec<String>) -> Result<()> {
        let cached_result = CachedResult {
            query: key.clone(),
            results,
            timestamp: Instant::now(),
            hit_count: 0,
        };
        
        // Add to hot cache
        self.hot_cache.insert(key.clone(), cached_result.clone()).await;
        
        // Also persist to disk for cold cache
        self.save_to_disk(&key, &cached_result).await?;
        
        Ok(())
    }
    
    async fn load_from_disk(&self, key: &str) -> Result<CachedResult> {
        let file_path = self.cold_cache_dir.join(format!("{}.cache", 
            blake3::hash(key.as_bytes()).to_hex()));
        
        let content = tokio::fs::read_to_string(file_path).await?;
        let result: CachedResult = serde_json::from_str(&content)?;
        
        // Check if not expired
        if result.timestamp.elapsed() < self.ttl {
            Ok(result)
        } else {
            Err(anyhow::anyhow!("Cache entry expired"))
        }
    }
    
    async fn save_to_disk(&self, key: &str, result: &CachedResult) -> Result<()> {
        let file_path = self.cold_cache_dir.join(format!("{}.cache", 
            blake3::hash(key.as_bytes()).to_hex()));
        
        let content = serde_json::to_string(result)?;
        tokio::fs::write(file_path, content).await?;
        
        Ok(())
    }
    
    fn update_hit_rate(&self, metrics: &mut CacheMetrics) {
        if metrics.total_requests > 0 {
            metrics.hit_rate = (metrics.cache_hits as f64) / (metrics.total_requests as f64) * 100.0;
        }
    }
    
    pub async fn get_metrics(&self) -> CacheMetrics {
        self.metrics.read().await.clone()
    }
    
    pub async fn clear(&self) {
        self.hot_cache.invalidate_all();
        self.warm_cache.write().await.clear();
        
        // Clear disk cache
        if let Ok(entries) = std::fs::read_dir(&self.cold_cache_dir) {
            for entry in entries.flatten() {
                if entry.path().extension() == Some(std::ffi::OsStr::new("cache")) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
    
    pub async fn warm_cache(&self, popular_queries: Vec<String>) -> Result<()> {
        println!("🔥 Warming cache with {} popular queries", popular_queries.len());
        
        for query in popular_queries {
            // Simulate warming with dummy data
            self.put(query.clone(), vec![format!("warmed_result_for_{}", query)]).await?;
        }
        
        Ok(())
    }
}

// Embedding cache for expensive AWS calls
pub struct EmbeddingCache {
    cache: Arc<MokaCache<String, Vec<f32>>>,
    metrics: Arc<RwLock<CacheMetrics>>,
}

impl EmbeddingCache {
    pub fn new(max_size: usize) -> Self {
        let cache = MokaCache::builder()
            .max_capacity(max_size as u64)
            .time_to_live(Duration::from_secs(3600)) // 1 hour TTL
            .build();
        
        Self {
            cache: Arc::new(cache),
            metrics: Arc::new(RwLock::new(CacheMetrics {
                total_requests: 0,
                cache_hits: 0,
                cache_misses: 0,
                evictions: 0,
                hit_rate: 0.0,
            })),
        }
    }
    
    pub async fn get(&self, text: &str) -> Option<Vec<f32>> {
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        
        if let Some(embedding) = self.cache.get(text).await {
            metrics.cache_hits += 1;
            metrics.hit_rate = (metrics.cache_hits as f64) / (metrics.total_requests as f64) * 100.0;
            Some(embedding)
        } else {
            metrics.cache_misses += 1;
            metrics.hit_rate = (metrics.cache_hits as f64) / (metrics.total_requests as f64) * 100.0;
            None
        }
    }
    
    pub async fn put(&self, text: String, embedding: Vec<f32>) {
        self.cache.insert(text, embedding).await;
    }
    
    pub async fn get_metrics(&self) -> CacheMetrics {
        self.metrics.read().await.clone()
    }
}
