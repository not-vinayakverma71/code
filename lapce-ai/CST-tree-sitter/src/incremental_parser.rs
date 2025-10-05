//! INCREMENTAL PARSING - <10ms updates for real-time editing

use tree_sitter::{Parser, Tree, InputEdit, Point, Language};
use std::sync::Arc;
use std::time::Instant;
use parking_lot::RwLock;

pub struct IncrementalParser {
    parser: Parser,
    current_tree: Option<Tree>,
    source: String,
    language: Language,
}

impl IncrementalParser {
    pub fn new(language: Language) -> Result<Self, Box<dyn std::error::Error>> {
        let mut parser = Parser::new();
        parser.set_language(&language)?;
        
        Ok(Self {
            parser,
            current_tree: None,
            source: String::new(),
            language,
        })
    }
    
    /// Initial parse of the entire document
    pub fn parse_full(&mut self, source: &str) -> Result<ParseMetrics, Box<dyn std::error::Error>> {
        let start = Instant::now();
        
        self.source = source.to_string();
        self.current_tree = self.parser.parse(source, None);
        
        let elapsed = start.elapsed();
        
        Ok(ParseMetrics {
            parse_time_ms: elapsed.as_secs_f64() * 1000.0,
            nodes_count: self.current_tree.as_ref().map(|t| count_nodes(t.root_node())).unwrap_or(0),
            incremental: false,
        })
    }
    
    /// Incremental parse with edit information
    pub fn parse_incremental(
        &mut self,
        new_source: &str,
        edit: Edit,
    ) -> Result<ParseMetrics, Box<dyn std::error::Error>> {
        let start = Instant::now();
        
        // Convert edit to tree-sitter format
        let input_edit = InputEdit {
            start_byte: edit.start_byte,
            old_end_byte: edit.old_end_byte,
            new_end_byte: edit.new_end_byte,
            start_position: Point::new(edit.start_row, edit.start_column),
            old_end_position: Point::new(edit.old_end_row, edit.old_end_column),
            new_end_position: Point::new(edit.new_end_row, edit.new_end_column),
        };
        
        // Apply edit to existing tree
        if let Some(ref mut tree) = self.current_tree {
            tree.edit(&input_edit);
        }
        
        // Parse incrementally using old tree
        self.source = new_source.to_string();
        self.current_tree = self.parser.parse(new_source, self.current_tree.as_ref());
        
        let elapsed = start.elapsed();
        
        Ok(ParseMetrics {
            parse_time_ms: elapsed.as_secs_f64() * 1000.0,
            nodes_count: self.current_tree.as_ref().map(|t| count_nodes(t.root_node())).unwrap_or(0),
            incremental: true,
        })
    }
    
    /// Get changed ranges between old and new tree
    pub fn get_changed_ranges(&self, old_tree: &Tree) -> Vec<tree_sitter::Range> {
        if let Some(ref new_tree) = self.current_tree {
            old_tree.changed_ranges(new_tree).collect()
        } else {
            vec![]
        }
    }
    
    pub fn get_tree(&self) -> Option<&Tree> {
        self.current_tree.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct Edit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_row: usize,
    pub start_column: usize,
    pub old_end_row: usize,
    pub old_end_column: usize,
    pub new_end_row: usize,
    pub new_end_column: usize,
}

#[derive(Debug)]
pub struct ParseMetrics {
    pub parse_time_ms: f64,
    pub nodes_count: usize,
    pub incremental: bool,
}

fn count_nodes(node: tree_sitter::Node) -> usize {
    let mut count = 1;
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            count += count_nodes(child);
        }
    }
    count
}

/// Concurrent incremental parser pool for multiple files
pub struct IncrementalParserPool {
    parsers: dashmap::DashMap<std::path::PathBuf, Arc<RwLock<IncrementalParser>>>,
}

impl IncrementalParserPool {
    pub fn new() -> Self {
        Self {
            parsers: dashmap::DashMap::new(),
        }
    }
    
    pub fn get_or_create(
        &self,
        path: &std::path::Path,
        language: Language,
    ) -> Result<Arc<RwLock<IncrementalParser>>, Box<dyn std::error::Error>> {
        if let Some(parser) = self.parsers.get(path) {
            Ok(parser.clone())
        } else {
            let parser = Arc::new(RwLock::new(IncrementalParser::new(language)?));
            self.parsers.insert(path.to_path_buf(), parser.clone());
            Ok(parser)
        }
    }
    
    pub fn remove(&self, path: &std::path::Path) {
        self.parsers.remove(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_incremental_parsing_performance() {
        let lang = tree_sitter_rust::LANGUAGE.into();
        let mut parser = IncrementalParser::new(lang.into()).unwrap();
        
        // Initial parse
        let source = "fn main() {\n    println!(\"hello\");\n}";
        let metrics = parser.parse_full(source).unwrap();
        assert!(!metrics.incremental);
        
        // Incremental edit - add a line
        let new_source = "fn main() {\n    let x = 42;\n    println!(\"hello\");\n}";
        let edit = Edit {
            start_byte: 13,
            old_end_byte: 13,
            new_end_byte: 29,
            start_row: 1,
            start_column: 4,
            old_end_row: 1,
            old_end_column: 4,
            new_end_row: 2,
            new_end_column: 4,
        };
        
        let metrics = parser.parse_incremental(new_source, edit).unwrap();
        assert!(metrics.incremental);
        assert!(metrics.parse_time_ms < 10.0, "Incremental parse took {:.2}ms, expected <10ms", metrics.parse_time_ms);
    }
}
