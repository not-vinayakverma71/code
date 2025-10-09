//! Frozen tier - disk-backed cache for ultra-low memory usage
//! Phase 4 optimization: Offload cold data to disk

use std::path::{Path, PathBuf};
use std::fs;
use std::sync::Arc;
use dashmap::DashMap;
use bytes::Bytes;
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, Instant};

use super::delta_codec::DeltaEntry;

/// Frozen entry metadata (kept in RAM)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrozenMetadata {
    /// File path hash
    pub path_hash: u64,
    /// Disk file path
    pub disk_path: PathBuf,
    /// Original source size
    pub original_size: usize,
    /// Compressed size on disk
    pub compressed_size: usize,
    /// CRC32 for validation
    pub crc32: u32,
    /// Last access time
    pub last_access: SystemTime,
    /// Compression method
    pub compression: CompressionMethod,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CompressionMethod {
    Zstd { level: i32 },
    Lzma { level: u32 },
    Brotli { quality: u32 },
}

/// Frozen entry on disk
#[derive(Serialize, Deserialize)]
pub struct FrozenEntry {
    /// Delta entry if available
    pub delta_entry: Option<DeltaEntry>,
    /// Compressed tree + source data
    pub compressed_data: Vec<u8>,
    /// Source hash for validation
    pub source_hash: u64,
    /// Original size before compression
    pub original_size: usize,
    /// Metadata
    pub metadata: FrozenMetadata,
}

/// Frozen tier storage manager
pub struct FrozenTier {
    /// Base directory for frozen files
    base_dir: PathBuf,
    /// In-memory index (path -> metadata)
    index: DashMap<PathBuf, FrozenMetadata>,
    /// Compression level for new entries
    compression_level: i32,
    /// Maximum frozen storage size
    max_disk_bytes: usize,
    /// Current disk usage
    current_disk_bytes: Arc<std::sync::atomic::AtomicUsize>,
}

impl FrozenTier {
    /// Create new frozen tier
    pub fn new(base_dir: PathBuf, max_disk_gb: f64) -> Result<Self, String> {
        // Create base directory
        fs::create_dir_all(&base_dir)
            .map_err(|e| format!("Failed to create frozen tier directory: {}", e))?;
        
        // Load existing index
        let index = Self::load_index(&base_dir)?;
        
        // Calculate current disk usage
        let current_bytes = index.iter()
            .map(|entry| entry.compressed_size)
            .sum();
        
        Ok(Self {
            base_dir,
            index: DashMap::from_iter(index.into_iter().map(|m| {
                let path = PathBuf::from(format!("{:016x}", m.path_hash));
                (path, m)
            })),
            compression_level: 19, // Zstd max compression
            max_disk_bytes: (max_disk_gb * 1024.0 * 1024.0 * 1024.0) as usize,
            current_disk_bytes: Arc::new(std::sync::atomic::AtomicUsize::new(current_bytes)),
        })
    }
    
    /// Load index from disk
    fn load_index(base_dir: &Path) -> Result<Vec<FrozenMetadata>, String> {
        let index_path = base_dir.join("frozen_index.bincode");
        if !index_path.exists() {
            return Ok(Vec::new());
        }
        
        let data = fs::read(&index_path)
            .map_err(|e| format!("Failed to read frozen index: {}", e))?;
        
        bincode::deserialize(&data)
            .map_err(|e| format!("Failed to deserialize frozen index: {}", e))
    }
    
    /// Save index to disk
    fn save_index(&self) -> Result<(), String> {
        let index_path = self.base_dir.join("frozen_index.bincode");
        
        let metadata: Vec<FrozenMetadata> = self.index
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        
        let data = bincode::serialize(&metadata)
            .map_err(|e| format!("Failed to serialize frozen index: {}", e))?;
        
        fs::write(&index_path, data)
            .map_err(|e| format!("Failed to write frozen index: {}", e))?;
        
        Ok(())
    }
    
    /// Freeze data to disk
    pub fn freeze(
        &self,
        path: PathBuf,
        source: &Bytes,
        delta_entry: Option<DeltaEntry>,
        tree_data: Vec<u8>,
    ) -> Result<(), String> {
        let start = Instant::now();
        
        // Calculate path hash
        let path_hash = Self::hash_path(&path);
        let disk_filename = format!("{:016x}.frozen", path_hash);
        let disk_path = self.base_dir.join(&disk_filename);
        
        // Combine tree and source
        let mut combined = Vec::with_capacity(tree_data.len() + source.len() + 8);
        combined.extend_from_slice(&(tree_data.len() as u64).to_le_bytes());
        combined.extend_from_slice(&tree_data);
        combined.extend_from_slice(source);
        
        // Compress with maximum compression
        let compressed = zstd::encode_all(&combined[..], self.compression_level)
            .map_err(|e| format!("Failed to compress for freezing: {}", e))?;
        
        // Calculate CRC32
        let crc32 = crc32fast::hash(&combined);
        
        // Create frozen entry
        let frozen = FrozenEntry {
            delta_entry,
            compressed_data: compressed.clone(),
            source_hash: Self::hash_bytes(source),
            original_size: combined.len(),
            metadata: FrozenMetadata {
                path_hash,
                disk_path: disk_path.clone(),
                original_size: combined.len(),
                compressed_size: compressed.len(),
                crc32,
                last_access: SystemTime::now(),
                compression: CompressionMethod::Zstd { 
                    level: self.compression_level 
                },
            },
        };
        
        // Serialize and write to disk
        let serialized = bincode::serialize(&frozen)
            .map_err(|e| format!("Failed to serialize frozen entry: {}", e))?;
        
        fs::write(&disk_path, serialized)
            .map_err(|e| format!("Failed to write frozen file: {}", e))?;
        
        // Update index
        self.index.insert(path.clone(), frozen.metadata.clone());
        
        // Update disk usage
        self.current_disk_bytes.fetch_add(
            compressed.len(),
            std::sync::atomic::Ordering::Relaxed
        );
        
        // Save index periodically
        if self.index.len() % 100 == 0 {
            self.save_index()?;
        }
        
        let elapsed = start.elapsed();
        log::debug!(
            "Froze {} to disk: {} -> {} bytes ({:.1}% reduction) in {:?}",
            path.display(),
            combined.len(),
            compressed.len(),
            (1.0 - compressed.len() as f64 / combined.len() as f64) * 100.0,
            elapsed
        );
        
        Ok(())
    }
    
    /// Thaw frozen data from disk
    pub fn thaw(&self, path: &PathBuf) -> Result<(Bytes, Option<DeltaEntry>, Vec<u8>), String> {
        let start = Instant::now();
        
        // Look up in index
        let metadata = self.index.get(path)
            .ok_or_else(|| format!("Path not found in frozen index: {}", path.display()))?
            .clone();
        
        // Read from disk
        let serialized = fs::read(&metadata.disk_path)
            .map_err(|e| format!("Failed to read frozen file: {}", e))?;
        
        // Deserialize
        let frozen: FrozenEntry = bincode::deserialize(&serialized)
            .map_err(|e| format!("Failed to deserialize frozen entry: {}", e))?;
        
        // Decompress
        let decompressed = zstd::decode_all(&frozen.compressed_data[..])
            .map_err(|e| format!("Failed to decompress frozen data: {}", e))?;
        
        // Validate CRC
        let actual_crc = crc32fast::hash(&decompressed);
        if actual_crc != metadata.crc32 {
            return Err(format!(
                "CRC mismatch for frozen data: expected {:08x}, got {:08x}",
                metadata.crc32, actual_crc
            ));
        }
        
        // Split tree and source
        if decompressed.len() < 8 {
            return Err("Frozen data too small".to_string());
        }
        
        let tree_len = u64::from_le_bytes(
            decompressed[0..8].try_into().unwrap()
        ) as usize;
        
        if decompressed.len() < 8 + tree_len {
            return Err("Frozen data corrupted".to_string());
        }
        
        let _tree_data = decompressed[8..8 + tree_len].to_vec();
        let _source = Bytes::from(decompressed[8 + tree_len..].to_vec());
        
        // Update access time
        self.index.get_mut(path).map(|mut entry| {
            entry.last_access = SystemTime::now();
        });
        
        let elapsed = start.elapsed();
        log::debug!(
            "Thawed {} from disk: {} bytes in {:?}",
            path.display(),
            metadata.compressed_size,
            elapsed
        );
        
        Ok((source, frozen.delta_entry, tree_data))
    }
    
    /// Check if path is frozen
    pub fn is_frozen(&self, path: &PathBuf) -> bool {
        self.index.contains_key(path)
    }
    
    /// Get frozen tier statistics
    pub fn stats(&self) -> FrozenTierStats {
        let total_entries = self.index.len();
        let total_disk_bytes = self.current_disk_bytes.load(
            std::sync::atomic::Ordering::Relaxed
        );
        let avg_compression = if total_entries > 0 {
            let (original, compressed): (usize, usize) = self.index
                .iter()
                .map(|e| (e.original_size, e.compressed_size))
                .fold((0, 0), |(o, c), (no, nc)| (o + no, c + nc));
            1.0 - (compressed as f64 / original as f64)
        } else {
            0.0
        };
        
        FrozenTierStats {
            total_entries,
            total_disk_bytes,
            avg_compression_ratio: avg_compression,
            max_disk_bytes: self.max_disk_bytes,
        }
    }
    
    /// Evict least recently used entries if over limit
    pub fn evict_if_needed(&self) -> Result<usize, String> {
        let current = self.current_disk_bytes.load(
            std::sync::atomic::Ordering::Relaxed
        );
        
        if current <= self.max_disk_bytes {
            return Ok(0);
        }
        
        // Sort by last access time
        let mut entries: Vec<_> = self.index
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        
        entries.sort_by_key(|(_, m)| m.last_access);
        
        let mut evicted = 0;
        let mut freed_bytes = 0;
        let target_bytes = current - self.max_disk_bytes;
        
        for (path, metadata) in entries {
            if freed_bytes >= target_bytes {
                break;
            }
            
            // Remove file
            if let Err(e) = fs::remove_file(&metadata.disk_path) {
                log::warn!("Failed to remove frozen file: {}", e);
                continue;
            }
            
            // Remove from index
            self.index.remove(&path);
            
            freed_bytes += metadata.compressed_size;
            evicted += 1;
        }
        
        self.current_disk_bytes.fetch_sub(
            freed_bytes,
            std::sync::atomic::Ordering::Relaxed
        );
        
        // Save updated index
        self.save_index()?;
        
        Ok(evicted)
    }
    
    /// Hash a path to u64
    fn hash_path(path: &Path) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Hash bytes to u64
    fn hash_bytes(data: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Debug, Clone)]
pub struct FrozenTierStats {
    pub total_entries: usize,
    pub total_disk_bytes: usize,
    pub avg_compression_ratio: f64,
    pub max_disk_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_freeze_thaw() {
        let dir = tempdir().unwrap();
        let frozen = FrozenTier::new(dir.path().to_path_buf(), 0.1).unwrap();
        
        let path = PathBuf::from("test.rs");
        let _source = Bytes::from("fn main() { println!(\"Hello\"); }");
        let _tree_data = vec![1, 2, 3, 4, 5];
        
        // Freeze
        frozen.freeze(path.clone(), &source, None, tree_data.clone()).unwrap();
        
        // Check it's frozen
        assert!(frozen.is_frozen(&path));
        
        // Thaw
        let (thawed_source, delta, thawed_tree) = frozen.thaw(&path).unwrap();
        
        // Verify
        assert_eq!(thawed_source, source);
        assert!(delta.is_none());
        assert_eq!(thawed_tree, tree_data);
    }
    
    #[test]
    fn test_compression_ratio() {
        let dir = tempdir().unwrap();
        let frozen = FrozenTier::new(dir.path().to_path_buf(), 0.1).unwrap();
        
        // Freeze highly compressible data
        let path = PathBuf::from("repetitive.txt");
        let _source = Bytes::from("a".repeat(10000));
        let _tree_data = vec![0; 1000];
        
        frozen.freeze(path.clone(), &source, None, tree_data).unwrap();
        
        let stats = frozen.stats();
        assert!(stats.avg_compression_ratio > 0.9, "Should compress well");
        assert!(stats.total_disk_bytes < 11000, "Should be much smaller");
    }
}
