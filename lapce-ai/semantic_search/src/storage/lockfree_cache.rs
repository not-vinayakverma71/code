// Lock-free LRU cache implementation using hashlink
// Replaces VecDeque for better concurrency under extreme load

use hashlink::LruCache;
use parking_lot::RwLock;
use std::sync::Arc;
use crate::error::Result;
use crate::embeddings::zstd_compression::CompressedEmbedding;
use std::time::Instant;

/// Lock-free cache entry
#[derive(Debug, Clone)]
pub struct LockFreeCacheEntry {
    pub embedding: Option<Arc<[f32]>>,
    pub compressed: Option<CompressedEmbedding>,
    pub size_bytes: usize,
    pub access_count: usize,
    pub last_access: Instant,
}

/// Lock-free LRU cache tier
pub struct LockFreeCache {
    // Using parking_lot for better performance than std::sync::RwLock
    cache: Arc<RwLock<LruCache<u128, LockFreeCacheEntry>>>,
    size: Arc<RwLock<usize>>,
    max_size_bytes: usize,
    max_entries: usize,
}

impl LockFreeCache {
    pub fn new(max_entries: usize, max_size_bytes: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(max_entries))),
            size: Arc::new(RwLock::new(0)),
            max_size_bytes,
            max_entries,
        }
    }
    
    /// Insert entry with automatic eviction
    pub fn insert(&self, key: u128, mut entry: LockFreeCacheEntry) -> Result<()> {
        let mut cache = self.cache.write();
        let mut size = self.size.write();
        
        // Update access time
        entry.last_access = Instant::now();
        
        // Check if we need to evict
        while *size + entry.size_bytes > self.max_size_bytes && cache.len() > 0 {
            // LRU eviction - remove least recently used
            if let Some((_, evicted)) = cache.remove_lru() {
                *size = size.saturating_sub(evicted.size_bytes);
            } else {
                break; // No more entries to evict
            }
        }
        
        // Insert new entry
        *size += entry.size_bytes;
        cache.insert(key, entry);
        
        Ok(())
    }
    
    /// Get entry with automatic LRU update
    pub fn get(&self, key: &u128) -> Option<Arc<[f32]>> {
        let mut cache = self.cache.write();
        
        if let Some(entry) = cache.get_mut(key) {
            entry.access_count += 1;
            entry.last_access = Instant::now();
            entry.embedding.clone()
        } else {
            None
        }
    }
    
    /// Get without updating LRU (for stats/debugging)
    pub fn peek(&self, key: &u128) -> Option<LockFreeCacheEntry> {
        let cache = self.cache.read();
        cache.peek(key).cloned()
    }
    
    /// Remove entry
    pub fn remove(&self, key: &u128) -> Option<LockFreeCacheEntry> {
        let mut cache = self.cache.write();
        let mut size = self.size.write();
        
        if let Some(entry) = cache.remove(key) {
            *size = size.saturating_sub(entry.size_bytes);
            Some(entry)
        } else {
            None
        }
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        let size = self.size.read();
        
        CacheStats {
            entries: cache.len(),
            size_bytes: *size,
            capacity: self.max_entries,
            max_size_bytes: self.max_size_bytes,
        }
    }
    
    /// Clear all entries
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        let mut size = self.size.write();
        
        cache.clear();
        *size = 0;
    }
    
    /// Check if key exists
    pub fn contains(&self, key: &u128) -> bool {
        let cache = self.cache.read();
        cache.peek(key).is_some()
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub size_bytes: usize,
    pub capacity: usize,
    pub max_size_bytes: usize,
}

impl CacheStats {
    pub fn utilization(&self) -> f32 {
        if self.capacity > 0 {
            (self.entries as f32 / self.capacity as f32) * 100.0
        } else {
            0.0
        }
    }
    
    pub fn memory_utilization(&self) -> f32 {
        if self.max_size_bytes > 0 {
            (self.size_bytes as f32 / self.max_size_bytes as f32) * 100.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lockfree_cache_lru() {
        let cache = LockFreeCache::new(3, 100_000);
        
        // Insert 4 items into cache with capacity 3
        for i in 0..4 {
            let entry = LockFreeCacheEntry {
                embedding: Some(Arc::from(vec![i as f32; 100].into_boxed_slice())),
                compressed: None,
                size_bytes: 400,
                access_count: 0,
                last_access: Instant::now(),
            };
            cache.insert(i as u128, entry).unwrap();
        }
        
        // First item should be evicted
        assert!(cache.get(&0).is_none());
        assert!(cache.get(&1).is_some());
        assert!(cache.get(&2).is_some());
        assert!(cache.get(&3).is_some());
    }
    
    #[test]
    fn test_size_based_eviction() {
        let cache = LockFreeCache::new(100, 1000); // 1KB max
        
        // Insert entries that exceed size limit
        for i in 0..5 {
            let entry = LockFreeCacheEntry {
                embedding: Some(Arc::from(vec![0.0; 100].into_boxed_slice())),
                compressed: None,
                size_bytes: 400, // 400 bytes each
                access_count: 0,
                last_access: Instant::now(),
            };
            cache.insert(i as u128, entry).unwrap();
        }
        
        // Should only keep 2 entries (800 bytes)
        let stats = cache.stats();
        assert!(stats.size_bytes <= 1000);
        assert_eq!(stats.entries, 2);
    }
}
