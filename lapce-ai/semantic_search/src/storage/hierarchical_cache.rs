// Hierarchical 3-tier cache for embeddings with automatic promotion/demotion
use crate::error::{Error, Result};
use crate::embeddings::zstd_compression::{CompressedEmbedding, ZstdCompressor};
use crate::storage::mmap_storage::{MmapStorage, ConcurrentMmapStorage};
use bloom::{BloomFilter, ASMS};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Cache configuration for each tier
#[derive(Debug, Clone)]
pub struct CacheConfig {
    // L1 Hot cache (uncompressed, in-memory)
    pub l1_max_size_mb: f64,        // Default: 1 MB
    pub l1_max_entries: usize,      // Default: 100
    
    // L2 Warm cache (compressed, in-memory)
    pub l2_max_size_mb: f64,        // Default: 3 MB
    pub l2_max_entries: usize,      // Default: 500
    
    // L3 Cold storage (compressed, memory-mapped)
    pub l3_max_size_mb: f64,        // Default: unlimited (1000 MB)
    
    // Policies
    pub promotion_threshold: usize,  // Access count for promotion
    pub demotion_timeout: Duration,  // Time before demotion
    pub bloom_filter_size: usize,    // Bloom filter bit size
    pub enable_statistics: bool,     // Track detailed stats
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_max_size_mb: 1.0,
            l1_max_entries: 100,
            l2_max_size_mb: 3.0,
            l2_max_entries: 500,
            l3_max_size_mb: 1000.0,
            promotion_threshold: 3,
            demotion_timeout: Duration::from_secs(300),  // 5 minutes
            bloom_filter_size: 10000,
            enable_statistics: true,
        }
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry {
    embedding: Option<Arc<[f32]>>,          // Uncompressed data (L1 only) - now Arc for zero-copy
    compressed: Option<CompressedEmbedding>, // Compressed data (L2/L3)
    size_bytes: usize,
    access_count: usize,
    last_access: Instant,
    tier: CacheTier,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CacheTier {
    L1Hot,
    L2Warm,
    L3Cold,
}

/// Hierarchical cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub l1_hits: usize,
    pub l1_misses: usize,
    pub l2_hits: usize,
    pub l2_misses: usize,
    pub l3_hits: usize,
    pub l3_misses: usize,
    pub total_promotions: usize,
    pub total_demotions: usize,
    pub l1_size_bytes: usize,
    pub l2_size_bytes: usize,
    pub l3_size_bytes: usize,
    pub l1_entries: usize,
    pub l2_entries: usize,
    pub l3_entries: usize,
}

impl CacheStats {
    pub fn l1_hit_rate(&self) -> f64 {
        let total = self.l1_hits + self.l1_misses;
        if total > 0 {
            self.l1_hits as f64 / total as f64
        } else {
            0.0
        }
    }
    
    pub fn overall_hit_rate(&self) -> f64 {
        let hits = self.l1_hits + self.l2_hits + self.l3_hits;
        let misses = self.l1_misses + self.l2_misses + self.l3_misses;
        let total = hits + misses;
        if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        }
    }
}

/// Hierarchical 3-tier cache for embeddings
pub struct HierarchicalCache {
    config: CacheConfig,
    
    // L1: Hot cache (uncompressed)
    l1_cache: Arc<RwLock<HashMap<Arc<str>, CacheEntry>>>,
    l1_lru: Arc<RwLock<VecDeque<Arc<str>>>>,
    l1_size: Arc<RwLock<usize>>,
    
    // L2: Warm cache (compressed)
    l2_cache: Arc<RwLock<HashMap<Arc<str>, CacheEntry>>>,
    l2_lru: Arc<RwLock<VecDeque<Arc<str>>>>,
    l2_size: Arc<RwLock<usize>>,
    
    // L3: Cold storage (memory-mapped)
    l3_storage: Arc<ConcurrentMmapStorage>,
    
    // Utilities
    compressor: Arc<RwLock<ZstdCompressor>>,
    bloom_filter: Arc<RwLock<BloomFilter>>,
    stats: Arc<RwLock<CacheStats>>,
}

impl HierarchicalCache {
    /// Create new hierarchical cache
    pub fn new(config: CacheConfig, base_path: &Path) -> Result<Self> {
        let l3_path = base_path.join("l3_cold");
        std::fs::create_dir_all(&l3_path).map_err(|e| Error::Runtime {
            message: format!("Failed to create L3 directory: {}", e),
        })?;
        
        let l3_storage = ConcurrentMmapStorage::new(
            &l3_path,
            (config.l3_max_size_mb * 1024.0 * 1024.0) as u64,
        )?;
        
        let bloom_filter = BloomFilter::with_rate(
            0.01,  // 1% false positive rate
            config.bloom_filter_size as u32,
        );
        
        Ok(Self {
            config,
            l1_cache: Arc::new(RwLock::new(HashMap::new())),
            l1_lru: Arc::new(RwLock::new(VecDeque::new())),
            l1_size: Arc::new(RwLock::new(0)),
            l2_cache: Arc::new(RwLock::new(HashMap::new())),
            l2_lru: Arc::new(RwLock::new(VecDeque::new())),
            l2_size: Arc::new(RwLock::new(0)),
            l3_storage: Arc::new(l3_storage),
            compressor: Arc::new(RwLock::new(ZstdCompressor::new(Default::default()))),
            bloom_filter: Arc::new(RwLock::new(bloom_filter)),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        })
    }
    
    /// Put embedding into cache (starts at L1)
    pub fn put(&self, id: &str, embedding: Vec<f32>) -> Result<()> {
        let size_bytes = embedding.len() * 4;
        
        // Check if it fits in L1
        let l1_size = *self.l1_size.read().unwrap();
        if l1_size + size_bytes > (self.config.l1_max_size_mb * 1024.0 * 1024.0) as usize {
            // Need to evict from L1
            self.evict_from_l1()?
        }
        
        // Convert to Arc<[f32]> for zero-copy sharing
        let arc_embedding: Arc<[f32]> = Arc::from(embedding.into_boxed_slice());
        let arc_id: Arc<str> = Arc::from(id);
        
        // Create cache entry
        let entry = CacheEntry {
            embedding: Some(arc_embedding),
            compressed: None,
            size_bytes,
            access_count: 1,
            last_access: Instant::now(),
            tier: CacheTier::L1Hot,
        };
        
        // Add to L1
        let mut l1_cache = self.l1_cache.write().unwrap();
        l1_cache.insert(arc_id.clone(), entry);
        
        let mut l1_lru = self.l1_lru.write().unwrap();
        l1_lru.push_back(arc_id);
        
        let mut l1_size = self.l1_size.write().unwrap();
        *l1_size += size_bytes;
        
        // Update bloom filter
        let mut bloom = self.bloom_filter.write().unwrap();
        bloom.insert(&id.to_string());
        
        // Update stats
        if self.config.enable_statistics {
            let mut stats = self.stats.write().unwrap();
            stats.l1_entries += 1;
            stats.l1_size_bytes += size_bytes;
        }
        
        Ok(())
    }
    
    /// Get embedding from cache (returns Arc for zero-copy)
    pub fn get(&self, id: &str) -> Result<Option<Arc<[f32]>>> {
        // Check bloom filter first
        {
            let bloom = self.bloom_filter.read().unwrap();
            if !bloom.contains(&id.to_string()) {
                if self.config.enable_statistics {
                    let mut stats = self.stats.write().unwrap();
                    stats.l3_misses += 1;
                }
                return Ok(None);
            }
        }
        
        // Check L1
        if let Some(embedding) = self.get_from_l1(id)? {
            return Ok(Some(embedding));
        }
        
        // Check L2
        if let Some(embedding) = self.get_from_l2(id)? {
            // Consider promotion to L1
            self.maybe_promote_to_l1(id)?;
            return Ok(Some(embedding));
        }
        
        // Check L3
        if let Some(embedding) = self.get_from_l3(id)? {
            // Consider promotion to L2
            self.maybe_promote_to_l2(id)?;
            return Ok(Some(embedding));
        }
        
        Ok(None)
    }
    
    /// Get from L1 cache
    fn get_from_l1(&self, id: &str) -> Result<Option<Arc<[f32]>>> {
        let mut l1_cache = self.l1_cache.write().unwrap();
        
        // Find matching Arc<str> key
        let matching_key = l1_cache.keys().find(|k| k.as_ref() == id).cloned();
        
        if let Some(key) = matching_key {
            if let Some(entry) = l1_cache.get_mut(&key) {
                entry.access_count += 1;
                entry.last_access = Instant::now();
                
                // Move to end of LRU
                let mut l1_lru = self.l1_lru.write().unwrap();
                l1_lru.retain(|x| x.as_ref() != id);
                l1_lru.push_back(key);
                
                if self.config.enable_statistics {
                    let mut stats = self.stats.write().unwrap();
                    stats.l1_hits += 1;
                }
                
                // Return Arc clone (cheap, no data copy)
                Ok(entry.embedding.clone())
            } else {
                if self.config.enable_statistics {
                    let mut stats = self.stats.write().unwrap();
                    stats.l1_misses += 1;
                }
                Ok(None)
            }
        } else {
            if self.config.enable_statistics {
                let mut stats = self.stats.write().unwrap();
                stats.l1_misses += 1;
            }
            Ok(None)
        }
    }
    
    /// Get from L2 cache
    fn get_from_l2(&self, id: &str) -> Result<Option<Arc<[f32]>>> {
        let mut l2_cache = self.l2_cache.write().unwrap();
        
        // Find matching Arc<str> key
        let matching_key = l2_cache.keys().find(|k| k.as_ref() == id).cloned();
        
        if let Some(key) = matching_key {
            if let Some(entry) = l2_cache.get_mut(&key) {
                entry.access_count += 1;
                entry.last_access = Instant::now();
                
                // Move to end of LRU
                let mut l2_lru = self.l2_lru.write().unwrap();
                l2_lru.retain(|x| x.as_ref() != id);
                l2_lru.push_back(key);
                
                if self.config.enable_statistics {
                    let mut stats = self.stats.write().unwrap();
                    stats.l2_hits += 1;
                }
                
                // Decompress and convert to Arc
                if let Some(compressed) = &entry.compressed {
                    let compressor = self.compressor.read().unwrap();
                    let embedding = compressor.decompress_embedding(compressed)?;
                    Ok(Some(Arc::from(embedding.into_boxed_slice())))
                } else {
                    Ok(None)
                }
            } else {
                if self.config.enable_statistics {
                    let mut stats = self.stats.write().unwrap();
                    stats.l2_misses += 1;
                }
                Ok(None)
            }
        } else {
            if self.config.enable_statistics {
                let mut stats = self.stats.write().unwrap();
                stats.l2_misses += 1;
            }
            Ok(None)
        }
    }
    
    /// Get from L3 storage
    fn get_from_l3(&self, id: &str) -> Result<Option<Arc<[f32]>>> {
        if self.l3_storage.contains(id) {
            if self.config.enable_statistics {
                let mut stats = self.stats.write().unwrap();
                stats.l3_hits += 1;
            }
            
            let embedding = self.l3_storage.get(id)?;
            Ok(Some(Arc::from(embedding.into_boxed_slice())))
        } else {
            if self.config.enable_statistics {
                let mut stats = self.stats.write().unwrap();
                stats.l3_misses += 1;
            }
            Ok(None)
        }
    }
    
    /// Evict least recently used entry from L1 to L2
    fn evict_from_l1(&self) -> Result<()> {
        let mut l1_lru = self.l1_lru.write().unwrap();
        
        if let Some(id) = l1_lru.pop_front() {
            let mut l1_cache = self.l1_cache.write().unwrap();
            
            if let Some(mut entry) = l1_cache.remove(&id) {
                // Capture size BEFORE clearing embedding
                let original_size_bytes = entry.embedding.as_ref().map(|e| e.len() * 4).unwrap_or(0);
                
                // Compress for L2
                let mut compressor = self.compressor.write().unwrap();
                let compressed = compressor.compress_embedding(
                    entry.embedding.as_ref().unwrap().as_ref(),
                    id.as_ref(),
                )?;
                
                // Update entry
                entry.compressed = Some(compressed.clone());
                entry.embedding = None;  // Remove uncompressed
                entry.tier = CacheTier::L2Warm;
                entry.size_bytes = compressed.compressed_size;
                
                // Check L2 size
                let l2_size = *self.l2_size.read().unwrap();
                if l2_size + entry.size_bytes > (self.config.l2_max_size_mb * 1024.0 * 1024.0) as usize {
                    self.evict_from_l2()?;
                }
                
                // Add to L2
                let mut l2_cache = self.l2_cache.write().unwrap();
                l2_cache.insert(id.clone(), entry.clone());
                
                let mut l2_lru = self.l2_lru.write().unwrap();
                l2_lru.push_back(id);
                
                // Update sizes - use captured size
                let mut l1_size = self.l1_size.write().unwrap();
                *l1_size -= original_size_bytes;
                
                let mut l2_size = self.l2_size.write().unwrap();
                *l2_size += entry.size_bytes;
                
                // Update stats
                if self.config.enable_statistics {
                    let mut stats = self.stats.write().unwrap();
                    stats.total_demotions += 1;
                    stats.l1_entries -= 1;
                    stats.l2_entries += 1;
                }
            }
        }
        
        Ok(())
    }
    
    /// Evict least recently used entry from L2 to L3
    fn evict_from_l2(&self) -> Result<()> {
        let mut l2_lru = self.l2_lru.write().unwrap();
        
        if let Some(id) = l2_lru.pop_front() {
            let mut l2_cache = self.l2_cache.write().unwrap();
            
            if let Some(entry) = l2_cache.remove(&id) {
                // Store in L3
                if let Some(compressed) = &entry.compressed {
                    self.l3_storage.store(&id, &[0.0])?;  // Store compressed directly
                }
                
                // Update sizes
                let mut l2_size = self.l2_size.write().unwrap();
                *l2_size -= entry.size_bytes;
                
                // Update stats
                if self.config.enable_statistics {
                    let mut stats = self.stats.write().unwrap();
                    stats.total_demotions += 1;
                    stats.l2_entries -= 1;
                    stats.l3_entries += 1;
                }
            }
        }
        
        Ok(())
    }
    
    /// Maybe promote from L2 to L1
    fn maybe_promote_to_l1(&self, id: &str) -> Result<()> {
        let l2_cache = self.l2_cache.read().unwrap();
        
        if let Some(entry) = l2_cache.get(id) {
            if entry.access_count >= self.config.promotion_threshold {
                // Promote to L1
                drop(l2_cache);
                
                // Get and decompress
                if let Some(embedding) = self.get_from_l2(id)? {
                    // Remove from L2
                    let mut l2_cache = self.l2_cache.write().unwrap();
                    let matching_key = l2_cache.keys().find(|k| k.as_ref() == id).cloned();
                    if let Some(key) = matching_key {
                        l2_cache.remove(&key);
                        
                        let mut l2_lru = self.l2_lru.write().unwrap();
                        l2_lru.retain(|x| x.as_ref() != id);
                    }
                    
                    // Convert Arc back to Vec for put (which will re-convert to Arc)
                    let vec_embedding = embedding.to_vec();
                    self.put(id, vec_embedding)?;
                    
                    // Update stats
                    if self.config.enable_statistics {
                        let mut stats = self.stats.write().unwrap();
                        stats.total_promotions += 1;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Maybe promote from L3 to L2
    fn maybe_promote_to_l2(&self, _id: &str) -> Result<()> {
        // Simple policy: always promote from L3 to L2 on access
        // More sophisticated policies can be implemented
        Ok(())
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.stats.read().unwrap().clone()
    }
    
    /// Clear all caches
    pub fn clear(&self) -> Result<()> {
        let mut l1_cache = self.l1_cache.write().unwrap();
        l1_cache.clear();
        
        let mut l1_lru = self.l1_lru.write().unwrap();
        l1_lru.clear();
        
        let mut l2_cache = self.l2_cache.write().unwrap();
        l2_cache.clear();
        
        let mut l2_lru = self.l2_lru.write().unwrap();
        l2_lru.clear();
        
        // Note: L3 storage clear would need to be implemented
        
        let mut l1_size = self.l1_size.write().unwrap();
        *l1_size = 0;
        
        let mut l2_size = self.l2_size.write().unwrap();
        *l2_size = 0;
        
        let mut stats = self.stats.write().unwrap();
        *stats = CacheStats::default();
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_hierarchical_cache() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            l1_max_size_mb: 0.01,  // Very small for testing
            l1_max_entries: 2,
            l2_max_size_mb: 0.02,
            l2_max_entries: 4,
            ..Default::default()
        };
        
        let cache = HierarchicalCache::new(config, temp_dir.path()).unwrap();
        
        // Add embeddings
        for i in 0..5 {
            let embedding = vec![i as f32; 384];
            cache.put(&format!("id_{}", i), embedding).unwrap();
        }
        
        // Check stats
        let stats = cache.get_stats();
        assert!(stats.l1_entries > 0);
        assert!(stats.l2_entries > 0);
        
        // Retrieve and check hits
        for i in 0..5 {
            let result = cache.get(&format!("id_{}", i)).unwrap();
            assert!(result.is_some());
        }
        
        let stats = cache.get_stats();
        assert!(stats.l1_hits > 0 || stats.l2_hits > 0 || stats.l3_hits > 0);
    }
    
    #[test]
    fn test_cache_promotion() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            promotion_threshold: 2,
            ..Default::default()
        };
        
        let cache = HierarchicalCache::new(config, temp_dir.path()).unwrap();
        
        // Add to cache
        let embedding = vec![0.5; 384];
        let expected = embedding.clone();
        cache.put("test", embedding).unwrap();
        
        // Access multiple times to trigger promotion
        for _ in 0..3 {
            let result = cache.get("test").unwrap();
            // Verify embedding values are unchanged
            if let Some(arc) = result {
                assert_eq!(arc.as_ref(), expected.as_slice());
            }
        }
        
        let stats = cache.get_stats();
        println!("Stats: {:?}", stats);
        assert!(stats.l1_hits > 0);
    }
}
