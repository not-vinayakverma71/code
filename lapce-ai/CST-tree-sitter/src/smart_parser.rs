//! Smart Parser - Combines Incremental Parsing + LRU Cache
//! This is how modern IDEs handle large codebases efficiently

use std::path::PathBuf;
use std::sync::Arc;
use tree_sitter::{Parser, Tree};
use parking_lot::RwLock;

use crate::incremental_parser_v2::{IncrementalParserV2, Edit, IncrementalParseResult};
use crate::lru_cache::{LRUParseCache, CacheStats};

pub struct SmartParser {
    incremental: Arc<IncrementalParserV2>,
    cache: Arc<LRUParseCache>,
    stats: Arc<RwLock<SmartParserStats>>,
}

#[derive(Debug, Clone, Default)]
pub struct SmartParserStats {
    pub total_parses: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub incremental_parses: usize,
    pub full_parses: usize,
    pub total_nodes_reused: usize,
    pub total_nodes_reparsed: usize,
    pub total_time_saved_ms: f64,
}

impl SmartParser {
    pub fn new(parser: Parser, max_cached_files: usize, max_memory_mb: usize) -> Self {
        Self {
            incremental: Arc::new(IncrementalParserV2::new(parser)),
            cache: Arc::new(LRUParseCache::new(max_cached_files, max_memory_mb)),
            stats: Arc::new(RwLock::new(SmartParserStats::default())),
        }
    }

    /// Parse a file smartly
    /// - Checks cache first
    /// - Uses incremental parsing if available
    /// - Falls back to full parse
    pub fn parse(
        &self,
        path: PathBuf,
        source: &[u8],
        edit: Option<Edit>,
    ) -> Result<Tree, String> {
        let mut stats = self.stats.write();
        stats.total_parses += 1;

        // Check cache first
        if edit.is_none() {
            if let Some(cached) = self.cache.get(&path) {
                stats.cache_hits += 1;
                return Ok(cached.tree);
            }
            stats.cache_misses += 1;
        }

        // Parse with incremental if we have edit
        let result = self.incremental.parse_incremental(&path, source, edit)?;
        
        if result.reused_nodes > 0 {
            stats.incremental_parses += 1;
            stats.total_nodes_reused += result.reused_nodes;
            stats.total_time_saved_ms += result.time_saved_ms;
        } else {
            stats.full_parses += 1;
        }
        stats.total_nodes_reparsed += result.reparsed_nodes;

        // Store in cache
        self.cache.insert(path, result.tree.clone(), source.to_vec());

        Ok(result.tree)
    }

    /// Get combined statistics
    pub fn get_stats(&self) -> (SmartParserStats, CacheStats) {
        (self.stats.read().clone(), self.cache.stats())
    }

    /// Clear all caches
    pub fn clear(&self) {
        self.cache.clear();
        *self.stats.write() = SmartParserStats::default();
    }

    /// Calculate cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let stats = self.stats.read();
        if stats.cache_hits + stats.cache_misses == 0 {
            0.0
        } else {
            (stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64) * 100.0
        }
    }

    /// Calculate incremental parse rate
    pub fn incremental_rate(&self) -> f64 {
        let stats = self.stats.read();
        if stats.total_parses == 0 {
            0.0
        } else {
            (stats.incremental_parses as f64 / stats.total_parses as f64) * 100.0
        }
    }

    /// Calculate node reuse rate
    pub fn node_reuse_rate(&self) -> f64 {
        let stats = self.stats.read();
        let total = stats.total_nodes_reused + stats.total_nodes_reparsed;
        if total == 0 {
            0.0
        } else {
            (stats.total_nodes_reused as f64 / total as f64) * 100.0
        }
    }
}

impl SmartParserStats {
    pub fn print_report(&self) {
        println!("\n📊 SMART PARSER STATISTICS");
        println!("================================================================================");
        println!("Total parses:        {}", self.total_parses);
        println!("Cache hits:          {} ({:.1}%)", self.cache_hits, 
            if self.total_parses > 0 { 
                (self.cache_hits as f64 / self.total_parses as f64) * 100.0 
            } else { 0.0 });
        println!("Cache misses:        {}", self.cache_misses);
        println!("\nParse Strategy:");
        println!("  Incremental:       {} ({:.1}%)", self.incremental_parses,
            if self.total_parses > 0 {
                (self.incremental_parses as f64 / self.total_parses as f64) * 100.0
            } else { 0.0 });
        println!("  Full parse:        {} ({:.1}%)", self.full_parses,
            if self.total_parses > 0 {
                (self.full_parses as f64 / self.total_parses as f64) * 100.0
            } else { 0.0 });
        println!("\nNode Reuse:");
        println!("  Reused nodes:      {}", self.total_nodes_reused);
        println!("  Reparsed nodes:    {}", self.total_nodes_reparsed);
        println!("  Reuse rate:        {:.1}%", 
            if self.total_nodes_reused + self.total_nodes_reparsed > 0 {
                (self.total_nodes_reused as f64 / 
                    (self.total_nodes_reused + self.total_nodes_reparsed) as f64) * 100.0
            } else { 0.0 });
        println!("\nPerformance:");
        println!("  Time saved:        {:.2} ms", self.total_time_saved_ms);
        println!("  Avg saved/parse:   {:.2} ms", 
            if self.incremental_parses > 0 {
                self.total_time_saved_ms / self.incremental_parses as f64
            } else { 0.0 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_parser_workflow() {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::LANGUAGE.into().into()).unwrap();
        
        let smart = SmartParser::new(parser, 100, 100);
        
        let path = PathBuf::from("test.rs");
        let code = b"fn main() { println!(\"test\"); }";
        
        // First parse - cache miss, full parse
        let _ = smart.parse(path.clone(), code, None).unwrap();
        assert_eq!(smart.cache_hit_rate(), 0.0);
        
        // Second parse same file - cache hit
        let _ = smart.parse(path.clone(), code, None).unwrap();
        assert!(smart.cache_hit_rate() > 0.0);
        
        let (stats, cache_stats) = smart.get_stats();
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(cache_stats.entries, 1);
    }
}
