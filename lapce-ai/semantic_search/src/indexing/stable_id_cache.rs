// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

//! Stable ID-based embedding cache for incremental indexing (CST-B05)
//!
//! This cache maps stable IDs from CST nodes to their pre-computed embeddings,
//! enabling incremental re-indexing by reusing embeddings for unchanged code.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// Cache entry containing embedding and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Embedding vector (384 dimensions for all-MiniLM-L6-v2)
    pub embedding: Vec<f32>,
    
    /// Source text that was embedded
    pub source_text: String,
    
    /// Node kind (for validation)
    pub node_kind: String,
    
    /// Timestamp when entry was created
    pub timestamp: u64,
    
    /// File path this node belongs to
    pub file_path: PathBuf,
}

/// Thread-safe cache mapping stable_id → embedding
pub struct StableIdEmbeddingCache {
    /// Main cache storage
    cache: Arc<RwLock<HashMap<u64, CacheEntry>>>,
    
    /// Maximum cache size (number of entries)
    max_size: usize,
    
    /// Cache hit/miss statistics
    stats: Arc<RwLock<CacheStats>>,
}

#[derive(Debug, Default)]
struct CacheStats {
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl StableIdEmbeddingCache {
    /// Create new cache with default max size (10,000 entries)
    pub fn new() -> Self {
        Self::with_capacity(10_000)
    }
    
    /// Create cache with specified capacity
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::with_capacity(max_size.min(1000)))),
            max_size,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }
    
    /// Get embedding for stable ID (returns None if not cached)
    pub fn get(&self, stable_id: u64) -> Option<CacheEntry> {
        let cache = self.cache.read();
        let result = cache.get(&stable_id).cloned();
        
        // Update stats
        let mut stats = self.stats.write();
        if result.is_some() {
            stats.hits += 1;
        } else {
            stats.misses += 1;
        }
        
        result
    }
    
    /// Insert or update cache entry
    pub fn insert(&self, stable_id: u64, entry: CacheEntry) {
        let mut cache = self.cache.write();
        
        // Evict oldest entries if cache is full
        if cache.len() >= self.max_size {
            self.evict_lru(&mut cache);
        }
        
        cache.insert(stable_id, entry);
    }
    
    /// Batch insert multiple entries
    pub fn insert_batch(&self, entries: Vec<(u64, CacheEntry)>) {
        let mut cache = self.cache.write();
        
        for (stable_id, entry) in entries {
            if cache.len() >= self.max_size {
                self.evict_lru(&mut cache);
            }
            cache.insert(stable_id, entry);
        }
    }
    
    /// Remove entry by stable ID
    pub fn remove(&self, stable_id: u64) -> Option<CacheEntry> {
        self.cache.write().remove(&stable_id)
    }
    
    /// Invalidate all entries for a specific file
    pub fn invalidate_file(&self, file_path: &PathBuf) {
        let mut cache = self.cache.write();
        cache.retain(|_, entry| &entry.file_path != file_path);
    }
    
    /// Clear entire cache
    pub fn clear(&self) {
        self.cache.write().clear();
        let mut stats = self.stats.write();
        *stats = CacheStats::default();
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> (u64, u64, u64, usize) {
        let stats = self.stats.read();
        let size = self.cache.read().len();
        (stats.hits, stats.misses, stats.evictions, size)
    }
    
    /// Get cache hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let stats = self.stats.read();
        let total = stats.hits + stats.misses;
        if total == 0 {
            0.0
        } else {
            stats.hits as f64 / total as f64
        }
    }
    
    /// Check if stable ID exists in cache
    pub fn contains(&self, stable_id: u64) -> bool {
        self.cache.read().contains_key(&stable_id)
    }
    
    /// Get all stable IDs in cache for a file
    pub fn get_file_ids(&self, file_path: &PathBuf) -> Vec<u64> {
        self.cache.read()
            .iter()
            .filter(|(_, entry)| &entry.file_path == file_path)
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Evict least recently used entries (simple FIFO for now)
    fn evict_lru(&self, cache: &mut HashMap<u64, CacheEntry>) {
        // Simple eviction: remove 10% of entries with oldest timestamps
        let mut entries: Vec<_> = cache.iter().map(|(id, entry)| (*id, entry.timestamp)).collect();
        entries.sort_by_key(|(_, ts)| *ts);
        
        let evict_count = (self.max_size / 10).max(1);
        for (id, _) in entries.iter().take(evict_count) {
            cache.remove(id);
        }
        
        let mut stats = self.stats.write();
        stats.evictions += evict_count as u64;
    }
    
    /// Serialize cache to disk (for persistence)
    #[cfg(feature = "persistence")]
    pub fn save_to_disk(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        use std::fs::File;
        use std::io::Write;
        
        let cache = self.cache.read();
        let serialized = bincode::serialize(&*cache)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        let mut file = File::create(path)?;
        file.write_all(&serialized)?;
        Ok(())
    }
    
    /// Load cache from disk
    #[cfg(feature = "persistence")]
    pub fn load_from_disk(path: &PathBuf, max_size: usize) -> Result<Self, std::io::Error> {
        use std::fs::File;
        use std::io::Read;
        
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        
        let cache_data: HashMap<u64, CacheEntry> = bincode::deserialize(&buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        Ok(Self {
            cache: Arc::new(RwLock::new(cache_data)),
            max_size,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        })
    }
}

impl Default for StableIdEmbeddingCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_insert_and_get() {
        let cache = StableIdEmbeddingCache::new();
        
        let entry = CacheEntry {
            embedding: vec![0.1, 0.2, 0.3],
            source_text: "fn test() {}".to_string(),
            node_kind: "function_item".to_string(),
            timestamp: 1000,
            file_path: PathBuf::from("/test.rs"),
        };
        
        cache.insert(12345, entry.clone());
        
        let retrieved = cache.get(12345).unwrap();
        assert_eq!(retrieved.embedding, entry.embedding);
        assert_eq!(retrieved.source_text, entry.source_text);
    }
    
    #[test]
    fn test_cache_stats() {
        let cache = StableIdEmbeddingCache::new();
        
        let entry = CacheEntry {
            embedding: vec![0.1],
            source_text: "test".to_string(),
            node_kind: "identifier".to_string(),
            timestamp: 1000,
            file_path: PathBuf::from("/test.rs"),
        };
        
        cache.insert(1, entry);
        
        // Hit
        cache.get(1);
        // Miss
        cache.get(2);
        
        let (hits, misses, _, _) = cache.stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
        assert_eq!(cache.hit_rate(), 0.5);
    }
    
    #[test]
    fn test_file_invalidation() {
        let cache = StableIdEmbeddingCache::new();
        
        let file1 = PathBuf::from("/file1.rs");
        let file2 = PathBuf::from("/file2.rs");
        
        cache.insert(1, CacheEntry {
            embedding: vec![0.1],
            source_text: "a".to_string(),
            node_kind: "id".to_string(),
            timestamp: 1000,
            file_path: file1.clone(),
        });
        
        cache.insert(2, CacheEntry {
            embedding: vec![0.2],
            source_text: "b".to_string(),
            node_kind: "id".to_string(),
            timestamp: 1000,
            file_path: file2.clone(),
        });
        
        cache.invalidate_file(&file1);
        
        assert!(cache.get(1).is_none());
        assert!(cache.get(2).is_some());
    }
    
    #[test]
    fn test_cache_eviction() {
        let cache = StableIdEmbeddingCache::with_capacity(10);
        
        // Insert more than capacity
        for i in 0..15 {
            cache.insert(i, CacheEntry {
                embedding: vec![i as f32],
                source_text: format!("node_{}", i),
                node_kind: "test".to_string(),
                timestamp: i,
                file_path: PathBuf::from("/test.rs"),
            });
        }
        
        let (_, _, evictions, size) = cache.stats();
        assert!(evictions > 0, "Should have evicted some entries");
        assert!(size <= 10, "Cache size should not exceed capacity");
    }
}
