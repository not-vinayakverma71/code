//! CST-tree-sitter: Complete 6-Phase Optimization Pipeline
//! 
//! Implements all optimization phases from the journey document:
//! - Phase 1: Varint + Packing + Interning (40% reduction)
//! - Phase 2: Delta Compression (60% cumulative)
//! - Phase 3: Bytecode Trees (75% cumulative)
//! - Phase 4a: Frozen Tier (93% cumulative)
//! - Phase 4b: Memory-Mapped Sources (95% cumulative)
//! - Phase 4c: Segmented Bytecode (97% cumulative)

// Core modules
pub mod compact;
pub mod cache;
pub mod phase4_cache;
pub mod complete_pipeline;
pub mod parser_pool;
pub mod cst_codec;
pub mod dynamic_compressed_cache;

// Re-export main pipeline components
pub use complete_pipeline::{
    CompletePipeline,
    CompletePipelineConfig,
    PipelineStats,
    ProcessedResult,
    StorageLocation,
};

pub use phase4_cache::{
    Phase4Cache,
    Phase4Config,
    Phase4Stats,
};

// Re-export bytecode components
pub use compact::bytecode::{
    TreeSitterBytecodeEncoder,
    TreeSitterBytecodeDecoder,
    BytecodeStream,
    SegmentedBytecodeStream,
    Opcode,
};

use tree_sitter::{Parser, Tree, Language};
use std::sync::Arc;
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
