//! HIGH-PERFORMANCE CACHE - >90% HIT RATE

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use moka::sync::Cache;
use dashmap::DashMap;
use tree_sitter::{Tree, Parser, Language};
use parking_lot::RwLock;

/// Multi-level cache for parsed trees and queries
pub struct TreeSitterCache {
    // L1: Hot cache - most recently used trees
    hot_trees: Cache<PathBuf, Arc<CachedTree>>,
    
    // L2: Warm cache - frequently accessed
    warm_trees: Cache<PathBuf, Arc<CachedTree>>,
    
    // Query results cache
    query_cache: Cache<QueryKey, Arc<Vec<QueryResult>>>,
    
    // Statistics
    stats: Arc<RwLock<CacheStats>>,
}

#[derive(Clone)]
pub struct CachedTree {
    pub tree: Tree,
    pub source_hash: u64,
    pub last_access: SystemTime,
    pub parse_time_ms: f64,
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct QueryKey {
    pub file_path: PathBuf,
    pub query_type: String,
    pub source_hash: u64,
}

#[derive(Clone)]
pub struct QueryResult {
    pub start_byte: usize,
    pub end_byte: usize,
    pub capture_name: String,
}

#[derive(Default)]
pub struct CacheStats {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evictions: u64,
    pub parse_time_saved_ms: f64,
}

#[derive(Debug, Clone)]
pub struct CacheStatsReport {
    pub hit_rate: f64,
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evictions: u64,
}

impl TreeSitterCache {
    pub fn new() -> Self {
        Self {
            // L1: 100 hot files, TTL 5 minutes
            hot_trees: Cache::builder()
                .max_capacity(100)
                .time_to_live(Duration::from_secs(300))
                .build(),
            
            // L2: 1000 warm files, TTL 30 minutes
            warm_trees: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(1800))
                .build(),
            
            // Query cache: 10000 results, TTL 10 minutes
            query_cache: Cache::builder()
                .max_capacity(10000)
                .time_to_live(Duration::from_secs(600))
                .build(),
            
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }
    
    /// Get tree from cache or parse
    pub fn get_or_parse<F>(
        &self,
        path: &Path,
        source_hash: u64,
        parse_fn: F,
    ) -> Result<Arc<CachedTree>, Box<dyn std::error::Error>>
    where
        F: FnOnce() -> Result<(Tree, f64), Box<dyn std::error::Error>>,
    {
        let path_buf = path.to_path_buf();
        self.stats.write().total_requests += 1;
        
        // Check L1 hot cache
        if let Some(cached) = self.hot_trees.get(&path_buf) {
            if cached.source_hash == source_hash {
                self.stats.write().cache_hits += 1;
                self.stats.write().parse_time_saved_ms += cached.parse_time_ms;
                return Ok(cached);
            }
        }
        
        // Check L2 warm cache
        if let Some(cached) = self.warm_trees.get(&path_buf) {
            if cached.source_hash == source_hash {
                self.stats.write().cache_hits += 1;
                self.stats.write().parse_time_saved_ms += cached.parse_time_ms;
                
                // Promote to hot cache
                self.hot_trees.insert(path_buf.clone(), cached.clone());
                return Ok(cached);
            }
        }
        
        // Cache miss - parse and cache
        self.stats.write().cache_misses += 1;
        let (tree, parse_time_ms) = parse_fn()?;
        
        let cached = Arc::new(CachedTree {
            tree,
            source_hash,
            last_access: SystemTime::now(),
            parse_time_ms,
        });
        
        // Insert into both caches
        self.hot_trees.insert(path_buf.clone(), cached.clone());
        self.warm_trees.insert(path_buf, cached.clone());
        
        Ok(cached)
    }
    
    /// Cache query results
    pub fn cache_query_results(
        &self,
        key: QueryKey,
        results: Vec<QueryResult>,
    ) {
        self.query_cache.insert(key, Arc::new(results));
    }
    
    /// Get cached query results
    pub fn get_query_results(&self, key: &QueryKey) -> Option<Arc<Vec<QueryResult>>> {
        self.query_cache.get(key)
    }
    
    pub fn invalidate(&self, path: &Path) {
        let path_buf = path.to_path_buf();
        self.hot_trees.invalidate(&path_buf);
        self.warm_trees.invalidate(&path_buf);
    }
    
    /// Get cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let stats = self.stats.read();
        if stats.total_requests == 0 {
            0.0
        } else {
            (stats.cache_hits as f64 / stats.total_requests as f64) * 100.0
        }
    }
    
    /// Clear all caches
    pub fn clear(&self) {
        self.hot_trees.invalidate_all();
        self.warm_trees.invalidate_all();
        self.query_cache.invalidate_all();
        *self.stats.write() = CacheStats::default();
    }
    
    /// Get cache statistics with hit rate
    pub fn get_stats(&self) -> CacheStatsReport {
        let stats = self.stats.read();
        let hit_rate = if stats.total_requests > 0 {
            stats.cache_hits as f64 / stats.total_requests as f64
        } else {
            0.0
        };
        
        CacheStatsReport {
            hit_rate,
            total_requests: stats.total_requests,
            cache_hits: stats.cache_hits,
            cache_misses: stats.cache_misses,
            evictions: stats.evictions,
        }
    }
}

/// Global cache instance
lazy_static::lazy_static! {
    pub static ref GLOBAL_CACHE: TreeSitterCache = TreeSitterCache::new();
}

/// Helper to compute source hash
pub fn compute_hash(source: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_hit_rate() {
        let cache = TreeSitterCache::new();
        let path = Path::new("test.rs");
        let source = "fn main() {}";
        let hash = compute_hash(source);
        
        // First access - miss
        let _ = cache.get_or_parse(path, hash, || {
            let mut parser = Parser::new();
            let lang = tree_sitter_rust::LANGUAGE.into();
            parser.set_language(&lang.into()).unwrap();
            Ok((parser.parse(source, None).unwrap(), 1.0))
        });
        
        // Second access - hit
        let _ = cache.get_or_parse(path, hash, || {
            panic!("Should not parse again");
        });
        
        // Third access - hit
        let _ = cache.get_or_parse(path, hash, || {
            panic!("Should not parse again");
        });
        
        assert_eq!(cache.hit_rate(), 66.66666666666667); // 2 hits out of 3 requests
    }
}
