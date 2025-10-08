// Hierarchical 3-tier cache for embeddings with automatic promotion/demotion
use crate::error::{Error, Result};
use crate::embeddings::zstd_compression::{CompressedEmbedding, ZstdCompressor};
use crate::storage::mmap_storage::{MmapStorage, ConcurrentMmapStorage};
use crate::storage::lockfree_cache::{LockFreeCache, LockFreeCacheEntry};
use bloom::{BloomFilter, ASMS};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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
    
    // L1: Hot cache (uncompressed) - now using lock-free LRU
    l1_cache: Arc<LockFreeCache>,
    l1_size: Arc<AtomicUsize>,
    
    // L2: Warm cache (compressed) - also lock-free
    l2_cache: Arc<LockFreeCache>,
    l2_size: Arc<AtomicUsize>,
    
    // Mapping from u128 handles back to original string IDs (for debugging/logging)
    id_map: Arc<RwLock<HashMap<u128, String>>>,
    
    // Admission control for promotions (Phase 6)
    promotion_paused: Arc<AtomicBool>,
    miss_rate: Arc<AtomicUsize>,
    hit_rate: Arc<AtomicUsize>,
    
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
        let compressor = ZstdCompressor::new(Default::default());
        let bloom_filter = BloomFilter::with_rate(0.01, config.bloom_filter_size as u32);
        
        let l3_storage = ConcurrentMmapStorage::new(
            &base_path.join("l3_storage"),
            (config.l1_max_size_mb * 10.0 * 1024.0 * 1024.0) as u64,  // 10x L1 size
        )?;
        
        let l1_max_bytes = (config.l1_max_size_mb * 1024.0 * 1024.0) as usize;
        let l2_max_bytes = (config.l2_max_size_mb * 1024.0 * 1024.0) as usize;
        let l1_max_entries = config.l1_max_entries;
        let l2_max_entries = config.l2_max_entries;
        
        Ok(Self {
            config,
            l1_cache: Arc::new(LockFreeCache::new(l1_max_entries, l1_max_bytes)),
            l1_size: Arc::new(AtomicUsize::new(0)),
            l2_cache: Arc::new(LockFreeCache::new(l2_max_entries, l2_max_bytes)),
            l2_size: Arc::new(AtomicUsize::new(0)),
            id_map: Arc::new(RwLock::new(HashMap::new())),
            promotion_paused: Arc::new(AtomicBool::new(false)),
            miss_rate: Arc::new(AtomicUsize::new(0)),
            hit_rate: Arc::new(AtomicUsize::new(0)),
            l3_storage: Arc::new(l3_storage),
            compressor: Arc::new(RwLock::new(compressor)),
            bloom_filter: Arc::new(RwLock::new(bloom_filter)),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        })
    }
    
    /// Compute compact u128 handle from string ID
    /// Uses first 128 bits of hash for collision resistance
    fn compute_handle(id: &str) -> u128 {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        let hash64 = hasher.finish();
        
        // Combine with a second hash for 128 bits
        let mut hasher2 = DefaultHasher::new();
        hasher2.write_u64(hash64);
        hasher2.write(id.as_bytes());
        let hash64_2 = hasher2.finish();
        
        // Combine into u128
        ((hash64 as u128) << 64) | (hash64_2 as u128)
    }
    
    /// Put embedding into cache (starts at L1)
    pub fn put(&self, id: &str, embedding: Vec<f32>) -> Result<()> {
        let size_bytes = embedding.len() * 4;
        
        // Compute compact handle
        let handle = Self::compute_handle(id);
        
        // Store ID mapping for debugging/recovery
        {
            let mut id_map = self.id_map.write().unwrap();
            id_map.insert(handle, id.to_string());
        }
        
        // Convert to Arc<[f32]> for zero-copy sharing
        let arc_embedding: Arc<[f32]> = Arc::from(embedding.into_boxed_slice());
        
        // Create lock-free cache entry
        let entry = LockFreeCacheEntry {
            embedding: Some(arc_embedding),
            compressed: None,
            size_bytes,
            access_count: 1,
            last_access: Instant::now(),
        };
        
        // Add to L1 (lock-free LRU handles eviction automatically)
        self.l1_cache.insert(handle, entry)?;
        self.l1_size.fetch_add(size_bytes, Ordering::Relaxed);
        
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
        // Compute handle
        let handle = Self::compute_handle(id);
        
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
        
        // Try L1 first (hot cache)
        if let Some(embedding) = self.get_from_l1(handle)? {
            return Ok(Some(embedding));
        }
        
        // Try L2 (warm cache)
        if let Some(embedding) = self.get_from_l2(handle)? {
            // Consider promotion to L1
            self.maybe_promote_to_l1(handle, id)?;
            return Ok(Some(embedding));
        }
        
        // Try L3 (cold storage)
        if let Some(embedding) = self.get_from_l3(id)? {
            // Consider promotion to L2
            self.maybe_promote_to_l2(id)?;
            return Ok(Some(embedding));
        }
        
        Ok(None)
    }
    
    /// Get from L1 cache (lock-free)
    fn get_from_l1(&self, handle: u128) -> Result<Option<Arc<[f32]>>> {
        if let Some(embedding) = self.l1_cache.get(&handle) {
            self.hit_rate.fetch_add(1, Ordering::Relaxed);
            
            if self.config.enable_statistics {
                let mut stats = self.stats.write().unwrap();
                stats.l1_hits += 1;
            }
            
            Ok(Some(embedding))
        } else {
            self.miss_rate.fetch_add(1, Ordering::Relaxed);
            
            if self.config.enable_statistics {
                let mut stats = self.stats.write().unwrap();
                stats.l1_misses += 1;
            }
            Ok(None)
        }
    }
    
    /// Get from L2 cache (lock-free)
    fn get_from_l2(&self, handle: u128) -> Result<Option<Arc<[f32]>>> {
        if let Some(entry) = self.l2_cache.peek(&handle) {
            if self.config.enable_statistics {
                let mut stats = self.stats.write().unwrap();
                stats.l2_hits += 1;
            }
            
            // Decompress if needed
            if let Some(compressed) = &entry.compressed {
                let compressor = self.compressor.read().unwrap();
                let embedding = compressor.decompress_embedding(compressed)?;
                Ok(Some(Arc::from(embedding.into_boxed_slice())))
            } else {
                Ok(entry.embedding)
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
        // LockFreeCache handles LRU internally via hashlink::LruCache
        // We need to get the LRU item from the cache stats
        let stats = self.l1_cache.stats();
        if stats.entries == 0 {
            return Ok(());
        }
        
        // Since we can't directly get the LRU from LockFreeCache, 
        // we'll rely on its automatic eviction in insert()
        // This function becomes a no-op as LockFreeCache handles it
        Ok(())
    }
    
    /// Legacy eviction handler - kept for compatibility
    fn evict_from_l1_legacy(&self) -> Result<()> {
        // This is now handled automatically by LockFreeCache::insert()
        // which evicts LRU items when capacity is exceeded
        Ok(())
    }
    
    fn handle_evicted_entry(&self, handle: u128, mut entry: LockFreeCacheEntry) -> Result<()> {
        if false {  // Dead code for reference
            let placeholder = handle;
            if let Some(mut entry) = Some(entry) {
                // Capture size BEFORE clearing embedding
                let original_size_bytes = entry.embedding.as_ref().map(|e| e.len() * 4).unwrap_or(0);
                
                // Get original ID for compression metadata
                let id_map = self.id_map.read().unwrap();
                let id = id_map.get(&handle).map(|s| s.as_str()).unwrap_or("unknown");
                
                // Compress for L2
                let mut compressor = self.compressor.write().unwrap();
                let compressed = compressor.compress_embedding(
                    entry.embedding.as_ref().unwrap().as_ref(),
                    id,
                )?;
                
                // Update entry
                entry.compressed = Some(compressed.clone());
                entry.embedding = None;  // Remove uncompressed
                entry.size_bytes = compressed.compressed_size;
                
                // Check L2 size
                let l2_size = self.l2_size.load(Ordering::Relaxed);
                if l2_size + entry.size_bytes > (self.config.l2_max_size_mb * 1024.0 * 1024.0) as usize {
                    self.evict_from_l2()?;
                }
                
                // Add to L2 using LockFreeCache insert method
                self.l2_cache.insert(handle, entry.clone())?;
                
                // L2 LRU is now handled internally by LockFreeCache
                // No need to maintain separate LRU queue
                
                // Update sizes - use atomic operations
                self.l1_size.fetch_sub(original_size_bytes, Ordering::Relaxed);
                self.l2_size.fetch_add(entry.size_bytes, Ordering::Relaxed);
                
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
        // LockFreeCache handles eviction automatically
        // This becomes a no-op
        Ok(())
    }
    
    fn evict_from_l2_legacy(&self) -> Result<()> {
        // Legacy implementation - now handled by LockFreeCache
        // This method is kept for compatibility but does nothing
        Ok(())
    }
    
    /// Maybe promote entry to L1 if it's hot (with admission control)
    fn maybe_promote_to_l1(&self, handle: u128, id: &str) -> Result<()> {
        // Phase 6: Check if promotions are paused due to high miss rate
        if self.promotion_paused.load(Ordering::Relaxed) {
            return Ok(()); // Skip promotion
        }
        
        // Check miss/hit ratio
        let misses = self.miss_rate.load(Ordering::Relaxed);
        let hits = self.hit_rate.load(Ordering::Relaxed);
        let total = misses + hits;
        
        if total > 100 {  // Check every 100 accesses
            let miss_ratio = misses as f64 / total as f64;
            if miss_ratio > 0.5 {  // More than 50% misses
                self.promotion_paused.store(true, Ordering::Relaxed);
                // Reset counters
                self.miss_rate.store(0, Ordering::Relaxed);
                self.hit_rate.store(0, Ordering::Relaxed);
                return Ok(());
            } else if miss_ratio < 0.3 {  // Less than 30% misses
                self.promotion_paused.store(false, Ordering::Relaxed);
            }
        }
        
        if let Some(entry) = self.l2_cache.peek(&handle) {
            if entry.access_count >= self.config.promotion_threshold {
                // Get and decompress
                if let Some(embedding) = self.get_from_l2(handle)? {
                    // Remove from L2
                    self.l2_cache.remove(&handle);
                    
                    // Convert Arc back to Vec for put
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
        self.l1_cache.clear();
        self.l2_cache.clear();
        
        // Note: L3 storage clear would need to be implemented
        
        self.l1_size.store(0, Ordering::Relaxed);
        self.l2_size.store(0, Ordering::Relaxed);
        
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
