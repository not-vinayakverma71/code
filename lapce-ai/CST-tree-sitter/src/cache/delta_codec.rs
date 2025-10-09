//! Delta compression codec for source text with 0% quality loss guarantees
//! Uses rolling hash chunking and XOR-based deltas with CRC32 validation

use std::sync::Arc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use dashmap::DashMap;
use serde::{Serialize, Deserialize};

/// Chunk size for deduplication (4KB default)
const CHUNK_SIZE: usize = 4096;

/// Minimum size for delta encoding to be worthwhile
const MIN_DELTA_BENEFIT: usize = 256;

/// Delta codec format version
const DELTA_FORMAT_VERSION: u16 = 1;

/// Header for versioned delta-compressed data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaHeader {
    version: u16,
    original_size: usize,
    compressed_size: usize,
    checksum: u32,
    chunk_count: usize,
}

/// CRC32 checksum for validation
fn crc32(data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Rolling hash for chunk boundaries
struct RollingHash {
    window: Vec<u8>,
    hash: u64,
    pos: usize,
}

impl RollingHash {
    fn new(size: usize) -> Self {
        Self {
            window: vec![0; size],
            hash: 0,
            pos: 0,
        }
    }
    
    fn update(&mut self, byte: u8) -> u64 {
        let old = self.window[self.pos];
        self.window[self.pos] = byte;
        self.pos = (self.pos + 1) % self.window.len();
        
        // Simple rolling hash (can be improved with Rabin-Karp)
        self.hash = self.hash.wrapping_sub(old as u64);
        self.hash = self.hash.wrapping_mul(31);
        self.hash = self.hash.wrapping_add(byte as u64);
        self.hash
    }
    
    fn is_boundary(&self) -> bool {
        // Chunk boundary when hash matches pattern
        (self.hash & 0xFFF) == 0xFFF  // Average 4KB chunks
    }
}

/// Shared chunk storage for deduplication
pub struct ChunkStore {
    chunks: DashMap<u64, Arc<Vec<u8>>>,
    stats: ChunkStats,
}

#[derive(Default)]
struct ChunkStats {
    total_chunks: std::sync::atomic::AtomicUsize,
    unique_chunks: std::sync::atomic::AtomicUsize,
    bytes_saved: std::sync::atomic::AtomicUsize,
}

impl ChunkStore {
    pub fn new() -> Self {
        Self {
            chunks: DashMap::new(),
            stats: ChunkStats::default(),
        }
    }
    
    /// Store chunk and return its hash
    pub fn store_chunk(&self, data: &[u8]) -> u64 {
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let hash = hasher.finish();
        
        let is_new = !self.chunks.contains_key(&hash);
        self.chunks.entry(hash)
            .or_insert_with(|| {
                self.stats.unique_chunks.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Arc::new(data.to_vec())
            });
        
        if !is_new {
            self.stats.bytes_saved.fetch_add(data.len(), std::sync::atomic::Ordering::Relaxed);
        }
        self.stats.total_chunks.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        hash
    }
    
    /// Retrieve chunk by hash
    pub fn get_chunk(&self, hash: u64) -> Option<Arc<Vec<u8>>> {
        self.chunks.get(&hash).map(|e| e.clone())
    }
    
    /// Get deduplication statistics
    pub fn stats(&self) -> (usize, usize, usize) {
        (
            self.stats.total_chunks.load(std::sync::atomic::Ordering::Relaxed),
            self.stats.unique_chunks.load(std::sync::atomic::Ordering::Relaxed),
            self.stats.bytes_saved.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}

/// Delta-encoded entry for warm/cold storage
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DeltaEntry {
    /// Base chunk hashes
    pub base_chunks: Vec<u64>,
    /// Delta data (XOR or byte-level diff)
    pub delta: Vec<u8>,
    /// Original size for validation
    pub original_size: usize,
    /// CRC32 of original for validation
    pub original_crc: u32,
}

/// Delta codec for source compression
pub struct DeltaCodec {
    chunk_store: Arc<ChunkStore>,
}

impl DeltaCodec {
    pub fn new(chunk_store: Arc<ChunkStore>) -> Self {
        Self { chunk_store }
    }
    
    /// Split source into chunks for deduplication
    fn chunkify(&self, source: &[u8]) -> Vec<u64> {
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut rolling = RollingHash::new(48);
        
        for (i, &byte) in source.iter().enumerate() {
            rolling.update(byte);
            
            // Check for chunk boundary or max size
            if rolling.is_boundary() || i - start >= CHUNK_SIZE * 2 {
                let chunk = &source[start..=i];
                let hash = self.chunk_store.store_chunk(chunk);
                chunks.push(hash);
                start = i + 1;
            }
        }
        
        // Store final chunk
        if start < source.len() {
            let chunk = &source[start..];
            let hash = self.chunk_store.store_chunk(chunk);
            chunks.push(hash);
        }
        
        chunks
    }
    
    /// Encode source as delta against base chunks
    pub fn encode(&self, source: &[u8]) -> Result<DeltaEntry, String> {
        // For small sources, chunking overhead isn't worth it
        if source.len() < MIN_DELTA_BENEFIT {
            return Err("Source too small for delta encoding".into());
        }
        
        let base_chunks = self.chunkify(source);
        let original_crc = crc32(source);
        
        // For now, store chunks only (no additional delta)
        // In production, we'd compute XOR delta against similar files
        Ok(DeltaEntry {
            base_chunks,
            delta: Vec::new(),
            original_size: source.len(),
            original_crc,
        })
    }
    
    /// Decode delta entry back to original source
    pub fn decode(&self, entry: &DeltaEntry) -> Result<Vec<u8>, String> {
        let mut result = Vec::with_capacity(entry.original_size);
        
        // Reconstruct from chunks
        for &chunk_hash in &entry.base_chunks {
            let chunk = self.chunk_store.get_chunk(chunk_hash)
                .ok_or_else(|| format!("Missing chunk: {}", chunk_hash))?;
            result.extend_from_slice(&chunk);
        }
        
        // Apply delta if present (XOR or patch)
        if !entry.delta.is_empty() {
            // In production, apply delta algorithm
            // For now, delta is empty
        }
        
        // Validate size
        if result.len() != entry.original_size {
            return Err(format!(
                "Size mismatch: expected {}, got {}",
                entry.original_size, result.len()
            ));
        }
        
        // Validate CRC32
        let computed_crc = crc32(&result);
        if computed_crc != entry.original_crc {
            return Err(format!(
                "CRC mismatch: expected {:08x}, got {:08x}",
                entry.original_crc, computed_crc
            ));
        }
        
        Ok(result)
    }
    
    /// Compute delta between two sources (for similar file compression)
    pub fn compute_delta(&self, base: &[u8], target: &[u8]) -> Vec<u8> {
        // Simple XOR delta for demonstration
        // In production, use more sophisticated algorithm (VCDIFF, bsdiff)
        let mut delta = Vec::with_capacity(target.len());
        
        for _i in 0..target.len() {
            if i < base.len() {
                delta.push(target[i] ^ base[i]);
            } else {
                delta.push(target[i]);
            }
        }
        
        delta
    }
    
    /// Apply delta to base to get target
    pub fn apply_delta(&self, base: &[u8], delta: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(delta.len());
        
        for _i in 0..delta.len() {
            if i < base.len() {
                result.push(base[i] ^ delta[i]);
            } else {
                result.push(delta[i]);
            }
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunking_deduplication() {
        let store = Arc::new(ChunkStore::new());
        let codec = DeltaCodec::new(store.clone());
        
        // Create large sources with guaranteed duplicate chunks
        // Use a repeating block that's larger than MIN_CHUNK_SIZE (1024)
        let block = vec![0xAA; 2048]; // 2KB block of 0xAA
        
        // Source 1: Block repeated 10 times = 20KB
        let mut source1 = Vec::new();
        for _ in 0..10 {
            source1.extend_from_slice(&block);
        }
        
        // Source 2: Exact same blocks - guaranteed maximum deduplication
        let mut source2 = Vec::new();
        for _ in 0..10 {
            source2.extend_from_slice(&block);
        }
        
        // Source 3: Slightly different to add variety
        let mut source3 = Vec::new();
        for _i in 0..10 {
            let mut block_copy = block.clone();
            if i == 5 {
                // Modify one block slightly
                block_copy[0] = 0xBB;
            }
            source3.extend_from_slice(&block_copy);
        }
        
        let entry1 = codec.encode(&source1).unwrap();
        let entry2 = codec.encode(&source2).unwrap();
        let entry3 = codec.encode(&source3).unwrap();
        
        // With identical blocks across multiple sources, we MUST have significant deduplication
        let (total, unique, saved) = store.stats();
        
        // Strong assertions - we know we have duplicate blocks
        assert!(total > 0, "Should have created chunks");
        assert!(unique < total, "Should have deduplication: unique {} < total {}", unique, total);
        assert!(saved > 0, "Should have saved bytes through deduplication: saved {}", saved);
        
        // Verify the ratio makes sense - with 3 nearly identical sources, dedup should be high
        let dedup_ratio = (total - unique) as f64 / total as f64;
        assert!(dedup_ratio > 0.5, "Should have at least 50% deduplication with 3 similar sources, got {:.1}%", dedup_ratio * 100.0);
        
        // Decode and verify all three
        let decoded1 = codec.decode(&entry1).unwrap();
        let decoded2 = codec.decode(&entry2).unwrap();
        let decoded3 = codec.decode(&entry3).unwrap();
        
        assert_eq!(decoded1, source1);
        assert_eq!(decoded2, source2);
        assert_eq!(decoded3, source3);
    }
    
    #[test]
    fn test_delta_integrity() {
        let store = Arc::new(ChunkStore::new());
        let codec = DeltaCodec::new(store);
        
        // Make source large enough for delta encoding (> 256 bytes)
        let mut source = Vec::new();
        for _ in 0..10 {
            source.extend_from_slice(b"The quick brown fox jumps over the lazy dog. ");
        }
        
        let entry = codec.encode(&source).unwrap();
        assert_eq!(entry.original_size, source.len());
        assert_eq!(entry.original_crc, crc32(&source));
        
        let decoded = codec.decode(&entry).unwrap();
        assert_eq!(decoded, source, "Decoded must match original exactly");
    }
    
    #[test]
    fn test_corruption_detection() {
        let store = Arc::new(ChunkStore::new());
        let codec = DeltaCodec::new(store);
        
        // Make source large enough for delta encoding (> 256 bytes)
        let mut source = Vec::new();
        for _ in 0..10 {
            source.extend_from_slice(b"Important data that must not be corrupted. ");
        }
        let mut entry = codec.encode(&source).unwrap();
        
        // Corrupt the CRC
        entry.original_crc ^= 0xFF;
        
        let result = codec.decode(&entry);
        assert!(result.is_err(), "Should detect CRC mismatch");
        assert!(result.unwrap_err().contains("CRC mismatch"));
    }
}
