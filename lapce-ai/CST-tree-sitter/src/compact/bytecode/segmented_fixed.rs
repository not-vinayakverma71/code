//! Fixed segmented bytecode storage - Phase 4c
//! Splits bytecode into segments for on-demand loading

use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use moka::sync::Cache;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use super::{BytecodeStream, Opcode};

/// Size of each bytecode segment (256KB)
const SEGMENT_SIZE: usize = 256 * 1024;

/// Maximum segments in memory
const MAX_CACHED_SEGMENTS: u64 = 100;

/// Segmented bytecode stream
pub struct SegmentedBytecodeStream {
    /// Segment index: node_index -> (segment_id, offset_in_segment)
    index: Vec<(u16, u32)>,
    
    /// Segment metadata
    segments: Vec<SegmentMetadata>,
    
    /// Storage directory
    storage_dir: PathBuf,
    
    /// In-memory segment cache (LRU)
    segment_cache: Cache<u16, Arc<Vec<u8>>>,
    
    /// String tables (kept in memory)
    pub kind_names: Vec<String>,
    pub field_names: Vec<String>,
    
    /// Metadata
    pub node_count: usize,
    pub source_len: usize,
    
    /// Statistics
    stats: Arc<SegmentStats>,
}

#[derive(Clone)]
struct SegmentMetadata {
    /// Segment ID
    id: u16,
    /// File path
    path: PathBuf,
    /// Compressed size on disk
    compressed_size: usize,
    /// Uncompressed size
    uncompressed_size: usize,
    /// Node range [start, end)
    node_range: (usize, usize),
    /// CRC32 for validation
    crc32: u32,
}

#[derive(Default)]
pub struct SegmentStats {
    pub segments_loaded: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub total_load_time_ms: AtomicU64,
}

impl SegmentedBytecodeStream {
    /// Create from regular bytecode stream
    pub fn from_bytecode_stream(
        stream: BytecodeStream,
        storage_dir: PathBuf,
    ) -> io::Result<Self> {
        fs::create_dir_all(&storage_dir)?;
        
        let mut segments = Vec::new();
        let mut index = Vec::with_capacity(stream.node_count.max(1));
        let mut current_segment = Vec::new();
        let mut segment_id = 0u16;
        let mut node_start = 0;
        let mut node_index = 0;
        
        // Simple segmentation: split by size
        let bytes = &stream.bytes;
        let mut pos = 0;
        
        while pos < bytes.len() {
            // Start of this node in the segment
            let offset_in_segment = current_segment.len() as u32;
            index.push((segment_id, offset_in_segment));
            
            // Copy bytes until we find a good split point or reach segment size
            let chunk_end = (pos + SEGMENT_SIZE).min(bytes.len());
            let chunk = &bytes[pos..chunk_end];
            current_segment.extend_from_slice(chunk);
            pos = chunk_end;
            node_index += 1;
            
            // Check if segment is full or we're done
            if current_segment.len() >= SEGMENT_SIZE || pos >= bytes.len() {
                // Compress and save segment
                let compressed = zstd::encode_all(&current_segment[..], 3)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                let crc32 = crc32fast::hash(&current_segment);
                
                let segment_path = storage_dir.join(format!("seg_{:04}.zst", segment_id));
                fs::write(&segment_path, &compressed)?;
                
                segments.push(SegmentMetadata {
                    id: segment_id,
                    path: segment_path,
                    compressed_size: compressed.len(),
                    uncompressed_size: current_segment.len(),
                    node_range: (node_start, node_index),
                    crc32,
                });
                
                // Start new segment
                current_segment.clear();
                segment_id += 1;
                node_start = node_index;
            }
        }
        
        // Create segment cache
        let segment_cache = Cache::builder()
            .max_capacity(MAX_CACHED_SEGMENTS)
            .build();
        
        Ok(Self {
            index,
            segments,
            storage_dir,
            segment_cache,
            kind_names: stream.kind_names.clone(),
            field_names: stream.field_names.clone(),
            node_count: stream.node_count,
            source_len: stream.source_len,
            stats: Arc::new(SegmentStats::default()),
        })
    }
    
    /// Load segment from disk
    fn load_segment(&self, segment_id: u16) -> io::Result<Arc<Vec<u8>>> {
        // Check cache first
        if let Some(cached) = self.segment_cache.get(&segment_id) {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(cached);
        }
        
        self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
        let start = std::time::Instant::now();
        
        // Find segment metadata
        let meta = self.segments.iter()
            .find(|s| s.id == segment_id)
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                format!("Segment {} not found", segment_id)
            ))?;
        
        // Read and decompress
        let compressed = fs::read(&meta.path)?;
        let decompressed = zstd::decode_all(&compressed[..])
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        
        // Validate CRC32
        let crc32 = crc32fast::hash(&decompressed);
        if crc32 != meta.crc32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Segment CRC32 mismatch"
            ));
        }
        
        let segment = Arc::new(decompressed);
        
        // Cache the segment
        self.segment_cache.insert(segment_id, segment.clone());
        
        let elapsed = start.elapsed();
        self.stats.total_load_time_ms.fetch_add(
            elapsed.as_millis() as u64,
            Ordering::Relaxed
        );
        self.stats.segments_loaded.fetch_add(1, Ordering::Relaxed);
        
        Ok(segment)
    }
    
    /// Get navigator for a specific node
    pub fn navigator(&self, node_index: usize) -> io::Result<SegmentedNavigator> {
        if node_index >= self.index.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Node index {} out of range", node_index)
            ));
        }
        
        let (segment_id, offset) = self.index[node_index];
        let segment = self.load_segment(segment_id)?;
        
        Ok(SegmentedNavigator {
            stream: self,
            current_segment: Some(segment),
            current_segment_id: segment_id,
        })
    }
    
    /// Get statistics snapshot
    pub fn stats(&self) -> SegmentStatsSnapshot {
        SegmentStatsSnapshot {
            segments_loaded: self.stats.segments_loaded.load(Ordering::Relaxed),
            cache_hits: self.stats.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.stats.cache_misses.load(Ordering::Relaxed),
            total_load_time_ms: self.stats.total_load_time_ms.load(Ordering::Relaxed),
            segments_count: self.segments.len(),
            total_compressed_size: self.segments.iter()
                .map(|s| s.compressed_size)
                .sum(),
            total_uncompressed_size: self.segments.iter()
                .map(|s| s.uncompressed_size)
                .sum(),
        }
    }
}

/// Navigator for segmented bytecode
pub struct SegmentedNavigator<'a> {
    stream: &'a SegmentedBytecodeStream,
    current_segment: Option<Arc<Vec<u8>>>,
    current_segment_id: u16,
}

impl<'a> SegmentedNavigator<'a> {
    /// Load segment for node
    pub fn load_for_node(&mut self, node_index: usize) -> io::Result<()> {
        if node_index >= self.stream.index.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Node index out of range"
            ));
        }
        
        let (segment_id, _) = self.stream.index[node_index];
        
        if segment_id != self.current_segment_id {
            let segment = self.stream.load_segment(segment_id)?;
            self.current_segment = Some(segment);
            self.current_segment_id = segment_id;
        }
        
        Ok(())
    }
    
    /// Get current segment data
    pub fn current_data(&self) -> Option<&[u8]> {
        self.current_segment.as_ref().map(|s| s.as_slice())
    }
}

/// Statistics snapshot
#[derive(Debug, Clone)]
pub struct SegmentStatsSnapshot {
    pub segments_loaded: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_load_time_ms: u64,
    pub segments_count: usize,
    pub total_compressed_size: usize,
    pub total_uncompressed_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_segmentation() {
        let dir = tempdir().unwrap();
        
        // Create a bytecode stream
        let mut stream = BytecodeStream::new();
        
        // Add some data
        for i in 0..1000 {
            stream.bytes.extend_from_slice(&i.to_le_bytes());
        }
        stream.node_count = 100;
        
        // Create segmented version
        let segmented = SegmentedBytecodeStream::from_bytecode_stream(
            stream,
            dir.path().to_path_buf()
        ).unwrap();
        
        // Check stats
        let stats = segmented.stats();
        assert!(stats.segments_count > 0);
        assert!(stats.total_compressed_size > 0);
    }
}
