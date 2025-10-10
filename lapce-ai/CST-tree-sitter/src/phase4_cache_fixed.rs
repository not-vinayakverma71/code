//! Phase 4 Cache - FIXED VERSION with MultiTierCache integration
//! Now actually retrieves data by re-parsing source

use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use bytes::Bytes;
use tree_sitter::{Tree, Parser};
use std::collections::HashMap;

use crate::multi_tier_cache::{
    MultiTierCache,
    MultiTierConfig,
};
use crate::parser_pool::{ParserPool, FileType};

/// Configuration for Phase 4 cache
#[derive(Clone)]
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
    
    /// Use short timeouts for testing (optional)
    pub test_mode: bool,
}

impl Default for Phase4Config {
    fn default() -> Self {
        Self {
            memory_budget_mb: 50,
            hot_tier_ratio: 0.4,
            warm_tier_ratio: 0.3,
            segment_size: 256 * 1024,
            storage_dir: std::env::temp_dir().join("phase4_cache"),
            enable_compression: true,
            test_mode: false,  // Production mode by default
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
    pub cache_hits: u64,
    pub cache_misses: u64,
}

/// Phase 4 Cache - FIXED with MultiTierCache integration
pub struct Phase4Cache {
    /// Configuration
    config: Phase4Config,
    
    /// Multi-tier cache system (THE FIX!)
    multi_tier: Arc<MultiTierCache>,
    
    /// Parser pool for efficient re-parsing
    parser_pool: Arc<ParserPool>,
    
    /// Parser cache by extension
    parsers: Arc<RwLock<HashMap<String, Parser>>>,
}

impl Phase4Cache {
    /// Create new Phase 4 cache with MultiTierCache
    pub fn new(config: Phase4Config) -> Result<Self, Box<dyn std::error::Error>> {
        // Set timeouts based on mode
        let (warm_timeout, cold_timeout, frozen_timeout) = if config.test_mode {
            // Test mode: short timeouts for quick verification
            (
                std::time::Duration::from_secs(5),    // 5 seconds
                std::time::Duration::from_secs(10),   // 10 seconds
                std::time::Duration::from_secs(15),   // 15 seconds
            )
        } else {
            // Production mode: realistic timeouts
            (
                std::time::Duration::from_secs(300),   // 5 minutes
                std::time::Duration::from_secs(900),   // 15 minutes
                std::time::Duration::from_secs(3600),  // 1 hour
            )
        };
        
        // Convert Phase4Config to MultiTierConfig
        let multi_tier_config = MultiTierConfig {
            hot_tier_mb: (config.memory_budget_mb as f32 * config.hot_tier_ratio) as usize,
            warm_tier_mb: (config.memory_budget_mb as f32 * config.warm_tier_ratio) as usize,
            cold_tier_mb: (config.memory_budget_mb as f32 * (1.0 - config.hot_tier_ratio - config.warm_tier_ratio)) as usize,
            storage_dir: config.storage_dir.clone(),
            enable_compression: config.enable_compression,
            demote_to_warm_timeout: warm_timeout,
            demote_to_cold_timeout: cold_timeout,
            demote_to_frozen_timeout: frozen_timeout,
            ..Default::default()
        };
        
        // Create multi-tier cache
        let multi_tier = Arc::new(MultiTierCache::new(multi_tier_config)?);
        
        // Create parser pool
        let parser_pool = Arc::new(ParserPool::new(5));
        
        Ok(Self {
            config,
            multi_tier,
            parser_pool,
            parsers: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Store a tree in the cache  
    pub fn store(
        &self,
        path: PathBuf,
        hash: u64,
        tree: Tree,
        source: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::{STORE_LATENCY, LOGGER, CacheLogEvent};
        use slog::info;
        use std::time::Instant;
        
        let start = Instant::now();
        let path_str = path.display().to_string();
        
        // Use multi-tier cache store method
        let result = self.multi_tier.store(path, hash, tree, source);
        
        let duration = start.elapsed();
        STORE_LATENCY.observe(duration.as_secs_f64());
        
        // Structured logging
        let log_event = CacheLogEvent {
            timestamp: format!("{:?}", std::time::SystemTime::now()),
            operation: "store".to_string(),
            tier: "multi_tier".to_string(),
            key: path_str.clone(),
            latency_ms: duration.as_secs_f64() * 1000.0,
            success: result.is_ok(),
            error: result.as_ref().err().map(|e| e.to_string()),
        };
        
        info!(LOGGER, "cache_operation"; 
            "event" => serde_json::to_string(&log_event).unwrap_or_default());
        
        result
    }
    
    /// Get a tree from cache - THE FIX!
    pub fn get(
        &self,
        path: &Path,
        hash: u64,
    ) -> Result<Option<(Tree, Bytes)>, Box<dyn std::error::Error>> {
        use crate::{GET_LATENCY, CACHE_HITS, CACHE_MISSES, LOGGER, CacheLogEvent};
        use slog::info;
        use std::time::Instant;
        
        let start = Instant::now();
        let path_str = path.display().to_string();
        
        // Get from multi-tier cache
        let result = if let Some((bytecode_stream, source)) = self.multi_tier.get(path)? {
            // THE FIX: Re-parse the source to get Tree
            // This is the ONLY way with tree-sitter
            
            // Determine file type from extension
            let file_type = Self::get_file_type(path);
            
            // Get or create parser for this file type
            let mut parser = self.get_or_create_parser(file_type)?;
            
            // Re-parse the source to get Tree
            if let Some(tree) = parser.parse(&source, None) {
                // Optional: Validate against bytecode for integrity
                // let decoder = TreeSitterBytecodeDecoder::new(bytecode_stream, source.to_vec());
                // decoder.verify()?;
                
                CACHE_HITS.inc();
                Some((tree, source))
            } else {
                return Err("Failed to re-parse source".into());
            }
        } else {
            CACHE_MISSES.inc();
            None
        };
        
        let duration = start.elapsed();
        GET_LATENCY.observe(duration.as_secs_f64());
        
        // Structured logging
        let log_event = CacheLogEvent {
            timestamp: format!("{:?}", std::time::SystemTime::now()),
            operation: "get".to_string(),
            tier: "multi_tier".to_string(),
            key: path_str,
            latency_ms: duration.as_secs_f64() * 1000.0,
            success: result.is_some(),
            error: None,
        };
        
        info!(LOGGER, "cache_operation";
            "event" => serde_json::to_string(&log_event).unwrap_or_default());
        
        Ok(result)
    }
    
    /// Determine file type from path
    fn get_file_type(path: &Path) -> FileType {
        match path.extension().and_then(|s| s.to_str()) {
            Some("rs") => FileType::Rust,
            Some("js") => FileType::JavaScript,
            Some("ts") | Some("tsx") => FileType::TypeScript,
            Some("py") => FileType::Python,
            Some("go") => FileType::Go,
            Some("java") => FileType::Java,
            Some("cpp") | Some("cc") | Some("cxx") | Some("c") | Some("h") | Some("hpp") => FileType::Cpp,
            _ => FileType::Other,
        }
    }
    
    /// Get or create parser for file type
    fn get_or_create_parser(&self, file_type: FileType) -> Result<Parser, Box<dyn std::error::Error>> {
        let mut parser = Parser::new();
        
        match file_type {
            FileType::Rust => parser.set_language(&tree_sitter_rust::LANGUAGE.into())?,
            #[cfg(feature = "lang-javascript")]
            FileType::JavaScript => parser.set_language(&tree_sitter_javascript::language())?,
            #[cfg(feature = "lang-typescript")]
            FileType::TypeScript => parser.set_language(&tree_sitter_typescript::language_typescript())?,
            #[cfg(not(feature = "lang-javascript"))]
            FileType::JavaScript => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Unsupported, "JavaScript support not compiled in"))),
            #[cfg(not(feature = "lang-typescript"))]
            FileType::TypeScript => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Unsupported, "TypeScript support not compiled in"))),
            FileType::Python => parser.set_language(&tree_sitter_python::LANGUAGE.into())?,
            FileType::Go => parser.set_language(&tree_sitter_go::LANGUAGE.into())?,
            FileType::Java => parser.set_language(&tree_sitter_java::LANGUAGE.into())?,
            FileType::Cpp => parser.set_language(&tree_sitter_cpp::LANGUAGE.into())?,
            FileType::Other => return Err("Unsupported file type".into()),
        }
        
        Ok(parser)
    }
    
    /// Get storage directory path
    pub fn storage_dir(&self) -> &std::path::Path {
        &self.config.storage_dir
    }
    
    /// Get current statistics
    pub fn stats(&self) -> Phase4Stats {
        let multi_stats = self.multi_tier.stats();
        
        Phase4Stats {
            hot_entries: multi_stats.hot_entries,
            warm_entries: multi_stats.warm_entries,
            cold_entries: multi_stats.cold_entries,
            frozen_entries: multi_stats.frozen_entries,
            hot_bytes: multi_stats.hot_bytes,
            warm_bytes: multi_stats.warm_bytes,
            cold_bytes: multi_stats.cold_bytes,
            frozen_bytes: multi_stats.frozen_bytes,
            total_memory_bytes: multi_stats.hot_bytes + multi_stats.warm_bytes + multi_stats.cold_bytes,
            total_disk_bytes: multi_stats.frozen_bytes,
            bytecode_compression_ratio: 1.0, // Would need to track
            cache_hits: multi_stats.total_hits,
            cache_misses: multi_stats.total_misses,
        }
    }
    
    /// Force tier management
    pub fn manage_tiers(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.multi_tier.manage_tiers()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_phase4_cache_fixed() {
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
        
        // Get - THIS SHOULD NOW WORK!
        let result = cache.get(&path, hash).unwrap();
        assert!(result.is_some()); // FIXED!
        
        let (retrieved_tree, retrieved_source) = result.unwrap();
        assert_eq!(retrieved_source.len(), source.len());
        assert_eq!(retrieved_tree.root_node().kind(), "source_file");
        
        // Check stats
        let stats = cache.stats();
        assert_eq!(stats.hot_entries, 1);
        assert_eq!(stats.cache_hits, 1);
    }
}
