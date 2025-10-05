//! Incremental updates for CompactTree
//! Efficient edit handling for real-time editing

use super::{CompactTree, CompactTreeBuilder, CompactNode};
use tree_sitter::{Parser, Tree, InputEdit, Point, Language};
use std::sync::Arc;
use std::time::Instant;
use std::collections::BTreeMap;
use bytes::Bytes;

/// Segmented CompactTree for incremental updates
/// Divides tree into segments that can be independently rebuilt
pub struct IncrementalCompactTree {
    /// Segments of the tree
    segments: BTreeMap<usize, TreeSegment>,
    
    /// Full source text
    source: Vec<u8>,
    
    /// Parser for incremental updates
    parser: Parser,
    
    /// Current Tree-sitter tree (for edits)
    current_tree: Option<Tree>,
    
    /// Segment size threshold (nodes)
    segment_size: usize,
}

/// A segment of the compact tree
#[derive(Clone)]
struct TreeSegment {
    /// Start byte in source
    start_byte: usize,
    
    /// End byte in source
    end_byte: usize,
    
    /// Compact representation of this segment
    compact_tree: Arc<CompactTree>,
    
    /// Node count in segment
    node_count: usize,
    
    /// Last modified timestamp
    last_modified: Instant,
    
    /// Dirty flag (needs rebuild)
    dirty: bool,
}

/// Edit information
#[derive(Debug, Clone)]
pub struct Edit {
    /// Start byte of the edit
    pub start_byte: usize,
    
    /// Old end byte (before edit)
    pub old_end_byte: usize,
    
    /// New end byte (after edit)
    pub new_end_byte: usize,
    
    /// Start position (row, column)
    pub start_position: Point,
    
    /// Old end position
    pub old_end_position: Point,
    
    /// New end position
    pub new_end_position: Point,
}

impl IncrementalCompactTree {
    /// Create new incremental tree
    pub fn new(language: Language, segment_size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let mut parser = Parser::new();
        parser.set_language(&language)?;
        
        Ok(Self {
            segments: BTreeMap::new(),
            source: Vec::new(),
            parser,
            current_tree: None,
            segment_size,
        })
    }
    
    /// Parse full source and segment it
    pub fn parse_full(&mut self, source: &[u8]) -> Result<ParseMetrics, Box<dyn std::error::Error>> {
        let start = Instant::now();
        
        self.source = source.to_vec();
        
        // Parse with Tree-sitter
        self.current_tree = self.parser.parse(source, None);
        
        // Build segments (clone tree to avoid borrow issues)
        if let Some(ref tree) = self.current_tree.clone() {
            self.build_segments(tree, source)?;
        }
        
        let elapsed = start.elapsed();
        
        Ok(ParseMetrics {
            parse_time_ms: elapsed.as_secs_f64() * 1000.0,
            total_nodes: self.total_nodes(),
            segment_count: self.segments.len(),
            incremental: false,
            rebuilt_segments: 0,
        })
    }
    
    /// Apply incremental edit
    pub fn apply_edit(
        &mut self,
        edit: &Edit,
        new_source: &[u8],
    ) -> Result<ParseMetrics, Box<dyn std::error::Error>> {
        let start = Instant::now();
        
        // Update source
        self.source = new_source.to_vec();
        
        // Mark affected segments as dirty
        self.mark_dirty_segments(edit);
        
        // Apply edit to Tree-sitter tree
        if let Some(tree) = &mut self.current_tree {
            let ts_edit = InputEdit {
                start_byte: edit.start_byte,
                old_end_byte: edit.old_end_byte,
                new_end_byte: edit.new_end_byte,
                start_position: edit.start_position,
                old_end_position: edit.old_end_position,
                new_end_position: edit.new_end_position,
            };
            tree.edit(&ts_edit);
        }
        
        // Reparse with Tree-sitter
        self.current_tree = self.parser.parse(new_source, self.current_tree.as_ref());
        
        // Rebuild dirty segments
        let rebuilt_count = self.rebuild_dirty_segments()?;
        
        let elapsed = start.elapsed();
        
        Ok(ParseMetrics {
            parse_time_ms: elapsed.as_secs_f64() * 1000.0,
            total_nodes: self.total_nodes(),
            segment_count: self.segments.len(),
            incremental: true,
            rebuilt_segments: rebuilt_count,
        })
    }
    
    /// Build segments from tree
    fn build_segments(&mut self, tree: &Tree, source: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        self.segments.clear();
        
        let root = tree.root_node();
        let mut segments = Vec::new();
        
        // Find natural segment boundaries (functions, classes, etc.)
        self.find_segment_boundaries(root, source, &mut segments);
        
        // If no natural boundaries, create uniform segments
        if segments.is_empty() {
            segments = self.create_uniform_segments(tree, source);
        }
        
        // Build compact tree for each segment
        for (i, (start, end)) in segments.iter().enumerate() {
            let segment_tree = self.build_segment_tree(tree, source, *start, *end)?;
            
            self.segments.insert(*start, TreeSegment {
                start_byte: *start,
                end_byte: *end,
                compact_tree: Arc::new(segment_tree),
                node_count: 0, // Will be updated
                last_modified: Instant::now(),
                dirty: false,
            });
        }
        
        Ok(())
    }
    
    /// Find natural segment boundaries
    fn find_segment_boundaries(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        segments: &mut Vec<(usize, usize)>,
    ) {
        // Look for function/class definitions as natural boundaries
        if node.kind() == "function_definition" ||
           node.kind() == "function_item" ||
           node.kind() == "class_definition" ||
           node.kind() == "impl_item" ||
           node.kind() == "module" {
            segments.push((node.start_byte(), node.end_byte()));
        } else {
            // Recurse to children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.find_segment_boundaries(child, source, segments);
            }
        }
    }
    
    /// Create uniform segments if no natural boundaries
    fn create_uniform_segments(&self, tree: &Tree, source: &[u8]) -> Vec<(usize, usize)> {
        let mut segments = Vec::new();
        let total_nodes = count_nodes(tree.root_node());
        let num_segments = (total_nodes / self.segment_size).max(1);
        let bytes_per_segment = source.len() / num_segments;
        
        for i in 0..num_segments {
            let start = i * bytes_per_segment;
            let end = if i == num_segments - 1 {
                source.len()
            } else {
                (i + 1) * bytes_per_segment
            };
            segments.push((start, end));
        }
        
        segments
    }
    
    /// Build compact tree for a segment
    fn build_segment_tree(
        &self,
        tree: &Tree,
        source: &[u8],
        start_byte: usize,
        end_byte: usize,
    ) -> Result<CompactTree, Box<dyn std::error::Error>> {
        // For now, build the whole tree
        // TODO: Build only the segment subtree
        let builder = CompactTreeBuilder::new();
        Ok(builder.build(tree, source))
    }
    
    /// Mark segments affected by edit as dirty
    fn mark_dirty_segments(&mut self, edit: &Edit) {
        for segment in self.segments.values_mut() {
            // Check if segment overlaps with edit
            if segment.start_byte <= edit.new_end_byte && segment.end_byte >= edit.start_byte {
                segment.dirty = true;
            }
        }
    }
    
    /// Rebuild dirty segments
    fn rebuild_dirty_segments(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut rebuilt_count = 0;
        
        if let Some(tree) = &self.current_tree {
            let dirty_segments: Vec<_> = self.segments
                .iter()
                .filter(|(_, seg)| seg.dirty)
                .map(|(&k, _)| k)
                .collect();
            
            for start_byte in dirty_segments {
                // Extract segment bounds before mutable borrow
                let (seg_start, seg_end) = {
                    if let Some(seg) = self.segments.get(&start_byte) {
                        (seg.start_byte, seg.end_byte)
                    } else {
                        continue;
                    }
                };
                
                // Now build the new tree
                let new_tree = self.build_segment_tree(
                    tree,
                    &self.source,
                    seg_start,
                    seg_end,
                )?;
                
                // Update the segment
                if let Some(segment) = self.segments.get_mut(&start_byte) {
                    segment.compact_tree = Arc::new(new_tree);
                    segment.last_modified = Instant::now();
                    segment.dirty = false;
                    rebuilt_count += 1;
                }
            }
        }
        
        Ok(rebuilt_count)
    }
    
    /// Get total node count
    fn total_nodes(&self) -> usize {
        self.segments.values().map(|s| s.node_count).sum()
    }
    
    /// Get segment containing byte offset
    pub fn get_segment_at(&self, byte_offset: usize) -> Option<&TreeSegment> {
        self.segments
            .values()
            .find(|seg| seg.start_byte <= byte_offset && byte_offset < seg.end_byte)
    }
    
    /// Get all segments in range
    pub fn get_segments_in_range(&self, start: usize, end: usize) -> Vec<&TreeSegment> {
        self.segments
            .values()
            .filter(|seg| seg.start_byte < end && seg.end_byte > start)
            .collect()
    }
    
    /// Merge adjacent small segments
    pub fn optimize_segments(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        // TODO: Implement segment merging for optimization
        Ok(0)
    }
}

/// Metrics for parse operations
#[derive(Debug, Clone)]
pub struct ParseMetrics {
    pub parse_time_ms: f64,
    pub total_nodes: usize,
    pub segment_count: usize,
    pub incremental: bool,
    pub rebuilt_segments: usize,
}

/// Count nodes in tree
fn count_nodes(node: tree_sitter::Node) -> usize {
    let mut count = 1;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count += count_nodes(child);
    }
    count
}

/// Microtree cache for frequently edited segments
pub struct MicrotreeCache {
    /// Cache of small trees for hot segments
    cache: BTreeMap<usize, CachedMicrotree>,
    
    /// Maximum cache size
    max_size: usize,
}

struct CachedMicrotree {
    /// Compact tree
    tree: Arc<CompactTree>,
    
    /// Access count
    access_count: usize,
    
    /// Last access time
    last_access: Instant,
    
    /// Size in bytes
    size_bytes: usize,
}

impl MicrotreeCache {
    /// Create new microtree cache
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: BTreeMap::new(),
            max_size,
        }
    }
    
    /// Get or build microtree
    pub fn get_or_build(
        &mut self,
        start: usize,
        tree: &Tree,
        source: &[u8],
    ) -> Arc<CompactTree> {
        if let Some(cached) = self.cache.get_mut(&start) {
            cached.access_count += 1;
            cached.last_access = Instant::now();
            return cached.tree.clone();
        }
        
        // Build new microtree
        let builder = CompactTreeBuilder::new();
        let compact = Arc::new(builder.build(tree, source));
        
        // Add to cache
        if self.cache.len() >= self.max_size {
            // Evict least recently used
            self.evict_lru();
        }
        
        self.cache.insert(start, CachedMicrotree {
            tree: compact.clone(),
            access_count: 1,
            last_access: Instant::now(),
            size_bytes: 0, // TODO: Calculate size
        });
        
        compact
    }
    
    /// Evict least recently used entry
    fn evict_lru(&mut self) {
        if let Some((&key, _)) = self.cache
            .iter()
            .min_by_key(|(_, v)| v.last_access) {
            self.cache.remove(&key);
        }
    }
}
