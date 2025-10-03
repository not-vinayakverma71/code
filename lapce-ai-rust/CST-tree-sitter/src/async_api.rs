//! Async API with proper caching for production use

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use dashmap::DashMap;
use sha2::{Sha256, Digest};

use crate::codex_integration::CodexSymbolExtractor;
use crate::codex_exact_format;
use crate::directory_traversal;

/// Cache entry with timestamp and content hash
#[derive(Clone)]
pub struct CacheEntry {
    pub result: String,
    pub timestamp: SystemTime,
    pub content_hash: String,
}

/// Async Tree-Sitter API with proper caching
pub struct AsyncTreeSitterAPI {
    extractor: Arc<CodexSymbolExtractor>,
    cache: Arc<DashMap<String, CacheEntry>>,
    cache_ttl: Duration,
}

impl AsyncTreeSitterAPI {
    pub fn new() -> Self {
        Self {
            extractor: Arc::new(CodexSymbolExtractor::new()),
            cache: Arc::new(DashMap::new()),
            cache_ttl: Duration::from_secs(300), // 5 minute TTL
        }
    }
    
    /// Generate cache key from file path and content hash
    fn generate_cache_key(&self, file_path: &str, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(file_path.as_bytes());
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// Check if cache entry is still valid
    fn is_cache_valid(&self, entry: &CacheEntry) -> bool {
        if let Ok(elapsed) = entry.timestamp.elapsed() {
            elapsed < self.cache_ttl
        } else {
            false
        }
    }
    
    /// Extract symbols with async and caching
    pub async fn extract_symbols(&self, file_path: &str, source_code: &str) -> Option<String> {
        let cache_key = self.generate_cache_key(file_path, source_code);
        
        // Check cache first
        if let Some(entry) = self.cache.get(&cache_key) {
            if self.is_cache_valid(&entry) {
                // Cache hit!
                return Some(entry.result.clone());
            } else {
                // Expired, remove it
                drop(entry);
                self.cache.remove(&cache_key);
            }
        }
        
        // Cache miss - compute result
        let extractor = self.extractor.clone();
        let file_path = file_path.to_string();
        let source_code = source_code.to_string();
        
        let result = tokio::task::spawn_blocking(move || {
            extractor.extract_from_file(&file_path, &source_code)
        }).await.ok()??;
        
        // Store in cache
        let entry = CacheEntry {
            result: result.clone(),
            timestamp: SystemTime::now(),
            content_hash: cache_key.clone(),
        };
        self.cache.insert(cache_key, entry);
        
        Some(result)
    }
    
    /// Extract from file path async
    pub async fn extract_from_path(&self, file_path: &str) -> Option<String> {
        // Read file async
        let content = tokio::fs::read_to_string(file_path).await.ok()?;
        self.extract_symbols(file_path, &content).await
    }
    
    /// Extract from directory async with progress
    pub async fn extract_from_directory<F>(&self, dir_path: &str, mut progress: F) -> String 
    where
        F: FnMut(usize, usize) + Send + 'static,
    {
        let dir_path = dir_path.to_string();
        let extractor = self.extractor.clone();
        
        tokio::task::spawn_blocking(move || {
            // Get files using available function
            let mut files = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&dir_path) {
                for entry in entries.flatten().take(50) {
                    if let Ok(path) = entry.path().canonicalize() {
                        if let Some(path_str) = path.to_str() {
                            if path_str.ends_with(".rs") || path_str.ends_with(".js") || 
                               path_str.ends_with(".py") || path_str.ends_with(".go") {
                                files.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
            let total = files.len();
            let mut result = String::new();
            
            for (idx, file) in files.iter().enumerate() {
                progress(idx + 1, total);
                if let Ok(content) = std::fs::read_to_string(file) {
                    if let Some(symbols) = extractor.extract_from_file(file, &content) {
                        result.push_str(&format!("# {}\n{}\n", file, symbols));
                    }
                }
            }
            
            result
        }).await.unwrap_or_default()
    }
    
    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize, f64) {
        let total_entries = self.cache.len();
        let mut valid_entries = 0;
        
        for entry in self.cache.iter() {
            if self.is_cache_valid(&entry) {
                valid_entries += 1;
            }
        }
        
        let hit_rate = if total_entries > 0 {
            (valid_entries as f64 / total_entries as f64) * 100.0
        } else {
            0.0
        };
        
        (total_entries, valid_entries, hit_rate)
    }
    
    /// Clear cache
    pub async fn clear_cache(&self) {
        self.cache.clear();
    }
}

/// Production async service with all optimizations
pub struct ProductionAsyncService {
    api: Arc<AsyncTreeSitterAPI>,
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

#[derive(Default, Clone)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_parse_time: Duration,
    pub languages_processed: HashMap<String, u64>,
}

impl ProductionAsyncService {
    pub fn new() -> Self {
        Self {
            api: Arc::new(AsyncTreeSitterAPI::new()),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        }
    }
    
    /// Extract with metrics tracking
    pub async fn extract_with_metrics(&self, file_path: &str, content: &str) -> Option<String> {
        let start = std::time::Instant::now();
        
        // Check if result is cached by trying extraction
        let cache_key = self.api.generate_cache_key(file_path, content);
        let was_cached = self.api.cache.contains_key(&cache_key);
        
        let result = self.api.extract_symbols(file_path, content).await;
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        
        if was_cached {
            metrics.cache_hits += 1;
        } else {
            metrics.cache_misses += 1;
        }
        
        metrics.total_parse_time += start.elapsed();
        
        // Track language
        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        *metrics.languages_processed.entry(ext.to_string()).or_insert(0) += 1;
        
        result
    }
    
    /// Get performance report
    pub async fn performance_report(&self) -> String {
        let metrics = self.metrics.read().await;
        let cache_stats = self.api.cache_stats().await;
        
        let hit_rate = if metrics.total_requests > 0 {
            (metrics.cache_hits as f64 / metrics.total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        let avg_time = if metrics.total_requests > 0 {
            metrics.total_parse_time.as_millis() / metrics.total_requests as u128
        } else {
            0
        };
        
        format!(
            "=== PERFORMANCE REPORT ===\n\
            Total Requests: {}\n\
            Cache Hits: {} ({:.1}%)\n\
            Cache Misses: {}\n\
            Average Parse Time: {}ms\n\
            Cache Entries: {} (valid: {})\n\
            Languages Processed: {:?}\n",
            metrics.total_requests,
            metrics.cache_hits,
            hit_rate,
            metrics.cache_misses,
            avg_time,
            cache_stats.0,
            cache_stats.1,
            metrics.languages_processed
        )
    }
}
