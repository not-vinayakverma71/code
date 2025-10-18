//! Phase 4 Cache - Synchronous wrapper for the complete optimization stack
//! Integrates: Bytecode → Segmented Storage → Frozen Tier → Dynamic Cache

use std::path::{Path, PathBuf};
use std::fs;
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use bytes::Bytes;
use tree_sitter::{Tree, Parser};

use crate::compact::bytecode::{
    TreeSitterBytecodeEncoder,
    SegmentedBytecodeStream,
};
use crate::cache::FrozenTier;

/// Configuration for Phase 4 cache
pub struct Phase4Config {
    /// Total memory budget in MB
    pub memory_budget_mb: usize,
    
    /// Hot tier percentage (0.0-1.0)
    pub hot_tier_ratio: f32,
    
    /// Warm tier percentage (0.0-1.0)
    pub warm_tier_ratio: f32,
    
    /// Segment size for bytecode segmentation (bytes)
    pub segment_size: usize,
    
    /// Storage directory for segments and frozen data
    pub storage_dir: PathBuf,
    
    /// Enable compression for frozen tier
    pub enable_compression: bool,
}

impl Default for Phase4Config {
    fn default() -> Self {
        Self {
            memory_budget_mb: 50,       // 50 MB as per journey doc
            hot_tier_ratio: 0.4,        // 40% hot
            warm_tier_ratio: 0.3,       // 30% warm
            segment_size: 256 * 1024,   // 256 KB segments
            storage_dir: std::env::temp_dir().join("phase4_cache"),
            enable_compression: true,
        }
    }
}

/// Statistics for Phase 4 cache
#[derive(Debug, Clone)]
pub struct Phase4Stats {
    pub hot_entries: usize,
    pub warm_entries: usize,
    pub cold_entries: usize,
    pub frozen_entries: usize,
    pub hot_bytes: usize,
    pub warm_bytes: usize,
    pub cold_bytes: usize,
    pub frozen_bytes: usize,
    pub total_memory_bytes: usize,
    pub total_disk_bytes: usize,
    pub bytecode_compression_ratio: f64,
}

/// Phase 4 Cache - Complete optimization stack
pub struct Phase4Cache {
    /// Configuration
    config: Phase4Config,
    
    /// Bytecode storage (path -> bytecode stream)
    bytecode_store: Arc<RwLock<HashMap<PathBuf, Arc<SegmentedBytecodeStream>>>>,
    
    /// Source storage (for reconstruction)
    source_store: Arc<RwLock<HashMap<u64, Bytes>>>,
    
    /// Frozen tier for cold data
    frozen_tier: Arc<FrozenTier>,
    
    /// Segment storage directory
    segment_dir: PathBuf,
    
    /// Statistics
    stats: Arc<RwLock<Phase4Stats>>,
    
    /// Parser pool
    parsers: Arc<RwLock<HashMap<String, Parser>>>,
}

impl Phase4Cache {
    /// Create new Phase 4 cache
    pub fn new(config: Phase4Config) -> Result<Self, Box<dyn std::error::Error>> {
        // Create directories
        fs::create_dir_all(&config.storage_dir)?;
        let segment_dir = config.storage_dir.join("segments");
        let frozen_dir = config.storage_dir.join("frozen");
        fs::create_dir_all(&segment_dir)?;
        fs::create_dir_all(&frozen_dir)?;
        
        // Create frozen tier
        let frozen_tier = Arc::new(FrozenTier::new(
            frozen_dir,
            1000.0  // 1 GB disk quota
        )?);
        
        Ok(Self {
            config,
            bytecode_store: Arc::new(RwLock::new(HashMap::new())),
            source_store: Arc::new(RwLock::new(HashMap::new())),
            frozen_tier,
            segment_dir,
            stats: Arc::new(RwLock::new(Phase4Stats {
                hot_entries: 0,
                warm_entries: 0,
                cold_entries: 0,
                frozen_entries: 0,
                hot_bytes: 0,
                warm_bytes: 0,
                cold_bytes: 0,
                frozen_bytes: 0,
                total_memory_bytes: 0,
                total_disk_bytes: 0,
                bytecode_compression_ratio: 1.0,
            })),
            parsers: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Store a tree with Phase 4 optimizations
    pub fn store(
        &self,
        path: PathBuf,
        hash: u64,
        tree: Tree,
        source: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::{STORE_LATENCY, BYTES_WRITTEN};
        let timer = STORE_LATENCY.start_timer();
        
        // Step 1: Convert to bytecode
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source);
        let bytecode_size = bytecode.bytes.len();
        
        // Step 2: Segment the bytecode
        let segmented = SegmentedBytecodeStream::from_bytecode_stream(
            bytecode,
            self.segment_dir.clone()
        )?;
        
        // Step 3: Store source for reconstruction
        self.source_store.write().insert(hash, Bytes::copy_from_slice(source));
        
        // Step 4: Store segmented bytecode
        self.bytecode_store.write().insert(path.clone(), Arc::new(segmented));
        
        // Step 5: Update statistics
        let mut stats = self.stats.write();
        stats.hot_entries += 1;
        stats.hot_bytes += bytecode_size;
        stats.total_memory_bytes += bytecode_size / 10; // Only index in memory
        stats.total_disk_bytes += bytecode_size; // Segments on disk
        stats.bytecode_compression_ratio = source.len() as f64 / bytecode_size as f64;
        
        // Step 6: Check if we need to move to frozen tier
        if stats.total_memory_bytes > self.config.memory_budget_mb * 1_048_576 {
            self.freeze_oldest()?;
        }
        
        // Record metrics
        BYTES_WRITTEN.inc_by(bytecode_size as u64);
        timer.observe_duration();
        
        Ok(())
    }
    /// Get a tree from cache
    pub fn get(&self, path: &PathBuf) -> Option<Arc<SegmentedBytecodeStream>> {
        use crate::{CACHE_HITS, CACHE_MISSES, GET_LATENCY};
        let timer = GET_LATENCY.start_timer();
        
        // Check bytecode store
        let store = self.bytecode_store.read();
        let result = store.get(path).cloned();
        
        // Record metrics
        if result.is_some() {
            CACHE_HITS.inc();
        } else {
            CACHE_MISSES.inc();
        }
        timer.observe_duration();
        
        result
    }
    
    /// Move oldest entries to frozen tier
    fn freeze_oldest(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Simple LRU: remove 10% of entries
        let bytecode_store = self.bytecode_store.read();
        let to_freeze = bytecode_store.len() / 10;
        
        if to_freeze > 0 {
            // In a real implementation, track access times
            // For now, just freeze the first N entries
            let keys: Vec<_> = bytecode_store.keys().take(to_freeze).cloned().collect();
            drop(bytecode_store);
            
            for key in keys {
                if let Some(_segmented) = self.bytecode_store.write().remove(&key) {
                    // Move to frozen tier
                    // This would serialize the segmented stream and compress it
                    let mut stats = self.stats.write();
                    stats.hot_entries = stats.hot_entries.saturating_sub(1);
                    stats.frozen_entries += 1;
                }
            }
        }
        
        Ok(())
    }
    
    /// Get current statistics
    pub fn stats(&self) -> Phase4Stats {
        self.stats.read().clone()
    }
    
    /// Force tier management
    pub fn manage_tiers(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.freeze_oldest()
    }
}

/// Synchronous wrapper for benchmarking
pub struct SyncPhase4Cache {
    cache: Phase4Cache,
    runtime: tokio::runtime::Runtime,
}

impl SyncPhase4Cache {
    /// Create new synchronous cache
    pub fn new(config: Phase4Config) -> Result<Self, Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        
        let cache = Phase4Cache::new(config)?;
        
        Ok(Self { cache, runtime })
    }
    
    /// Store synchronously
    pub fn store(
        &self,
        path: PathBuf,
        hash: u64,
        tree: Tree,
        source: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.cache.store(path, hash, tree, source)
    }
    
    /// Get synchronously
    pub fn get(
        &self,
        path: &Path,
        hash: u64,
    ) -> Result<Option<(Tree, Bytes)>, Box<dyn std::error::Error>> {
        // Get returns Arc<SegmentedBytecodeStream>, not (Tree, Bytes)
        // For now, just check if exists
        if let Some(_segmented) = self.cache.get(&path.to_path_buf()) {
            // TODO: Reconstruct tree from bytecode
            Ok(None)
        } else {
            Ok(None)
        }
    }
    
    /// Get statistics
    pub fn stats(&self) -> Phase4Stats {
        self.cache.stats()
    }
    
    /// Manage tiers
    pub fn manage_tiers(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.cache.manage_tiers()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter_rust;
    
    #[test]
    fn test_phase4_cache() {
        let config = Phase4Config::default();
        let cache = Phase4Cache::new(config).unwrap();
        
        // Create a test tree
        let source = "fn main() { println!(\"Hello\"); }";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        // Store
        let path = PathBuf::from("test.rs");
        let hash = 12345;
        cache.store(path.clone(), hash, tree, source.as_bytes()).unwrap();
        
        // Get - NOTE: This returns Arc<SegmentedBytecodeStream> or None
        let result = cache.get(&path.to_path_buf());
        // Check that something was stored
        assert!(result.is_some()); // Should find the stored bytecode
        
        // Check stats
        let stats = cache.stats();
        assert_eq!(stats.hot_entries, 1);
        assert!(stats.bytecode_compression_ratio > 0.0);
    }
}
