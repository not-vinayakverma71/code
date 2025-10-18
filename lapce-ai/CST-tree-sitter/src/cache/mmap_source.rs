//! Memory-mapped source storage - Phase 4b
//! Reduces RAM usage by keeping source files memory-mapped

use std::path::PathBuf;
use std::sync::Arc;
use std::fs;
use std::io;
use memmap2::{Mmap, MmapOptions};
use dashmap::DashMap;
use bytes::Bytes;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use parking_lot::RwLock;

/// Memory-mapped source entry
pub struct MmapEntry {
    /// Memory map
    pub mmap: Arc<Mmap>,
    /// Original file path
    pub path: PathBuf,
    /// File size
    pub size: usize,
    /// Access count for eviction
    pub access_count: AtomicU64,
}

/// Memory-mapped source storage manager
pub struct MmapSourceStorage {
    /// Storage directory for source files
    storage_dir: PathBuf,
    /// Active memory maps
    mmaps: DashMap<u64, Arc<MmapEntry>>,
    /// LRU tracking for eviction
    lru_tracker: Arc<RwLock<Vec<u64>>>,
    /// Maximum number of mapped files
    max_mapped_files: usize,
    /// Statistics
    stats: MmapStats,
}

#[derive(Default)]
pub struct MmapStats {
    pub total_mapped: AtomicUsize,
    pub total_bytes_mapped: AtomicUsize,
    pub page_faults: AtomicU64,
    pub evictions: AtomicU64,
}

impl MmapSourceStorage {
    /// Create new mmap storage
    pub fn new(storage_dir: PathBuf, max_mapped_files: usize) -> Result<Self, io::Error> {
        fs::create_dir_all(&storage_dir)?;
        
        Ok(Self {
            storage_dir,
            mmaps: DashMap::new(),
            lru_tracker: Arc::new(RwLock::new(Vec::with_capacity(max_mapped_files))),
            max_mapped_files,
            stats: MmapStats::default(),
        })
    }
    
    /// Store source and return memory-mapped access
    pub fn store_source(&self, hash: u64, source: &[u8]) -> Result<Arc<Mmap>, io::Error> {
        // Check if already mapped
        if let Some(entry) = self.mmaps.get(&hash) {
            entry.access_count.fetch_add(1, Ordering::Relaxed);
            self.update_lru(hash);
            return Ok(entry.mmap.clone());
        }
        
        // Evict if at capacity
        while self.mmaps.len() >= self.max_mapped_files {
            self.evict_lru();
        }
        
        // Write to file
        let file_path = self.storage_dir.join(format!("{:016x}.src", hash));
        fs::write(&file_path, source)?;
        
        // Create memory map
        let file = fs::File::open(&file_path)?;
        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)?
        };
        
        let mmap = Arc::new(mmap);
        let entry = Arc::new(MmapEntry {
            mmap: mmap.clone(),
            path: file_path,
            size: source.len(),
            access_count: AtomicU64::new(1),
        });
        
        self.mmaps.insert(hash, entry);
        self.stats.total_mapped.fetch_add(1, Ordering::Relaxed);
        self.stats.total_bytes_mapped.fetch_add(source.len(), Ordering::Relaxed);
        self.update_lru(hash);
        
        Ok(mmap)
    }
    
    /// Get memory-mapped source
    pub fn get_source(&self, hash: u64) -> Option<Arc<Mmap>> {
        self.mmaps.get(&hash).map(|entry| {
            entry.access_count.fetch_add(1, Ordering::Relaxed);
            self.update_lru(hash);
            entry.mmap.clone()
        })
    }
    
    /// Store and get as Bytes (for compatibility)
    pub fn store_as_bytes(&self, hash: u64, source: &[u8]) -> Result<Bytes, io::Error> {
        let mmap = self.store_source(hash, source)?;
        Ok(Bytes::copy_from_slice(&mmap[..]))
    }
    
    /// Get as Bytes (for compatibility)  
    pub fn get_as_bytes(&self, hash: u64) -> Option<Bytes> {
        self.get_source(hash)
            .map(|mmap| Bytes::copy_from_slice(&mmap[..]))
    }
    
    /// Convert existing Bytes to mmap
    pub fn migrate_to_mmap(&self, hash: u64, bytes: &Bytes) -> Result<Arc<Mmap>, io::Error> {
        self.store_source(hash, bytes)
    }
    
    /// Update LRU tracking
    fn update_lru(&self, hash: u64) {
        let mut lru = self.lru_tracker.write();
        lru.retain(|&h| h != hash);
        lru.push(hash);
        
        // Keep bounded
        if lru.len() > self.max_mapped_files * 2 {
            lru.drain(0..self.max_mapped_files);
        }
    }
    
    /// Evict least recently used
    fn evict_lru(&self) {
        // Find least accessed entry
        let mut min_access = u64::MAX;
        let mut evict_hash = None;
        
        for item in self.mmaps.iter() {
            let hash = *item.key();
            let access = item.value().access_count.load(Ordering::Relaxed);
            if access < min_access {
                min_access = access;
                evict_hash = Some(hash);
            }
        }
        
        if let Some(hash) = evict_hash {
            if let Some((_, entry)) = self.mmaps.remove(&hash) {
                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
                self.stats.total_mapped.fetch_sub(1, Ordering::Relaxed);
                self.stats.total_bytes_mapped.fetch_sub(entry.size, Ordering::Relaxed);
                
                // Optionally remove file
                let _ = fs::remove_file(&entry.path);
            }
        }
    }
    
    /// Clear all mappings
    pub fn clear(&self) {
        let count = self.mmaps.len();
        self.mmaps.clear();
        self.stats.total_mapped.store(0, Ordering::Relaxed);
        self.stats.total_bytes_mapped.store(0, Ordering::Relaxed);
        
        // Clean up files
        if let Ok(entries) = fs::read_dir(&self.storage_dir) {
            for entry in entries.flatten() {
                if entry.path().extension() == Some("src".as_ref()) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }
    
    /// Get statistics
    pub fn stats(&self) -> MmapStatsSnapshot {
        MmapStatsSnapshot {
            total_mapped: self.stats.total_mapped.load(Ordering::Relaxed),
            total_bytes_mapped: self.stats.total_bytes_mapped.load(Ordering::Relaxed),
            page_faults: self.stats.page_faults.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            active_maps: self.mmaps.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MmapStatsSnapshot {
    pub total_mapped: usize,
    pub total_bytes_mapped: usize,
    pub page_faults: u64,
    pub evictions: u64,
    pub active_maps: usize,
}

/// Zero-copy source view backed by mmap
pub struct MmapSourceView {
    mmap: Arc<Mmap>,
    offset: usize,
    len: usize,
}

impl MmapSourceView {
    pub fn new(mmap: Arc<Mmap>, offset: usize, len: usize) -> Self {
        assert!(offset + len <= mmap.len());
        Self { mmap, offset, len }
    }
    
    pub fn as_slice(&self) -> &[u8] {
        &self.mmap[self.offset..self.offset + self.len]
    }
    
    pub fn to_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(self.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_mmap_storage() {
        let dir = tempdir().unwrap();
        let storage = MmapSourceStorage::new(dir.path().to_path_buf(), 10).unwrap();
        
        // Store source
        let source = b"Hello, memory-mapped world!";
        let hash = 12345;
        
        let mmap = storage.store_source(hash, source).unwrap();
        assert_eq!(&mmap[..], source);
        
        // Get source
        let retrieved = storage.get_source(hash).unwrap();
        assert_eq!(&retrieved[..], source);
        
        // Stats
        let stats = storage.stats();
        assert_eq!(stats.total_mapped, 1);
        assert_eq!(stats.total_bytes_mapped, source.len());
    }
    
    #[test]
    fn test_lru_eviction() {
        let dir = tempdir().unwrap();
        let storage = MmapSourceStorage::new(dir.path().to_path_buf(), 3).unwrap();
        
        // Fill to capacity
        for i in 0..5 {
            let source = format!("Source {}", i);
            storage.store_source(i, source.as_bytes()).unwrap();
        }
        
        // Should have evicted some
        let stats = storage.stats();
        assert!(stats.active_maps <= 3);
        assert!(stats.evictions > 0);
    }
    
    #[test]
    fn test_zero_copy_view() {
        let dir = tempdir().unwrap();
        let storage = MmapSourceStorage::new(dir.path().to_path_buf(), 10).unwrap();
        
        let source = b"Hello, world!";
        let hash = 99999;
        let mmap = storage.store_source(hash, source).unwrap();
        
        // Create view
        let view = MmapSourceView::new(mmap, 7, 5);
        assert_eq!(view.as_slice(), b"world");
    }
}
