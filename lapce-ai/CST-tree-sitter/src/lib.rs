//! CST-tree-sitter: Complete 6-Phase Optimization Pipeline
//! 
//! Implements all optimization phases from the journey document:
//! - Phase 1: Varint + Packing + Interning (40% reduction)
//! - Phase 2: Delta Compression (60% cumulative)
//! - Phase 3: Bytecode Trees (75% cumulative)
//! - Phase 4a: Frozen Tier (93% cumulative)
//! - Phase 4b: Memory-Mapped Sources (95% cumulative)

pub mod ast;
pub mod symbols;
pub mod incremental;
pub mod cst_api;
pub mod native_parser_manager;

use lazy_static::lazy_static;
use prometheus::{Histogram, HistogramOpts, IntCounter, IntGauge, register_histogram, register_int_counter, register_int_gauge};
use serde::{Deserialize, Serialize};
use slog::{o, Drain, Logger};

// Initialize structured logging
lazy_static! {
    pub static ref LOGGER: Logger = {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        slog::Logger::root(drain, o!("module" => "cst-tree-sitter"))
    };
    
    // Prometheus metrics for cache operations
    pub static ref CACHE_HITS: IntCounter = register_int_counter!(
        "cst_cache_hits_total",
        "Total number of cache hits"
    ).unwrap();
    
    pub static ref CACHE_MISSES: IntCounter = register_int_counter!(
        "cst_cache_misses_total", 
        "Total number of cache misses"
    ).unwrap();
    
    pub static ref CACHE_PROMOTIONS: IntCounter = register_int_counter!(
        "cst_cache_promotions_total",
        "Total number of tier promotions"
    ).unwrap();
    
    pub static ref CACHE_DEMOTIONS: IntCounter = register_int_counter!(
        "cst_cache_demotions_total",
        "Total number of tier demotions"
    ).unwrap();
    
    pub static ref GET_LATENCY: Histogram = register_histogram!(
        HistogramOpts::new("cst_cache_get_duration_seconds", "Cache get operation duration")
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0])
    ).unwrap();
    
    pub static ref STORE_LATENCY: Histogram = register_histogram!(
        HistogramOpts::new("cst_cache_store_duration_seconds", "Cache store operation duration")
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0])
    ).unwrap();
    
    pub static ref MEMORY_USAGE: IntGauge = register_int_gauge!(
        "cst_cache_memory_bytes",
        "Current memory usage in bytes"
    ).unwrap();
    
    pub static ref DISK_USAGE: IntGauge = register_int_gauge!(
        "cst_cache_disk_bytes",
        "Current disk usage in bytes"
    ).unwrap();
    
    // Parsing metrics
    pub static ref PARSE_DURATION: Histogram = register_histogram!(
        HistogramOpts::new("cst_parse_duration_seconds", "Tree-sitter parse duration")
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0])
    ).unwrap();
    
    pub static ref VERIFY_DURATION: Histogram = register_histogram!(
        HistogramOpts::new("cst_verify_duration_seconds", "Bytecode verification duration")
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1])
    ).unwrap();
    
    pub static ref ENCODE_DURATION: Histogram = register_histogram!(
        HistogramOpts::new("cst_encode_duration_seconds", "Bytecode encoding duration")
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0])
    ).unwrap();
    
    pub static ref DECODE_DURATION: Histogram = register_histogram!(
        HistogramOpts::new("cst_decode_duration_seconds", "Bytecode decoding duration")
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.25])
    ).unwrap();
    
    pub static ref NODES_ENCODED: IntCounter = register_int_counter!(
        "cst_nodes_encoded_total",
        "Total number of nodes encoded"
    ).unwrap();
    
    pub static ref NODES_DECODED: IntCounter = register_int_counter!(
        "cst_nodes_decoded_total",
        "Total number of nodes decoded"
    ).unwrap();
    
    pub static ref SEGMENT_LOADS: IntCounter = register_int_counter!(
        "cst_segment_loads_total",
        "Total number of segment loads from disk"
    ).unwrap();
    
    pub static ref BYTES_WRITTEN: IntCounter = register_int_counter!(
        "cst_bytes_written_total",
        "Total bytes written to cache"
    ).unwrap();
    
    pub static ref BYTES_READ: IntCounter = register_int_counter!(
        "cst_bytes_read_total",
        "Total bytes read from cache"
    ).unwrap();
}

/// Structured log event for cache operations
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheLogEvent {
    pub timestamp: String,
    pub operation: String,
    pub tier: String,
    pub key: String,
    pub latency_ms: f64,
    pub success: bool,
    pub error: Option<String>,
}

/// Performance metrics for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub throughput: f64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub memory_mb: f64,
    pub cpu_percent: f64,
}

// Core modules
pub mod compact;
pub mod cache;
pub mod language;
pub mod parser_pool;
pub mod multi_tier_cache;
pub mod logging;
pub mod complete_pipeline;
pub mod cst_codec;
pub mod dynamic_compressed_cache;
pub mod phase4_cache;
pub mod phase4_cache_fixed;
pub mod performance_config;

// IPC and monitoring modules
pub mod ipc {
    pub mod health_server;
}

// Re-export main pipeline components
pub use complete_pipeline::{
    CompletePipeline,
    CompletePipelineConfig,
    PipelineStats,
    ProcessedResult,
    StorageLocation,
};

// Export the fixed implementation as the primary one
pub use phase4_cache_fixed::{
    Phase4Cache,
    Phase4Config,
    Phase4Stats,
};

// Keep old one available but deprecated
#[deprecated(note = "Use phase4_cache_fixed::Phase4Cache instead")]
pub use phase4_cache as phase4_cache_old;

pub use multi_tier_cache::{
    MultiTierCache,
    MultiTierConfig,
    MultiTierStats,
};

// Re-export performance configuration
pub use performance_config::{
    PerformanceConfig,
    PerformanceSLO,
    SLOValidation,
    LanguageTuning,
    CompressionPolicy,
    AccessPattern,
    AutoTuneConfig,
    AutoTuner,
};

// Re-export bytecode components
pub use compact::bytecode::{
    TreeSitterBytecodeEncoder,
    TreeSitterBytecodeDecoder,
    BytecodeStream,
    SegmentedBytecodeStream,
    Opcode,
};

use tree_sitter::{Parser, Tree};
use std::path::Path;
use std::collections::HashMap;

/// Legacy simple integration (kept for compatibility)
pub struct TreeSitterIntegration {
    parsers: HashMap<String, Parser>,
}

impl TreeSitterIntegration {
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
        }
    }
    
    pub fn parse_rust(&mut self, code: &str) -> Option<Tree> {
        self.parsers.entry("rust".to_string())
            .or_insert_with(|| {
                let mut p = Parser::new();
                let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
                p.set_language(&lang).unwrap();
                p
            })
            .parse(code, None)
    }
    
    pub fn parse_file(&mut self, path: &Path) -> Result<Tree, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| e.to_string())?;
        
        let ext = path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
            
        match ext {
            "rs" => self.parse_rust(&content).ok_or("Parse failed".to_string()),
            _ => Err(format!("Unsupported extension: {}", ext)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse() {
        let mut parser = TreeSitterIntegration::new();
        let tree = parser.parse_rust("fn main() {}");
        assert!(tree.is_some());
    }
}
