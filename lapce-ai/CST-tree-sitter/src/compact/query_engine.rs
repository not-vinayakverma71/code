//! Query engine optimized for CompactTree
//! Leverages succinct structure for efficient searches

use super::{CompactTree, CompactNode};
use crate::query_cache::{QueryType, QueryMatch};
use crate::compact::interning::{SymbolId, InternResult, intern, resolve, INTERN_POOL};
use crate::compact::varint::{DeltaEncoder, DeltaDecoder};
use tree_sitter::{Query, QueryCursor, QueryCapture};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// Query engine for CompactTree
pub struct CompactQueryEngine {
    /// Compiled queries by type
    queries: HashMap<QueryType, CompiledQuery>,
    
    /// Query cursor cache
    cursors: Vec<CompactQueryCursor>,
    
    /// Statistics
    stats: QueryStats,
}

/// Compiled query for CompactTree
struct CompiledQuery {
    /// Original Tree-sitter query
    ts_query: Query,
    
    /// Optimized patterns for CompactTree
    patterns: Vec<QueryPattern>,
    
    /// Capture names
    capture_names: Vec<String>,
}

/// Optimized query pattern
#[derive(Debug, Clone)]
struct QueryPattern {
    /// Node kind to match
    kind: String,
    
    /// Required attributes
    is_named: Option<bool>,
    is_error: Option<bool>,
    
    /// Field name requirement
    field_name: Option<String>,
    
    /// Child patterns
    children: Vec<QueryPattern>,
    
    /// Capture index
    capture_id: Option<usize>,
}

/// Query cursor for CompactTree
pub struct CompactQueryCursor {
    /// Current position in tree
    stack: Vec<TraversalFrame>,
    
    /// Matched nodes
    matches: Vec<CompactQueryMatch>,
}

struct TraversalFrame {
    node: usize, // BP position
    pattern_index: usize,
    captures: Vec<(usize, usize)>, // (capture_id, bp_position)
}

/// Query match result
#[derive(Debug, Clone)]
pub struct CompactQueryMatch {
    pub pattern_index: usize,
    pub captures: Vec<CompactQueryCapture>,
}

#[derive(Debug, Clone)]
pub struct CompactQueryCapture {
    pub index: usize,
    pub node: usize, // BP position
}

/// Query statistics
struct QueryStats {
    total_queries: AtomicUsize,
    total_time_ms: parking_lot::RwLock<f64>,
    cache_hits: AtomicUsize,
    nodes_visited: AtomicUsize,
}

impl Default for QueryStats {
    fn default() -> Self {
        Self {
            total_queries: AtomicUsize::new(0),
            total_time_ms: parking_lot::RwLock::new(0.0),
            cache_hits: AtomicUsize::new(0),
            nodes_visited: AtomicUsize::new(0),
        }
    }
}

/// Snapshot of query statistics
#[derive(Debug, Clone)]
pub struct QueryStatsSnapshot {
    pub total_queries: usize,
    pub total_time_ms: f64,
    pub cache_hits: usize,
    pub nodes_visited: usize,
}

impl CompactQueryEngine {
    /// Create new query engine
    pub fn new() -> Self {
        Self {
            queries: HashMap::new(),
            cursors: Vec::new(),
            stats: QueryStats::default(),
        }
    }
    
    /// Register a query
    pub fn register_query(
        &mut self,
        query_type: QueryType,
        query_str: &str,
        language: tree_sitter::Language,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ts_query = Query::new(&language, query_str)?;
        
        // Parse patterns into optimized format
        let patterns = self.parse_patterns(&ts_query)?;
        
        let capture_names = ts_query.capture_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        
        self.queries.insert(query_type, CompiledQuery {
            ts_query,
            patterns,
            capture_names,
        });
        
        Ok(())
    }
    
    /// Execute query on CompactTree
    pub fn query(
        &mut self,
        tree: &CompactTree,
        query_type: QueryType,
    ) -> Result<Vec<QueryMatch>, Box<dyn std::error::Error>> {
        let start = Instant::now();
        
        // Check if query exists
        if !self.queries.contains_key(&query_type) {
            return Err("Query not registered".into());
        }
        
        // Execute query (we'll get the query inside execute_query_by_type)
        let matches = self.execute_query_by_type(tree, query_type)?;
        
        // Update statistics
        self.stats.total_queries.fetch_add(1, Ordering::Relaxed);
        *self.stats.total_time_ms.write() += start.elapsed().as_secs_f64() * 1000.0;
        
        Ok(matches)
    }
    
    /// Execute query by type (helper to avoid borrow issues)
    fn execute_query_by_type(
        &mut self,
        tree: &CompactTree,
        query_type: QueryType,
    ) -> Result<Vec<QueryMatch>, Box<dyn std::error::Error>> {
        let query = self.queries.get(&query_type)
            .ok_or("Query not registered")?;
        self.execute_query(tree, query)
    }
    
    /// Execute query with optimizations
    fn execute_query(
        &self,
        tree: &CompactTree,
        query: &CompiledQuery,
    ) -> Result<Vec<QueryMatch>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        
        // Use optimized patterns for CompactTree
        for (pattern_idx, pattern) in query.patterns.iter().enumerate() {
            let matches = self.match_pattern(tree, pattern, pattern_idx);
            
            for m in matches {
                // Convert to QueryMatch
                let query_match = QueryMatch {
                    start_byte: 0, // Will be filled from node
                    end_byte: 0,
                    capture_name: query.capture_names
                        .get(m.captures[0].index)
                        .unwrap_or(&String::new())
                        .clone(),
                };
                results.push(query_match);
            }
        }
        
        Ok(results)
    }
    
    /// Match a pattern in the tree
    fn match_pattern(
        &self,
        tree: &CompactTree,
        pattern: &QueryPattern,
        pattern_idx: usize,
    ) -> Vec<CompactQueryMatch> {
        let mut matches = Vec::new();
        let root = tree.root();
        
        // Traverse tree looking for matches
        self.match_node_recursive(tree, root.bp_position(), pattern, pattern_idx, &mut matches);
        
        matches
    }
    
    /// Recursively match nodes
    fn match_node_recursive(
        &self,
        tree: &CompactTree,
        bp_pos: usize,
        pattern: &QueryPattern,
        pattern_idx: usize,
        matches: &mut Vec<CompactQueryMatch>,
    ) {
        if let Some(node) = tree.node_at(bp_pos) {
            // Check if node matches pattern
            if self.node_matches_pattern(&node, pattern) {
                // Add to matches if this is a capture
                if let Some(capture_id) = pattern.capture_id {
                    matches.push(CompactQueryMatch {
                        pattern_index: pattern_idx,
                        captures: vec![CompactQueryCapture {
                            index: capture_id,
                            node: bp_pos,
                        }],
                    });
                }
                
                // Check children patterns
                if !pattern.children.is_empty() {
                    self.match_children_patterns(tree, &node, &pattern.children, pattern_idx, matches);
                }
            }
            
            // Continue searching in children
            for child in node.children() {
                self.match_node_recursive(tree, child.bp_position(), pattern, pattern_idx, matches);
            }
            
            self.stats.nodes_visited.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// Check if node matches pattern
    fn node_matches_pattern(&self, node: &CompactNode, pattern: &QueryPattern) -> bool {
        // Check kind
        if node.kind() != pattern.kind {
            return false;
        }
        
        // Check attributes
        if let Some(is_named) = pattern.is_named {
            if node.is_named() != is_named {
                return false;
            }
        }
        
        if let Some(is_error) = pattern.is_error {
            if node.is_error() != is_error {
                return false;
            }
        }
        
        // Check field name
        if let Some(ref field) = pattern.field_name {
            if node.field_name() != Some(field.as_str()) {
                return false;
            }
        }
        
        true
    }
    
    /// Match children patterns
    fn match_children_patterns(
        &self,
        tree: &CompactTree,
        parent: &CompactNode,
        patterns: &[QueryPattern],
        pattern_idx: usize,
        matches: &mut Vec<CompactQueryMatch>,
    ) {
        // Simple ordered matching for now
        // TODO: Implement full pattern matching logic
        let children: Vec<_> = parent.children().collect();
        
        if children.len() < patterns.len() {
            return;
        }
        
        for (child, pattern) in children.iter().zip(patterns.iter()) {
            if !self.node_matches_pattern(child, pattern) {
                return;
            }
        }
        
        // All patterns matched
        let mut captures = Vec::new();
        for (i, (child, pattern)) in children.iter().zip(patterns.iter()).enumerate() {
            if let Some(capture_id) = pattern.capture_id {
                captures.push(CompactQueryCapture {
                    index: capture_id,
                    node: child.bp_position(),
                });
            }
        }
        
        if !captures.is_empty() {
            matches.push(CompactQueryMatch {
                pattern_index: pattern_idx,
                captures,
            });
        }
    }
    
    /// Parse Tree-sitter query into optimized patterns
    fn parse_patterns(&self, query: &Query) -> Result<Vec<QueryPattern>, Box<dyn std::error::Error>> {
        // Simplified pattern extraction
        // TODO: Implement full S-expression parsing
        let mut patterns = Vec::new();
        
        // For now, create simple patterns based on capture names
        for (i, name) in query.capture_names().iter().enumerate() {
            patterns.push(QueryPattern {
                kind: name.to_string(),
                is_named: Some(true),
                is_error: None,
                field_name: None,
                children: Vec::new(),
                capture_id: Some(i),
            });
        }
        
        Ok(patterns)
    }
    
    /// Get query statistics
    pub fn stats(&self) -> QueryStatsSnapshot {
        QueryStatsSnapshot {
            total_queries: self.stats.total_queries.load(Ordering::Relaxed),
            total_time_ms: *self.stats.total_time_ms.read(),
            cache_hits: self.stats.cache_hits.load(Ordering::Relaxed),
            nodes_visited: self.stats.nodes_visited.load(Ordering::Relaxed),
        }
    }
}

/// Advanced query operations using succinct structure
pub struct SuccinctQueryOps;

impl SuccinctQueryOps {
    /// Find all nodes of a specific kind using BP structure
    pub fn find_by_kind(tree: &CompactTree, kind: &str) -> Vec<usize> {
        let mut results = Vec::new();
        let mut stack = vec![0]; // Start with root
        
        while let Some(bp_pos) = stack.pop() {
            if let Some(node) = tree.node_at(bp_pos) {
                if node.kind() == kind {
                    results.push(bp_pos);
                }
                
                // Add children to stack
                for child in node.children() {
                    stack.push(child.bp_position());
                }
            }
        }
        
        results
    }
    
    /// Find nodes in byte range using succinct position index
    pub fn find_in_range(tree: &CompactTree, start: usize, end: usize) -> Vec<usize> {
        let mut results = Vec::new();
        let mut stack = vec![0];
        
        while let Some(bp_pos) = stack.pop() {
            if let Some(node) = tree.node_at(bp_pos) {
                let node_start = node.start_byte();
                let node_end = node.end_byte();
                
                // Check if node overlaps with range
                if node_start < end && node_end > start {
                    results.push(bp_pos);
                    
                    // Add children only if they might overlap
                    for child in node.children() {
                        stack.push(child.bp_position());
                    }
                }
            }
        }
        
        results
    }
    
    /// Find parent chain using BP enclose operation
    pub fn get_parent_chain(tree: &CompactTree, bp_pos: usize) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut current = bp_pos;
        
        while let Some(node) = tree.node_at(current) {
            if let Some(parent) = node.parent() {
                chain.push(parent.bp_position());
                current = parent.bp_position();
            } else {
                break;
            }
        }
        
        chain.reverse();
        chain
    }
    
    /// Find siblings efficiently using BP operations
    pub fn get_siblings(tree: &CompactTree, bp_pos: usize) -> Vec<usize> {
        let mut siblings = Vec::new();
        
        if let Some(node) = tree.node_at(bp_pos) {
            // Get previous siblings
            let mut prev = node.previous_sibling();
            while let Some(p) = prev {
                siblings.push(p.bp_position());
                prev = p.previous_sibling();
            }
            
            siblings.reverse();
            
            // Get next siblings
            let mut next = node.next_sibling();
            while let Some(n) = next {
                siblings.push(n.bp_position());
                next = n.next_sibling();
            }
        }
        
        siblings
    }
    
    /// Fast subtree size calculation using BP
    pub fn subtree_size(tree: &CompactTree, bp_pos: usize) -> usize {
        if let Some(node) = tree.node_at(bp_pos) {
            node.subtree_size()
        } else {
            0
        }
    }
}

/// Encoded position list using delta-varint encoding
#[derive(Clone, Debug)]
struct EncodedPositions {
    /// Delta-encoded varint positions
    data: Vec<u8>,
    /// Number of positions encoded
    count: usize,
}

impl EncodedPositions {
    /// Encode positions using delta-varint
    fn from_positions(positions: &[usize]) -> Self {
        if positions.is_empty() {
            return Self { data: Vec::new(), count: 0 };
        }
        
        // Ensure positions are sorted for delta encoding
        let mut sorted = positions.to_vec();
        sorted.sort_unstable();
        
        let mut encoder = DeltaEncoder::new();
        for &pos in &sorted {
            encoder.encode(pos as u64);
        }
        
        Self {
            data: encoder.finish(),
            count: sorted.len(),
        }
    }
    
    /// Decode all positions
    fn to_positions(&self) -> Vec<usize> {
        if self.data.is_empty() {
            return Vec::new();
        }
        
        let mut decoder = DeltaDecoder::new(&self.data);
        let mut positions = Vec::with_capacity(self.count);
        
        while decoder.has_more() {
            if let Ok(pos) = decoder.decode() {
                positions.push(pos as usize);
            } else {
                break;
            }
        }
        
        positions
    }
    
    /// Memory size in bytes
    fn size_bytes(&self) -> usize {
        self.data.len() + std::mem::size_of::<usize>()
    }
}

/// Symbol index for fast lookup with varint-encoded positions
pub struct SymbolIndex {
    /// All symbol occurrences (interned) with delta-varint positions
    symbols: HashMap<SymbolId, EncodedPositions>,
    
    /// Definition sites (interned) with delta-varint positions  
    definitions: HashMap<SymbolId, EncodedPositions>,
    
    /// Reference sites (interned) with delta-varint positions
    references: HashMap<SymbolId, EncodedPositions>,
    
    /// Temporary buffers for building (cleared after build)
    temp_symbols: HashMap<SymbolId, Vec<usize>>,
    temp_definitions: HashMap<SymbolId, Vec<usize>>,
    temp_references: HashMap<SymbolId, Vec<usize>>,
}

impl SymbolIndex {
    /// Build symbol index from CompactTree
    pub fn build(tree: &CompactTree) -> Self {
        let mut index = Self {
            symbols: HashMap::new(),
            definitions: HashMap::new(),
            references: HashMap::new(),
            temp_symbols: HashMap::new(),
            temp_definitions: HashMap::new(),
            temp_references: HashMap::new(),
        };
        
        // First pass: collect all positions in temporary buffers
        index.index_node(tree, 0);
        
        // Second pass: encode all positions using delta-varint
        index.finalize_encoding();
        
        index
    }
    
    /// Finalize encoding by converting temporary buffers to encoded positions
    fn finalize_encoding(&mut self) {
        // Encode symbols
        for (id, positions) in self.temp_symbols.drain() {
            self.symbols.insert(id, EncodedPositions::from_positions(&positions));
        }
        
        // Encode definitions
        for (id, positions) in self.temp_definitions.drain() {
            self.definitions.insert(id, EncodedPositions::from_positions(&positions));
        }
        
        // Encode references
        for (id, positions) in self.temp_references.drain() {
            self.references.insert(id, EncodedPositions::from_positions(&positions));
        }
    }
    
    fn index_node(&mut self, tree: &CompactTree, bp_pos: usize) {
        if let Some(node) = tree.node_at(bp_pos) {
            // Index based on node kind
            match node.kind() {
                "identifier" | "type_identifier" => {
                    // Add to symbols
                    if let Ok(text) = node.utf8_text(tree.source()) {
                        // Intern the string (always intern identifiers)
                        let intern_result = intern(text);
                        if let InternResult::Interned(id) = intern_result {
                            self.temp_symbols.entry(id)
                                .or_default()
                                .push(bp_pos);
                        }
                        // Note: bypassed strings are not indexed (should never happen for identifiers)
                    }
                }
                "function_definition" | "function_item" | "method_definition" => {
                    // Add to definitions
                    if let Some(name_node) = self.find_name_node(tree, &node) {
                        if let Ok(text) = name_node.utf8_text(tree.source()) {
                            let intern_result = intern(text);
                            if let InternResult::Interned(id) = intern_result {
                                self.temp_definitions.entry(id)
                                    .or_default()
                                    .push(bp_pos);
                            }
                        }
                    }
                }
                "call_expression" | "function_call" => {
                    // Add to references
                    if let Some(name_node) = self.find_name_node(tree, &node) {
                        if let Ok(text) = name_node.utf8_text(tree.source()) {
                            let intern_result = intern(text);
                            if let InternResult::Interned(id) = intern_result {
                                self.temp_references.entry(id)
                                    .or_default()
                                    .push(bp_pos);
                            }
                        }
                    }
                }
                _ => {}
            }
            
            // Recurse to children
            for child in node.children() {
                self.index_node(tree, child.bp_position());
            }
        }
    }
    
    fn find_name_node<'a>(&self, tree: &'a CompactTree, parent: &CompactNode<'a>) -> Option<CompactNode<'a>> {
        // Find identifier child
        parent.children()
            .find(|child| child.kind() == "identifier" || child.kind() == "type_identifier")
    }
    
    /// Find all occurrences of a symbol
    pub fn find_symbol(&self, name: &str) -> Vec<usize> {
        // Look up the interned ID from the global pool
        INTERN_POOL.get_id(name)
            .and_then(|id| self.symbols.get(&id))
            .map(|encoded| encoded.to_positions())
            .unwrap_or_default()
    }
    
    /// Find definition of a symbol
    pub fn find_definition(&self, name: &str) -> Option<usize> {
        // Look up the interned ID from the global pool
        INTERN_POOL.get_id(name)
            .and_then(|id| self.definitions.get(&id))
            .and_then(|encoded| {
                let positions = encoded.to_positions();
                positions.first().copied()
            })
    }
    
    /// Find all references to a symbol
    pub fn find_references(&self, name: &str) -> Vec<usize> {
        // Look up the interned ID from the global pool
        INTERN_POOL.get_id(name)
            .and_then(|id| self.references.get(&id))
            .map(|encoded| encoded.to_positions())
            .unwrap_or_default()
    }
    
    /// Calculate total memory usage of the index
    pub fn memory_usage(&self) -> usize {
        let symbols_size: usize = self.symbols.values()
            .map(|e| e.size_bytes())
            .sum();
        let definitions_size: usize = self.definitions.values()
            .map(|e| e.size_bytes())
            .sum();
        let references_size: usize = self.references.values()
            .map(|e| e.size_bytes())
            .sum();
        
        symbols_size + definitions_size + references_size + 
            (self.symbols.len() + self.definitions.len() + self.references.len()) * 
            std::mem::size_of::<(SymbolId, EncodedPositions)>()
    }
}
