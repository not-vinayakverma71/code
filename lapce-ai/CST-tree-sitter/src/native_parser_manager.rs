//! Native Parser Manager - Facade for semantic layer integration
//! Implements the interface described in docs/05-TREE-SITTER-INTEGRATION.md

use std::path::Path;
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use tree_sitter::{Parser, Tree, Language};
use crate::cst_api::{CstApi, CstApiBuilder};
use crate::symbols::SymbolExtractor;
use crate::language::registry::LanguageRegistry;

/// Native parser manager for tree-sitter integration
pub struct NativeParserManager {
    /// Language parsers (shared instances)
    parsers: Arc<RwLock<HashMap<String, Parser>>>,
    
    /// CST cache
    cst_cache: Arc<RwLock<HashMap<String, Arc<CstApi>>>>,
    
    /// Metrics
    metrics: Arc<ParserMetrics>,
}

/// Parser metrics for observability
pub struct ParserMetrics {
    pub parse_count: std::sync::atomic::AtomicU64,
    pub cache_hits: std::sync::atomic::AtomicU64,
    pub cache_misses: std::sync::atomic::AtomicU64,
    pub total_parse_time_ms: std::sync::atomic::AtomicU64,
}

impl ParserMetrics {
    pub fn new() -> Self {
        Self {
            parse_count: std::sync::atomic::AtomicU64::new(0),
            cache_hits: std::sync::atomic::AtomicU64::new(0),
            cache_misses: std::sync::atomic::AtomicU64::new(0),
            total_parse_time_ms: std::sync::atomic::AtomicU64::new(0),
        }
    }
    
    pub fn record_parse(&self, duration_ms: u64) {
        self.parse_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.total_parse_time_ms.fetch_add(duration_ms, std::sync::atomic::Ordering::Relaxed);
    }
    
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Parse result with metadata
pub struct ParseResult {
    pub cst_api: Arc<CstApi>,
    pub language_name: String,
    pub parse_time: std::time::Duration,
}

impl NativeParserManager {
    /// Create new parser manager
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            parsers: Arc::new(RwLock::new(HashMap::new())),
            cst_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(ParserMetrics::new()),
        })
    }
    
    /// Get or create parser for a language
    fn get_parser(&self, language_name: &str) -> Result<Parser, String> {
        let mut parsers = self.parsers.write();
        
        if let Some(parser) = parsers.get(language_name) {
            // Return a new parser with same language
            let mut new_parser = Parser::new();
            new_parser.set_language(&parser.language().unwrap()).unwrap();
            Ok(new_parser)
        } else {
            // Get language from registry
            let registry = LanguageRegistry::instance();
            let lang_info = registry.by_name(language_name)
                .map_err(|e| format!("Failed to get language: {}", e))?;
            
            let mut parser = Parser::new();
            parser.set_language(&lang_info.language)
                .map_err(|e| format!("Failed to set language: {:?}", e))?;
            
            // Cache it
            parsers.insert(language_name.to_string(), parser);
            
            // Return a new instance
            let mut new_parser = Parser::new();
            new_parser.set_language(&lang_info.language).unwrap();
            Ok(new_parser)
        }
    }
    
    
    /// Parse a file and return CST API
    pub async fn parse_file(&self, path: &Path) -> Result<ParseResult, String> {
        let start = std::time::Instant::now();
        
        // Detect language from file extension
        let registry = LanguageRegistry::instance();
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| format!("No extension for file: {:?}", path))?;
        
        let lang_info = registry.by_extension(extension)
            .map_err(|e| format!("Failed to detect language: {}", e))?;
        
        let language_name = lang_info.name.to_string();
        
        // Check cache
        let cache_key = path.to_string_lossy().to_string();
        if let Some(cached) = self.cst_cache.read().get(&cache_key) {
            self.metrics.record_cache_hit();
            return Ok(ParseResult {
                cst_api: cached.clone(),
                language_name,
                parse_time: start.elapsed(),
            });
        }
        
        self.metrics.record_cache_miss();
        
        // Read file
        let source = tokio::fs::read(path).await
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        // Parse with tree-sitter
        let mut parser = self.get_parser(&language_name)?;
        let tree = parser.parse(&source, None)
            .ok_or_else(|| "Failed to parse file".to_string())?;
        
        // Build CST API
        let cst_api = CstApiBuilder::new()
            .build_from_tree(&tree, &source)?;
        
        let cst_api = Arc::new(cst_api);
        
        // Cache the result
        self.cst_cache.write().insert(cache_key, cst_api.clone());
        
        // Record metrics
        let duration = start.elapsed();
        self.metrics.record_parse(duration.as_millis() as u64);
        
        Ok(ParseResult {
            cst_api,
            language_name,
            parse_time: duration,
        })
    }
    
    /// Extract symbols from a file
    pub async fn extract_symbols(&self, path: &Path) -> Result<Vec<crate::symbols::Symbol>, String> {
        let parse_result = self.parse_file(path).await?;
        
        // Use symbol extractor
        let mut extractor = SymbolExtractor::new(parse_result.language_name.clone());
        
        // For now, we'll use a simple approach
        // In production, this would traverse the CST properly
        let mut symbols = Vec::new();
        
        // Find all function-like nodes
        for kind in &["function_item", "function_declaration", "method_definition", "def"] {
            let nodes = parse_result.cst_api.find_nodes_by_kind(kind);
            for node in nodes {
                symbols.push(crate::symbols::Symbol {
                    name: format!("function {}", node.kind_name),
                    kind: crate::ast::kinds::CanonicalKind::FunctionDeclaration,
                    range: crate::symbols::Range {
                        start: crate::symbols::Position { line: 0, column: node.start_byte },
                        end: crate::symbols::Position { line: 0, column: node.end_byte },
                    },
                    children: Vec::new(),
                    doc_comment: None,
                    stable_id: node.stable_id,
                });
            }
        }
        
        Ok(symbols)
    }
    
    /// Clear cache to free memory
    pub fn clear_cache(&self) {
        self.cst_cache.write().clear();
    }
    
    /// Get metrics
    pub fn metrics(&self) -> &ParserMetrics {
        &self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[tokio::test]
    async fn test_parser_manager() {
        let manager = NativeParserManager::new().unwrap();
        
        // Create a temp file with .rs extension
        let mut file = tempfile::Builder::new()
            .suffix(".rs")
            .tempfile()
            .unwrap();
        writeln!(file, "fn main() {{ println!(\"Hello\"); }}").unwrap();
        
        // Parse it
        let result = manager.parse_file(file.path()).await.unwrap();
        
        // Check we got a valid CST
        assert!(result.cst_api.metadata().node_count > 0);
        assert_eq!(result.language_name, "rust");
        
        // Check caching works
        let result2 = manager.parse_file(file.path()).await.unwrap();
        assert_eq!(
            Arc::as_ptr(&result.cst_api),
            Arc::as_ptr(&result2.cst_api),
            "Should return cached instance"
        );
        
        // Check metrics
        assert_eq!(manager.metrics.cache_hits.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_eq!(manager.metrics.cache_misses.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}
