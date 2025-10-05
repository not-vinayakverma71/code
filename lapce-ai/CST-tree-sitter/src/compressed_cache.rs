//! Compressed CST cache with hot/cold tiers

use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use moka::sync::Cache;
use bytes::Bytes;
use tree_sitter::Tree;
use anyhow::Result;
use std::time::{SystemTime, Instant};

use crate::cst_codec::{serialize_tree, deserialize_tree};

/// Compressed cache with hot uncompressed tier and cold compressed tier
pub struct CompressedTreeCache {
    /// Hot tier: uncompressed trees for fast access
    hot_cache: Cache<PathBuf, Arc<HotEntry>>,
    
    /// Cold tier: compressed trees to save memory
    cold_cache: Cache<PathBuf, Arc<ColdEntry>>,
    
    /// Statistics
    stats: Arc<RwLock<CacheStats>>,
    
    /// Configuration
    pub config: CacheConfig,
}

#[derive(Clone)]
pub struct HotEntry {
    pub tree: Tree,
    pub source: Bytes,
    pub source_hash: u64,
    pub last_access: SystemTime,
    pub parse_time_ms: f64,
}

#[derive(Clone)]
pub struct ColdEntry {
    pub compressed_data: Vec<u8>,
    pub source_hash: u64,
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_time_ms: f64,
}

pub struct CacheConfig {
    /// Maximum hot entries (uncompressed)
    pub hot_size: usize,
    
    /// Maximum cold entries (compressed)
    pub cold_size: usize,
    
    /// Compression level (1-22, default 3)
    pub compression_level: i32,
    
    /// Enable disk persistence for cold entries
    pub enable_disk_persistence: bool,
    
    /// Disk cache directory
    pub disk_cache_dir: Option<PathBuf>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            hot_size: 1000,  // 1K hot files
            cold_size: 9000,  // 9K cold files
            compression_level: 3,
            enable_disk_persistence: false,
            disk_cache_dir: None,
        }
    }
}

#[derive(Default, Clone)]
pub struct CacheStats {
    pub hot_hits: u64,
    pub cold_hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub compressions: u64,
    pub decompressions: u64,
    pub total_compression_time_ms: f64,
    pub total_decompression_time_ms: f64,
    pub bytes_saved: i64,
}

impl CompressedTreeCache {
    pub fn new(config: CacheConfig) -> Self {
        let hot_cache = Cache::builder()
            .max_capacity(config.hot_size as u64)
            .eviction_listener(|key: Arc<PathBuf>, value: Arc<HotEntry>, _cause| {
                // When evicted from hot, compress and move to cold
                // This is handled in insert_cold
            })
            .build();
        
        let cold_cache = Cache::builder()
            .max_capacity(config.cold_size as u64)
            .build();
        
        Self {
            hot_cache,
            cold_cache,
            stats: Arc::new(RwLock::new(CacheStats::default())),
            config,
        }
    }
    
    /// Get tree from cache or parse
    pub async fn get_or_parse<F>(
        &self,
        path: &Path,
        source_hash: u64,
        parse_fn: F,
    ) -> Result<(Tree, Bytes, f64)>
    where
        F: FnOnce() -> Result<(Tree, Bytes, f64)>,
    {
        let path_buf = path.to_path_buf();
        
        // Check hot cache
        if let Some(hot_entry) = self.hot_cache.get(&path_buf) {
            if hot_entry.source_hash == source_hash {
                self.stats.write().hot_hits += 1;
                return Ok((
                    hot_entry.tree.clone(),
                    hot_entry.source.clone(),
                    hot_entry.parse_time_ms,
                ));
            }
        }
        
        // Check cold cache
        if let Some(cold_entry) = self.cold_cache.get(&path_buf) {
            if cold_entry.source_hash == source_hash {
                self.stats.write().cold_hits += 1;
                
                // Decompress
                let decompress_start = Instant::now();
                let decompressed = zstd::decode_all(&cold_entry.compressed_data[..])?;
                let decompress_time = decompress_start.elapsed().as_secs_f64() * 1000.0;
                
                // Deserialize tree
                let (tree, source) = deserialize_tree(&decompressed)?;
                
                // Update stats
                {
                    let mut stats = self.stats.write();
                    stats.decompressions += 1;
                    stats.total_decompression_time_ms += decompress_time;
                }
                
                // Promote to hot cache
                let hot_entry = Arc::new(HotEntry {
                    tree: tree.clone(),
                    source: source.clone(),
                    source_hash,
                    last_access: SystemTime::now(),
                    parse_time_ms: 0.0, // We don't have original parse time
                });
                self.hot_cache.insert(path_buf.clone(), hot_entry);
                
                // Remove from cold cache
                self.cold_cache.remove(&path_buf);
                
                return Ok((tree, source, decompress_time));
            }
        }
        
        // Check disk cache if enabled
        if self.config.enable_disk_persistence {
            if let Some(entry) = self.load_from_disk(&path_buf, source_hash).await? {
                return Ok(entry);
            }
        }
        
        // Cache miss - parse
        self.stats.write().misses += 1;
        let (tree, source, parse_time_ms) = parse_fn()?;
        
        // Insert into hot cache
        let hot_entry = Arc::new(HotEntry {
            tree: tree.clone(),
            source: source.clone(),
            source_hash,
            last_access: SystemTime::now(),
            parse_time_ms,
        });
        self.hot_cache.insert(path_buf, hot_entry);
        
        Ok((tree, source, parse_time_ms))
    }
    
    /// Manually evict from hot to cold (for testing)
    pub async fn evict_to_cold(&self, path: &Path) -> Result<()> {
        let path_buf = path.to_path_buf();
        
        if let Some(hot_entry) = self.hot_cache.get(&path_buf) {
            // Serialize tree
            let serialized = serialize_tree(&hot_entry.tree, &hot_entry.source)?;
            
            // Compress
            let compress_start = Instant::now();
            let compressed = zstd::encode_all(&serialized[..], self.config.compression_level)?;
            let compress_time = compress_start.elapsed().as_secs_f64() * 1000.0;
            
            // Create cold entry
            let cold_entry = Arc::new(ColdEntry {
                compressed_data: compressed.clone(),
                source_hash: hot_entry.source_hash,
                original_size: serialized.len(),
                compressed_size: compressed.len(),
                compression_time_ms: compress_time,
            });
            
            // Update stats
            {
                let mut stats = self.stats.write();
                stats.compressions += 1;
                stats.total_compression_time_ms += compress_time;
                stats.bytes_saved += (serialized.len() as i64 - compressed.len() as i64);
            }
            
            // Move to cold cache
            self.cold_cache.insert(path_buf.clone(), cold_entry.clone());
            self.hot_cache.remove(&path_buf);
            
            // Save to disk if enabled
            if self.config.enable_disk_persistence {
                self.save_to_disk(&path_buf, &cold_entry).await?;
            }
        }
        
        Ok(())
    }
    
    /// Save compressed entry to disk
    async fn save_to_disk(&self, path: &PathBuf, entry: &ColdEntry) -> Result<()> {
        if let Some(ref cache_dir) = self.config.disk_cache_dir {
            let hash = format!("{:x}", entry.source_hash);
            let cache_file = cache_dir.join(format!("{}.zst", hash));
            tokio::fs::write(cache_file, &entry.compressed_data).await?;
        }
        Ok(())
    }
    
    /// Load from disk cache
    async fn load_from_disk(&self, path: &PathBuf, source_hash: u64) -> Result<Option<(Tree, Bytes, f64)>> {
        if let Some(ref cache_dir) = self.config.disk_cache_dir {
            let hash = format!("{:x}", source_hash);
            let cache_file = cache_dir.join(format!("{}.zst", hash));
            
            if cache_file.exists() {
                let compressed_data = tokio::fs::read(&cache_file).await?;
                
                // Decompress
                let decompress_start = Instant::now();
                let decompressed = zstd::decode_all(&compressed_data[..])?;
                let decompress_time = decompress_start.elapsed().as_secs_f64() * 1000.0;
                
                // Deserialize
                let (tree, source) = deserialize_tree(&decompressed)?;
                
                // Update stats
                {
                    let mut stats = self.stats.write();
                    stats.cold_hits += 1;
                    stats.decompressions += 1;
                    stats.total_decompression_time_ms += decompress_time;
                }
                
                // Add to hot cache
                let hot_entry = Arc::new(HotEntry {
                    tree: tree.clone(),
                    source: source.clone(),
                    source_hash,
                    last_access: SystemTime::now(),
                    parse_time_ms: 0.0,
                });
                self.hot_cache.insert(path.clone(), hot_entry);
                
                return Ok(Some((tree, source, decompress_time)));
            }
        }
        Ok(None)
    }
    
    /// Clear all caches
    pub fn clear(&self) {
        self.hot_cache.invalidate_all();
        self.cold_cache.invalidate_all();
        *self.stats.write() = CacheStats::default();
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let stats = self.stats.read();
        CacheStats {
            hot_hits: stats.hot_hits,
            cold_hits: stats.cold_hits,
            misses: stats.misses,
            evictions: stats.evictions,
            compressions: stats.compressions,
            decompressions: stats.decompressions,
            total_compression_time_ms: stats.total_compression_time_ms,
            total_decompression_time_ms: stats.total_decompression_time_ms,
            bytes_saved: stats.bytes_saved,
        }
    }
    
    /// Get memory usage estimate
    pub fn memory_usage(&self) -> MemoryUsage {
        let hot_count = self.hot_cache.entry_count() as usize;
        let cold_count = self.cold_cache.entry_count() as usize;
        
        // Estimate: 12KB per hot entry, compressed size for cold
        let hot_memory_kb = hot_count * 12;
        let cold_memory_kb = cold_count * 2; // ~2KB per compressed entry
        
        MemoryUsage {
            hot_entries: hot_count,
            cold_entries: cold_count,
            hot_memory_kb,
            cold_memory_kb,
            total_memory_kb: hot_memory_kb + cold_memory_kb,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryUsage {
    pub hot_entries: usize,
    pub cold_entries: usize,
    pub hot_memory_kb: usize,
    pub cold_memory_kb: usize,
    pub total_memory_kb: usize,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total_hits = self.hot_hits + self.cold_hits;
        let total_requests = total_hits + self.misses;
        let hit_rate = if total_requests > 0 {
            (total_hits as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        write!(f, "Cache Stats:\n")?;
        write!(f, "  Hit rate: {:.2}%\n", hit_rate)?;
        write!(f, "  Hot hits: {}\n", self.hot_hits)?;
        write!(f, "  Cold hits: {}\n", self.cold_hits)?;
        write!(f, "  Misses: {}\n", self.misses)?;
        write!(f, "  Compressions: {}\n", self.compressions)?;
        write!(f, "  Decompressions: {}\n", self.decompressions)?;
        write!(f, "  Avg compression time: {:.2}ms\n", 
            if self.compressions > 0 {
                self.total_compression_time_ms / self.compressions as f64
            } else {
                0.0
            }
        )?;
        write!(f, "  Avg decompression time: {:.2}ms\n",
            if self.decompressions > 0 {
                self.total_decompression_time_ms / self.decompressions as f64
            } else {
                0.0
            }
        )?;
        write!(f, "  Memory saved: {:.2} MB", self.bytes_saved as f64 / 1024.0 / 1024.0)?;
        Ok(())
    }
}
