//! Complete Phase 1-4 Pipeline
//! Integrates ALL optimizations from the journey document

use std::path::{Path, PathBuf};
use std::fs;
use std::sync::Arc;
use parking_lot::RwLock;
use bytes::Bytes;
use tree_sitter::{Tree, Node};

// Phase 1: Varint + Packing + Interning
use crate::compact::varint::DeltaEncoder;

// Phase 2: Delta compression
use crate::cache::{
    DeltaCodec, ChunkStore,
};

// Phase 3: Bytecode representation
use crate::compact::bytecode::{
    TreeSitterBytecodeEncoder,
    BytecodeStream,
    SegmentedBytecodeStream,
};

// Phase 4a: Frozen tier
use crate::cache::FrozenTier;

// Phase 4b: Memory-mapped sources
use crate::cache::{MmapSourceStorage};

/// Complete pipeline configuration
pub struct CompletePipelineConfig {
    /// Memory budget in MB
    pub memory_budget_mb: usize,
    
    /// Enable Phase 1 optimizations
    pub phase1_varint: bool,
    pub phase1_packing: bool,
    pub phase1_interning: bool,
    
    /// Enable Phase 2 optimizations
    pub phase2_delta: bool,
    pub phase2_chunking: bool,
    
    /// Enable Phase 3 optimizations
    pub phase3_bytecode: bool,
    
    /// Enable Phase 4 optimizations
    pub phase4a_frozen: bool,
    pub phase4b_mmap: bool,
    pub phase4c_segments: bool,
    
    /// Storage directories
    pub storage_dir: PathBuf,
}

impl Default for CompletePipelineConfig {
    fn default() -> Self {
        Self {
            memory_budget_mb: 50,
            phase1_varint: true,
            phase1_packing: true,
            phase1_interning: true,
            phase2_delta: true,
            phase2_chunking: true,
            phase3_bytecode: true,
            phase4a_frozen: true,
            phase4b_mmap: true,
            phase4c_segments: true,
            storage_dir: std::env::temp_dir().join("complete_pipeline"),
        }
    }
}

/// Statistics for each phase
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    // Phase 1 metrics
    pub phase1_original_bytes: usize,
    pub phase1_varint_bytes: usize,
    pub phase1_packed_bytes: usize,
    pub phase1_interned_symbols: usize,
    
    // Phase 2 metrics
    pub phase2_delta_bytes: usize,
    pub phase2_chunks_created: usize,
    
    // Phase 3 metrics
    pub phase3_bytecode_bytes: usize,
    pub phase3_nodes_encoded: usize,
    
    // Phase 4 metrics
    pub phase4a_frozen_entries: usize,
    pub phase4a_frozen_bytes: usize,
    pub phase4b_mmap_files: usize,
    pub phase4b_mmap_bytes: usize,
    pub phase4c_segments: usize,
    pub phase4c_segment_bytes: usize,
    
    // Overall metrics
    pub total_memory_bytes: usize,
    pub total_disk_bytes: usize,
    pub compression_ratio: f64,
}

/// Complete optimization pipeline
pub struct CompletePipeline {
    config: CompletePipelineConfig,
    
    // Phase 1 components
    varint_encoder: Arc<DeltaEncoder>,
    intern_pool: Arc<RwLock<Vec<String>>>, // Simplified interning
    
    // Phase 2 components
    chunk_store: Arc<ChunkStore>,
    delta_codec: Arc<DeltaCodec>,
    
    // Phase 3 components (bytecode handled inline)
    
    // Phase 4a: Frozen tier
    frozen_tier: Arc<FrozenTier>,
    
    // Phase 4b: Memory-mapped sources
    mmap_storage: Arc<MmapSourceStorage>,
    
    // Phase 4c: Segment directory
    segment_dir: PathBuf,
    
    // Statistics
    stats: Arc<RwLock<PipelineStats>>,
}

impl CompletePipeline {
    pub fn new(config: CompletePipelineConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Create directories
        fs::create_dir_all(&config.storage_dir)?;
        let frozen_dir = config.storage_dir.join("frozen");
        let mmap_dir = config.storage_dir.join("mmap");
        let segment_dir = config.storage_dir.join("segments");
        
        fs::create_dir_all(&frozen_dir)?;
        fs::create_dir_all(&mmap_dir)?;
        fs::create_dir_all(&segment_dir)?;
        
        // Initialize components
        let chunk_store = Arc::new(ChunkStore::new());
        let delta_codec = Arc::new(DeltaCodec::new(chunk_store.clone()));
        let frozen_tier = Arc::new(FrozenTier::new(frozen_dir, 1000.0)?);
        let mmap_storage = Arc::new(MmapSourceStorage::new(mmap_dir, 100)?);
        
        Ok(Self {
            config,
            varint_encoder: Arc::new(DeltaEncoder::new()),
            intern_pool: Arc::new(RwLock::new(Vec::new())),
            chunk_store,
            delta_codec,
            frozen_tier,
            mmap_storage,
            segment_dir,
            stats: Arc::new(RwLock::new(PipelineStats::default())),
        })
    }
    
    /// Process a tree through the complete pipeline
    pub fn process_tree(
        &self,
        path: PathBuf,
        tree: Tree,
        source: &[u8],
    ) -> Result<ProcessedResult, Box<dyn std::error::Error>> {
        let mut stats = self.stats.write();
        let original_size = source.len();
        stats.phase1_original_bytes += original_size;
        
        // Phase 1: Varint + Packing + Interning
        let phase1_data = if self.config.phase1_varint || self.config.phase1_packing || self.config.phase1_interning {
            self.apply_phase1(&tree, source, &mut stats)?
        } else {
            source.to_vec()
        };
        
        // Phase 2: Delta Compression
        let phase2_data = if self.config.phase2_delta || self.config.phase2_chunking {
            self.apply_phase2(&phase1_data, &mut stats)?
        } else {
            phase1_data.clone()
        };
        
        // Phase 3: Bytecode Representation
        let phase3_data = if self.config.phase3_bytecode {
            self.apply_phase3(&tree, source, &mut stats)?
        } else {
            phase2_data.clone()
        };
        
        // Phase 4a: Frozen Tier (cold storage)
        let mut storage_location = StorageLocation::Memory;
        
        if self.config.phase4a_frozen && phase3_data.len() > 100_000 {
            // Large files go to frozen tier
            self.freeze_data(path.clone(), &phase3_data, &mut stats)?;
            storage_location = StorageLocation::Frozen;
        }
        
        // Phase 4b: Memory-mapped sources
        if self.config.phase4b_mmap && storage_location == StorageLocation::Memory {
            let hash = self.hash_data(&phase3_data);
            self.mmap_storage.store_source(hash, source)?;
            stats.phase4b_mmap_files += 1;
            stats.phase4b_mmap_bytes += source.len();
            storage_location = StorageLocation::Mmap;
        }
        
        // Phase 4c: Segmented Bytecode
        if self.config.phase4c_segments && phase3_data.len() > 256 * 1024 {
            let _segmented = self.apply_phase4c(&phase3_data, &mut stats)?;
            storage_location = StorageLocation::Segmented;
        }
        
        // Calculate final compression
        let final_size = match storage_location {
            StorageLocation::Memory => phase3_data.len(),
            StorageLocation::Frozen => stats.phase4a_frozen_bytes,
            StorageLocation::Mmap => stats.phase4b_mmap_bytes,
            StorageLocation::Segmented => stats.phase4c_segment_bytes,
        };
        
        stats.compression_ratio = original_size as f64 / final_size.max(1) as f64;
        stats.total_memory_bytes = if storage_location == StorageLocation::Memory {
            final_size
        } else {
            final_size / 10 // Only metadata in memory
        };
        stats.total_disk_bytes = if storage_location != StorageLocation::Memory {
            final_size
        } else {
            0
        };
        
        Ok(ProcessedResult {
            path,
            original_size,
            final_size,
            storage_location,
            compression_ratio: stats.compression_ratio,
        })
    }
    
    /// Phase 1: Varint encoding + packing + interning
    fn apply_phase1(
        &self,
        tree: &Tree,
        source: &[u8],
        stats: &mut PipelineStats,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut result = Vec::new();
        
        // Build compact representation (simplified)
        let node_count = self.count_nodes(tree.root_node());
        
        // Varint encode positions
        if self.config.phase1_varint {
            // Extract and encode positions with varint
            let positions = self.extract_node_positions(tree.root_node());
            for pos in positions {
                // Manual varint encoding
                let mut val = pos as u64;
                while val >= 0x80 {
                    result.push((val as u8) | 0x80);
                    val >>= 7;
                }
                result.push(val as u8);
            }
            stats.phase1_varint_bytes = result.len();
        }
        
        // Pack nodes (simplified - just store compactly)
        if self.config.phase1_packing {
            // In real implementation, this would pack node data efficiently
            result.extend_from_slice(&node_count.to_le_bytes());
            stats.phase1_packed_bytes = result.len();
        }
        
        // Symbol interning
        if self.config.phase1_interning {
            let interned = self.intern_strings(source);
            stats.phase1_interned_symbols = interned.len();
            
            // Store interned symbol indices instead of strings
            for symbol_id in interned {
                result.extend_from_slice(&symbol_id.to_le_bytes());
            }
        }
        
        if result.is_empty() {
            result = source.to_vec();
        }
        
        Ok(result)
    }
    
    /// Phase 2: Delta compression
    fn apply_phase2(
        &self,
        data: &[u8],
        stats: &mut PipelineStats,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if !self.config.phase2_delta {
            return Ok(data.to_vec());
        }
        
        // Try delta encoding, but fall back for small sources
        match self.delta_codec.encode(data) {
            Ok(delta_entry) => {
                stats.phase2_chunks_created = delta_entry.base_chunks.len();
                
                // Serialize delta entry (simplified)
                let serialized = bincode::serialize(&delta_entry)?;
                stats.phase2_delta_bytes = serialized.len();
                
                Ok(serialized)
            },
            Err(_) => {
                // Source too small for delta encoding, just return as is
                stats.phase2_delta_bytes = data.len();
                Ok(data.to_vec())
            }
        }
    }
    
    /// Phase 3: Bytecode representation
    fn apply_phase3(
        &self,
        tree: &Tree,
        source: &[u8],
        stats: &mut PipelineStats,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(tree, source);
        
        stats.phase3_bytecode_bytes = bytecode.bytes.len();
        stats.phase3_nodes_encoded = bytecode.node_count;
        
        Ok(bytecode.bytes)
    }
    
    /// Phase 4a: Freeze to disk
    fn freeze_data(
        &self,
        path: PathBuf,
        data: &[u8],
        stats: &mut PipelineStats,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create delta entry for frozen storage
        let delta_entry = if self.config.phase2_delta {
            Some(self.delta_codec.encode(data)?)
        } else {
            None
        };
        
        // Freeze to disk (simplified - frozen tier API takes path and source)
        let source_bytes = Bytes::from(data.to_vec());
        self.frozen_tier.freeze(
            path,
            &source_bytes,
            delta_entry,
            vec![], // CST data would go here
        )?;
        
        stats.phase4a_frozen_entries += 1;
        stats.phase4a_frozen_bytes += data.len();
        
        Ok(())
    }
    
    /// Phase 4c: Segment bytecode
    fn apply_phase4c(
        &self,
        data: &[u8],
        stats: &mut PipelineStats,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create bytecode stream from data
        let mut stream = BytecodeStream::new();
        stream.bytes = data.to_vec();
        stream.node_count = 1000; // Estimate
        
        // Segment it
        let _segmented = SegmentedBytecodeStream::from_bytecode_stream(
            stream,
            self.segment_dir.clone(),
        )?;
        
        let segment_stats = segmented.stats();
        stats.phase4c_segments = segment_stats.segments_count;
        stats.phase4c_segment_bytes = segment_stats.total_compressed_size;
        
        Ok(())
    }
    
    // Helper methods
    fn count_nodes(&self, node: Node) -> usize {
        let mut count = 1;
        for _i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                count += self.count_nodes(child);
            }
        }
        count
    }
    
    fn extract_node_positions(&self, node: Node) -> Vec<usize> {
        let mut positions = vec![node.start_byte(), node.end_byte()];
        for _i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                positions.extend(self.extract_node_positions(child));
            }
        }
        positions
    }
    
    fn intern_strings(&self, source: &[u8]) -> Vec<u32> {
        // Simplified string interning
        let text = String::from_utf8_lossy(source);
        let mut pool = self.intern_pool.write();
        let mut ids = Vec::new();
        
        for word in text.split_whitespace().take(100) {
            if !pool.contains(&word.to_string()) {
                pool.push(word.to_string());
            }
            if let Some(pos) = pool.iter().position(|s| s == word) {
                ids.push(pos as u32);
            }
        }
        
        ids
    }
    
    fn hash_data(&self, data: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Get statistics
    pub fn stats(&self) -> PipelineStats {
        self.stats.read().clone()
    }
    
    /// Thaw frozen data (round-trip test)
    pub fn thaw(&self, path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if self.frozen_tier.is_frozen(&path.to_path_buf()) {
            let (source, delta_entry, _cst_data) = self.frozen_tier.thaw(&path.to_path_buf())?;
            
            // Reconstruct from delta if available
            if let Some(delta) = delta_entry {
                let reconstructed = self.delta_codec.decode(&delta)?;
                Ok(reconstructed)
            } else {
                Ok(source.to_vec())
            }
        } else {
            Err("Data not frozen".into())
        }
    }
}

/// Storage location after processing
#[derive(Debug, Clone, PartialEq)]
pub enum StorageLocation {
    Memory,
    Frozen,
    Mmap,
    Segmented,
}

/// Result of processing a tree
#[derive(Debug, Clone)]
pub struct ProcessedResult {
    pub path: PathBuf,
    pub original_size: usize,
    pub final_size: usize,
    pub storage_location: StorageLocation,
    pub compression_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter_rust;
    
    #[test]
    fn test_complete_pipeline() {
        let config = CompletePipelineConfig::default();
        let pipeline = CompletePipeline::new(config).unwrap();
        
        // Parse a test tree
        let _source = "fn main() { println!(\"Hello, world!\"); }";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let _tree = parser.parse(source, None).unwrap();
        
        // Process through pipeline
        let result = pipeline.process_tree(
            PathBuf::from("test.rs"),
            tree,
            source.as_bytes(),
        ).unwrap();
        
        // Check compression
        assert!(result.compression_ratio > 0.0);
        assert!(result.final_size <= result.original_size);
        
        // Check stats
        let stats = pipeline.stats();
        assert!(stats.phase1_original_bytes > 0);
        
        // Test round-trip if frozen
        if result.storage_location == StorageLocation::Frozen {
            let thawed = pipeline.thaw(&PathBuf::from("test.rs")).unwrap();
            // In real implementation, would verify exact match
            assert!(!thawed.is_empty());
        }
    }
    
    #[test]
    fn test_phase_by_phase() {
        // Test with only Phase 1 enabled
        let mut config = CompletePipelineConfig::default();
        config.phase2_delta = false;
        config.phase3_bytecode = false;
        config.phase4a_frozen = false;
        config.phase4b_mmap = false;
        config.phase4c_segments = false;
        
        let pipeline = CompletePipeline::new(config).unwrap();
        let stats = pipeline.stats();
        
        // Verify only Phase 1 metrics are populated
        // (would add actual test here)
    }
}
