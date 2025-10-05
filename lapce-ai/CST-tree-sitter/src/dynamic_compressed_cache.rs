//! Dynamic frequency-based compressed cache with adaptive hot/cold management

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use parking_lot::RwLock;
use moka::sync::Cache;
use bytes::Bytes;
use tree_sitter::Tree;
use std::time::{SystemTime, Instant};
use std::collections::HashMap;
use dashmap::DashMap;

use crate::cst_codec::{serialize_source_only, deserialize_source_only};
use crate::cache::{DeltaCodec, DeltaEntry, ChunkStore};

/// Dynamic cache that automatically manages hot/cold tiers based on access frequency
pub struct DynamicCompressedCache {
    /// Hot tier: frequently accessed files (uncompressed)
    hot_cache: Cache<PathBuf, Arc<HotEntry>>,
    
    /// Warm tier: moderately accessed files (light compression)
    warm_cache: Cache<PathBuf, Arc<WarmEntry>>,
    
    /// Cold tier: rarely accessed files (heavy compression)
    cold_cache: Cache<PathBuf, Arc<ColdEntry>>,
    
    /// Shared source store: deduplicates source storage across all hot entries
    /// Key: source_hash, Value: source bytes
    source_store: Arc<DashMap<u64, Arc<Bytes>>>,
    
    /// Delta codec for source compression
    delta_codec: Arc<DeltaCodec>,
    
    /// Chunk store for deduplication
    chunk_store: Arc<ChunkStore>,
    
    /// Access frequency tracking
    access_tracker: Arc<RwLock<AccessTracker>>,
    
    /// Statistics
    stats: Arc<CacheStats>,
    
    /// Configuration
    config: DynamicCacheConfig,
}

#[derive(Clone)]
pub struct HotEntry {
    pub tree: Tree,
    pub source_hash: u64,  // Now only stores the hash, source is in shared store
    pub parse_time_ms: f64,
    access_count: Arc<AtomicU64>,
    last_access: Arc<RwLock<SystemTime>>,
}

#[derive(Clone)]
pub struct WarmEntry {
    pub delta_entry: Option<DeltaEntry>,  // Delta-encoded source (if beneficial)
    pub compressed_tree_source: Vec<u8>,  // Fallback: Light compression (LZ4)
    pub source_hash: u64,
    pub original_size: usize,
    access_count: Arc<AtomicU64>,
    last_access: Arc<RwLock<SystemTime>>,
}

#[derive(Clone)]
pub struct ColdEntry {
    pub delta_entry: Option<DeltaEntry>,  // Delta-encoded source (if beneficial)
    pub compressed_data: Vec<u8>,  // Fallback: Heavy compression (ZSTD)
    pub source_hash: u64,
    pub original_size: usize,
    access_count: Arc<AtomicU64>,
}

pub struct DynamicCacheConfig {
    /// Maximum total memory (MB)
    pub max_memory_mb: usize,
    
    /// Percentage of memory for hot tier
    pub hot_tier_percent: f32,
    
    /// Percentage of memory for warm tier
    pub warm_tier_percent: f32,
    
    /// Access threshold for hot promotion
    pub hot_threshold: u64,
    
    /// Access threshold for warm promotion
    pub warm_threshold: u64,
    
    /// Time-based decay factor (accesses decay over time)
    pub decay_interval_secs: u64,
    
    /// Enable adaptive sizing (auto-adjust tier sizes based on workload)
    pub adaptive_sizing: bool,
    
    /// ZSTD compression level for cold tier (1-22)
    pub cold_compression_level: i32,
}

impl Default for DynamicCacheConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 500,  // 500 MB default
            hot_tier_percent: 0.4,  // 40% hot
            warm_tier_percent: 0.3,  // 30% warm
            // 30% cold (implicit)
            hot_threshold: 5,  // 5+ accesses = hot
            warm_threshold: 2,  // 2+ accesses = warm
            decay_interval_secs: 300,  // 5 minutes
            adaptive_sizing: true,
            cold_compression_level: 3,
        }
    }
}

struct AccessTracker {
    access_history: HashMap<PathBuf, AccessInfo>,
    total_accesses: u64,
    last_cleanup: Instant,
}

struct AccessInfo {
    count: u64,
    last_access: SystemTime,
    tier: CacheTier,
}

#[derive(Clone, Copy, PartialEq)]
enum CacheTier {
    Hot,
    Warm,
    Cold,
    None,
}

pub struct CacheStats {
    pub hot_hits: AtomicU64,
    pub warm_hits: AtomicU64,
    pub cold_hits: AtomicU64,
    pub misses: AtomicU64,
    pub promotions: AtomicU64,
    pub demotions: AtomicU64,
    pub evictions: AtomicU64,
    pub total_compression_time_ms: AtomicU64,
    pub total_decompression_time_ms: AtomicU64,
    pub memory_used_mb: AtomicUsize,
}

impl DynamicCompressedCache {
    pub fn new(config: DynamicCacheConfig) -> Self {
        // Calculate tier sizes based on percentages
        let total_mb = config.max_memory_mb as f32;
        let hot_mb = (total_mb * config.hot_tier_percent) as usize;
        let warm_mb = (total_mb * config.warm_tier_percent) as usize;
        let cold_mb = total_mb as usize - hot_mb - warm_mb;
        
        // Estimate capacity based on average file sizes
        // Hot: ~12KB per file, Warm: ~4KB per file, Cold: ~1KB per file
        let hot_capacity = (hot_mb * 1024 / 12).max(100);
        let warm_capacity = (warm_mb * 1024 / 4).max(200);
        let cold_capacity = (cold_mb * 1024).max(500);
        
        let hot_cache = Cache::builder()
            .max_capacity(hot_capacity as u64)
            .time_to_idle(std::time::Duration::from_secs(config.decay_interval_secs))
            .build();
        
        let warm_cache = Cache::builder()
            .max_capacity(warm_capacity as u64)
            .time_to_idle(std::time::Duration::from_secs(config.decay_interval_secs * 2))
            .build();
        
        let cold_cache = Cache::builder()
            .max_capacity(cold_capacity as u64)
            .build();
        
        let chunk_store = Arc::new(ChunkStore::new());
        let delta_codec = Arc::new(DeltaCodec::new(chunk_store.clone()));
        
        Self {
            hot_cache,
            warm_cache,
            cold_cache,
            source_store: Arc::new(DashMap::new()),
            delta_codec,
            chunk_store,
            access_tracker: Arc::new(RwLock::new(AccessTracker {
                access_history: HashMap::new(),
                total_accesses: 0,
                last_cleanup: Instant::now(),
            })),
            stats: Arc::new(CacheStats {
                hot_hits: AtomicU64::new(0),
                warm_hits: AtomicU64::new(0),
                cold_hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
                promotions: AtomicU64::new(0),
                demotions: AtomicU64::new(0),
                evictions: AtomicU64::new(0),
                total_compression_time_ms: AtomicU64::new(0),
                total_decompression_time_ms: AtomicU64::new(0),
                memory_used_mb: AtomicUsize::new(0),
            }),
            config,
        }
    }
    
    /// Get or parse with automatic tier management
    pub async fn get_or_parse<F>(
        &self,
        path: &Path,
        source_hash: u64,
        parse_fn: F,
    ) -> Result<(Tree, Bytes), Box<dyn std::error::Error>>
    where
        F: FnOnce() -> Result<(Tree, Bytes, f64), Box<dyn std::error::Error>>,
    {
        let path_buf = path.to_path_buf();
        
        // Track access
        self.track_access(&path_buf);
        
        // Check hot cache first
        if let Some(entry) = self.hot_cache.get(&path_buf) {
            if entry.source_hash == source_hash {
                entry.access_count.fetch_add(1, Ordering::Relaxed);
                *entry.last_access.write() = SystemTime::now();
                self.stats.hot_hits.fetch_add(1, Ordering::Relaxed);
                
                // Retrieve source from shared store
                if let Some(source) = self.source_store.get(&entry.source_hash) {
                    return Ok((entry.tree.clone(), (**source).clone()));
                }
                // If source not found (shouldn't happen), continue to miss path
            }
        }
        
        // Check warm cache
        if let Some(entry) = self.warm_cache.get(&path_buf) {
            if entry.source_hash == source_hash {
                entry.access_count.fetch_add(1, Ordering::Relaxed);
                *entry.last_access.write() = SystemTime::now();
                self.stats.warm_hits.fetch_add(1, Ordering::Relaxed);
                
                let start = Instant::now();
                
                // Try delta decode first, fall back to LZ4
                let source = if let Some(delta_entry) = &entry.delta_entry {
                    // Reconstruct from delta
                    match self.delta_codec.decode(delta_entry) {
                        Ok(data) => {
                            let source = Bytes::from(data);
                            source
                        }
                        Err(e) => {
                            eprintln!("Delta decode failed, falling back to LZ4: {}", e);
                            // Fall back to LZ4
                            let decompressed = lz4::block::decompress(&entry.compressed_tree_source, Some(entry.original_size as i32))
                                .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error>)?;
                            deserialize_source_only(&decompressed)
                                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
                        }
                    }
                } else {
                    // Use LZ4 decompression
                    let decompressed = lz4::block::decompress(&entry.compressed_tree_source, Some(entry.original_size as i32))
                        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error>)?;
                    deserialize_source_only(&decompressed)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
                };
                
                let decompress_ms = start.elapsed().as_millis() as u64;
                self.stats.total_decompression_time_ms.fetch_add(decompress_ms, Ordering::Relaxed);
                
                // Reparse (tree-sitter limitation)
                let (tree, _, _) = parse_fn()?;
                
                // Check if should promote to hot
                if entry.access_count.load(Ordering::Relaxed) >= self.config.hot_threshold {
                    self.promote_to_hot(path_buf.clone(), tree.clone(), source.clone(), source_hash).await;
                }
                
                return Ok((tree, source));
            }
        }
        
        // Check cold cache
        if let Some(entry) = self.cold_cache.get(&path_buf) {
            if entry.source_hash == source_hash {
                entry.access_count.fetch_add(1, Ordering::Relaxed);
                self.stats.cold_hits.fetch_add(1, Ordering::Relaxed);
                
                // Decompress (ZSTD)
                let start = Instant::now();
                let decompressed = zstd::decode_all(&entry.compressed_data[..])
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                let decompress_ms = start.elapsed().as_millis() as u64;
                self.stats.total_decompression_time_ms.fetch_add(decompress_ms, Ordering::Relaxed);
                
                // Deserialize and reparse
                let source = deserialize_source_only(&decompressed)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                let (tree, _, _) = parse_fn()?;
                
                // Check if should promote
                let access_count = entry.access_count.load(Ordering::Relaxed);
                if access_count >= self.config.hot_threshold {
                    self.promote_to_hot(path_buf.clone(), tree.clone(), source.clone(), source_hash).await;
                } else if access_count >= self.config.warm_threshold {
                    self.promote_to_warm(path_buf.clone(), source.clone(), source_hash).await;
                }
                
                return Ok((tree, source));
            }
        }
        
        // Cache miss - parse and insert
        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        let (tree, source, parse_time_ms) = parse_fn()?;
        
        // Insert into appropriate tier based on initial heuristics
        self.insert_new(path_buf, tree.clone(), source.clone(), source_hash, parse_time_ms).await;
        
        Ok((tree, source))
    }
    
    /// Track access for frequency analysis
    fn track_access(&self, path: &PathBuf) {
        let mut tracker = self.access_tracker.write();
        
        // Clean up old entries periodically
        if tracker.last_cleanup.elapsed() > std::time::Duration::from_secs(self.config.decay_interval_secs) {
            self.cleanup_old_accesses(&mut tracker);
            tracker.last_cleanup = Instant::now();
        }
        
        // Update access info
        let info = tracker.access_history.entry(path.clone()).or_insert(AccessInfo {
            count: 0,
            last_access: SystemTime::now(),
            tier: CacheTier::None,
        });
        
        info.count += 1;
        info.last_access = SystemTime::now();
        tracker.total_accesses += 1;
    }
    
    /// Clean up old access records
    fn cleanup_old_accesses(&self, tracker: &mut AccessTracker) {
        let cutoff = SystemTime::now() - std::time::Duration::from_secs(self.config.decay_interval_secs * 10);
        tracker.access_history.retain(|_, info| info.last_access > cutoff);
    }
    
    /// Insert new entry into appropriate tier
    async fn insert_new(&self, path: PathBuf, tree: Tree, source: Bytes, hash: u64, parse_time: f64) {
        // Start in cold tier for new files
        // They'll be promoted if accessed frequently
        let serialized = serialize_source_only(&source);
        
        let start = Instant::now();
        let compressed = zstd::encode_all(&serialized[..], self.config.cold_compression_level).unwrap_or(serialized);
        let compress_ms = start.elapsed().as_millis() as u64;
        self.stats.total_compression_time_ms.fetch_add(compress_ms, Ordering::Relaxed);
        
        // Try delta encoding
        let delta_entry = match self.delta_codec.encode(&source) {
            Ok(delta) => Some(delta),
            Err(_) => None,
        };
        
        let entry = Arc::new(ColdEntry {
            delta_entry,
            compressed_data: compressed,
            source_hash: hash,
            original_size: source.len(),
            access_count: Arc::new(AtomicU64::new(1)),
        });
        
        self.cold_cache.insert(path, entry);
    }
    
    /// Promote entry to hot tier
    async fn promote_to_hot(&self, path: PathBuf, tree: Tree, source: Bytes, hash: u64) {
        self.stats.promotions.fetch_add(1, Ordering::Relaxed);
        
        // Remove from other tiers
        self.warm_cache.remove(&path);
        self.cold_cache.remove(&path);
        
        // Store source in shared store if not already present
        self.source_store.entry(hash).or_insert(Arc::new(source));
        
        // Insert into hot with only hash reference
        let entry = Arc::new(HotEntry {
            tree,
            source_hash: hash,
            parse_time_ms: 0.0,
            access_count: Arc::new(AtomicU64::new(1)),
            last_access: Arc::new(RwLock::new(SystemTime::now())),
        });
        
        self.hot_cache.insert(path, entry);
    }
    
    /// Promote entry to warm tier
    async fn promote_to_warm(&self, path: PathBuf, source: Bytes, hash: u64) {
        self.stats.promotions.fetch_add(1, Ordering::Relaxed);
        
        // Remove from cold
        self.cold_cache.remove(&path);
        
        let serialized = serialize_source_only(&source);
        let start = Instant::now();
        
        // Try delta encoding first
        let delta_entry = match self.delta_codec.encode(&source) {
            Ok(delta) => Some(delta),
            Err(_) => None,
        };
        
        // Always keep LZ4 as fallback
        let compressed = lz4::block::compress(&serialized, None, true)
            .unwrap_or(serialized.clone());
        
        let compress_ms = start.elapsed().as_millis() as u64;
        self.stats.total_compression_time_ms.fetch_add(compress_ms, Ordering::Relaxed);
        
        let entry = Arc::new(WarmEntry {
            delta_entry,
            compressed_tree_source: compressed,
            source_hash: hash,
            original_size: serialized.len(),
            access_count: Arc::new(AtomicU64::new(1)),
            last_access: Arc::new(RwLock::new(SystemTime::now())),
        });
        
        self.warm_cache.insert(path, entry);
    }
    
    /// Get current statistics
    pub fn stats(&self) -> CacheStatsSnapshot {
        let hot_count = self.hot_cache.entry_count();
        let warm_count = self.warm_cache.entry_count();
        let cold_count = self.cold_cache.entry_count();
        
        // Estimate memory usage
        let hot_memory_mb = (hot_count * 12 / 1024) as usize;  // ~12KB per hot entry
        let warm_memory_mb = (warm_count * 4 / 1024) as usize;  // ~4KB per warm entry
        let cold_memory_mb = (cold_count / 1024) as usize;      // ~1KB per cold entry
        let total_memory_mb = hot_memory_mb + warm_memory_mb + cold_memory_mb;
        
        self.stats.memory_used_mb.store(total_memory_mb, Ordering::Relaxed);
        
        let total_hits = self.stats.hot_hits.load(Ordering::Relaxed) +
                        self.stats.warm_hits.load(Ordering::Relaxed) +
                        self.stats.cold_hits.load(Ordering::Relaxed);
        let total_requests = total_hits + self.stats.misses.load(Ordering::Relaxed);
        
        CacheStatsSnapshot {
            hot_entries: hot_count as usize,
            warm_entries: warm_count as usize,
            cold_entries: cold_count as usize,
            hot_memory_mb,
            warm_memory_mb,
            cold_memory_mb,
            total_memory_mb,
            hot_hits: self.stats.hot_hits.load(Ordering::Relaxed),
            warm_hits: self.stats.warm_hits.load(Ordering::Relaxed),
            cold_hits: self.stats.cold_hits.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            hit_rate: if total_requests > 0 { 
                (total_hits as f64 / total_requests as f64) * 100.0 
            } else { 
                0.0 
            },
            promotions: self.stats.promotions.load(Ordering::Relaxed),
            demotions: self.stats.demotions.load(Ordering::Relaxed),
            avg_compression_ms: if self.stats.total_compression_time_ms.load(Ordering::Relaxed) > 0 {
                self.stats.total_compression_time_ms.load(Ordering::Relaxed) as f64 / 
                (self.stats.cold_hits.load(Ordering::Relaxed) + self.stats.misses.load(Ordering::Relaxed)) as f64
            } else { 0.0 },
            avg_decompression_ms: if self.stats.total_decompression_time_ms.load(Ordering::Relaxed) > 0 {
                self.stats.total_decompression_time_ms.load(Ordering::Relaxed) as f64 / 
                (self.stats.warm_hits.load(Ordering::Relaxed) + self.stats.cold_hits.load(Ordering::Relaxed)) as f64
            } else { 0.0 },
        }
    }
    
    /// Clear all caches
    pub fn clear(&self) {
        self.hot_cache.invalidate_all();
        self.warm_cache.invalidate_all();
        self.cold_cache.invalidate_all();
        self.source_store.clear();
        self.access_tracker.write().access_history.clear();
    }
    
    /// Adaptive resize based on workload
    pub fn adapt_to_workload(&self) {
        if !self.config.adaptive_sizing {
            return;
        }
        
        let stats = self.stats();
        let hot_ratio = stats.hot_hits as f64 / (stats.hot_hits + stats.warm_hits + stats.cold_hits + 1) as f64;
        
        // If hot cache is getting most hits, increase its size
        if hot_ratio > 0.7 && stats.hot_memory_mb < self.config.max_memory_mb / 2 {
            // Would resize caches here in production
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStatsSnapshot {
    pub hot_entries: usize,
    pub warm_entries: usize,
    pub cold_entries: usize,
    pub hot_memory_mb: usize,
    pub warm_memory_mb: usize,
    pub cold_memory_mb: usize,
    pub total_memory_mb: usize,
    pub hot_hits: u64,
    pub warm_hits: u64,
    pub cold_hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub promotions: u64,
    pub demotions: u64,
    pub avg_compression_ms: f64,
    pub avg_decompression_ms: f64,
}

impl std::fmt::Display for CacheStatsSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Dynamic Cache Statistics ===")?;
        writeln!(f, "Entries:")?;
        writeln!(f, "  Hot:  {} files ({} MB)", self.hot_entries, self.hot_memory_mb)?;
        writeln!(f, "  Warm: {} files ({} MB)", self.warm_entries, self.warm_memory_mb)?;
        writeln!(f, "  Cold: {} files ({} MB)", self.cold_entries, self.cold_memory_mb)?;
        writeln!(f, "  Total: {} files ({} MB)", 
            self.hot_entries + self.warm_entries + self.cold_entries,
            self.total_memory_mb)?;
        writeln!(f)?;
        writeln!(f, "Performance:")?;
        writeln!(f, "  Hit rate: {:.1}%", self.hit_rate)?;
        writeln!(f, "  Hot hits: {}", self.hot_hits)?;
        writeln!(f, "  Warm hits: {}", self.warm_hits)?;
        writeln!(f, "  Cold hits: {}", self.cold_hits)?;
        writeln!(f, "  Misses: {}", self.misses)?;
        writeln!(f, "  Promotions: {}", self.promotions)?;
        writeln!(f)?;
        writeln!(f, "Timing:")?;
        writeln!(f, "  Avg compression: {:.2}ms", self.avg_compression_ms)?;
        writeln!(f, "  Avg decompression: {:.2}ms", self.avg_decompression_ms)?;
        Ok(())
    }
}
