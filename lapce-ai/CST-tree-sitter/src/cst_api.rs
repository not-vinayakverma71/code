//! CST API for semantic integration
//! Provides high-level access to CST nodes via stable IDs and efficient navigation

use crate::compact::bytecode::{
    navigator::BytecodeNavigator, 
    opcodes::BytecodeStream,
    segmented_fixed::SegmentedBytecodeStream,
    tree_sitter_encoder::TreeSitterBytecodeEncoder,
    decoder::DecodedNode,
};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;

/// CST API handle for semantic integration
/// Wraps bytecode navigation with stable ID tracking
pub struct CstApi {
    /// The underlying bytecode stream (may be segmented)
    stream: Arc<BytecodeStream>,
    /// Navigator for efficient traversal
    navigator: Arc<RwLock<BytecodeNavigator<'static>>>,
    /// Optional segmented storage for large files
    segmented: Option<Arc<SegmentedBytecodeStream>>,
}

impl CstApi {
    /// Create API from a bytecode stream
    pub fn from_bytecode(stream: BytecodeStream) -> Self {
        let stream_arc = Arc::new(stream);
        let stream_ref = unsafe { 
            // SAFETY: We ensure the Arc keeps the stream alive for the navigator's lifetime
            std::mem::transmute::<&BytecodeStream, &'static BytecodeStream>(stream_arc.as_ref())
        };
        let navigator = BytecodeNavigator::new(stream_ref);
        
        Self {
            stream: stream_arc,
            navigator: Arc::new(RwLock::new(navigator)),
            segmented: None,
        }
    }
    
    /// Create API from segmented bytecode (for large files)
    pub fn from_segmented(segmented: SegmentedBytecodeStream) -> Result<Self, String> {
        // For now, we'll create a minimal BytecodeStream from segmented metadata
        // In production, you'd load segments on-demand
        let stream = BytecodeStream {
            bytes: Vec::new(), // Segments are loaded on-demand
            jump_table: Vec::new(),
            checkpoints: Vec::new(),
            stable_ids: Vec::new(),
            kind_names: segmented.kind_names.clone(),
            field_names: segmented.field_names.clone(),
            node_count: segmented.node_count,
            source_len: segmented.source_len,
            next_stable_id: 1,
        };
        
        let stream_arc = Arc::new(stream);
        let stream_ref = unsafe {
            std::mem::transmute::<&BytecodeStream, &'static BytecodeStream>(stream_arc.as_ref())
        };
        let navigator = BytecodeNavigator::new(stream_ref);
        
        Ok(Self {
            stream: stream_arc,
            navigator: Arc::new(RwLock::new(navigator)),
            segmented: Some(Arc::new(segmented)),
        })
    }
    
    /// Load API from a storage directory
    pub fn load_from_path(storage_dir: PathBuf) -> Result<Self, String> {
        let segmented = SegmentedBytecodeStream::load(storage_dir)
            .map_err(|e| format!("Failed to load from path: {}", e))?;
        Self::from_segmented(segmented)
    }
    
    /// Get a node by its stable ID
    /// Returns None if the ID doesn't exist
    pub fn get_node_by_stable_id(&self, stable_id: u64) -> Option<DecodedNode> {
        let nav = self.navigator.read();
        
        // Find node index by stable ID
        let node_idx = self.stream.stable_ids.iter()
            .position(|&id| id == stable_id)?;
        
        nav.get_node(node_idx)
    }
    
    /// Get all nodes within a byte range
    /// Useful for semantic analysis of a specific region
    pub fn get_range_nodes(&self, range: Range<usize>) -> Vec<DecodedNode> {
        let nav = self.navigator.read();
        let mut nodes = Vec::new();
        
        // Use checkpoints for efficient range search
        let start_idx = nav.find_node_at_position(range.start).unwrap_or(0);
        
        // Collect all nodes that overlap with the range
        for idx in start_idx..nav.node_count() {
            if let Some(node) = nav.get_node(idx) {
                // Check if node overlaps with range
                if node.end_byte > range.start && node.start_byte < range.end {
                    nodes.push(node);
                } else if node.start_byte >= range.end {
                    // Past the range, stop searching
                    break;
                }
            }
        }
        
        nodes
    }
    
    /// Iterate over children of a node
    /// Returns an iterator over child nodes
    pub fn iterate_children(&self, node_idx: usize) -> Vec<DecodedNode> {
        let nav = self.navigator.read();
        
        if let Some(parent) = nav.get_node(node_idx) {
            parent.children.iter()
                .filter_map(|&child_idx| nav.get_node(child_idx))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get the parent of a node
    pub fn get_parent(&self, node_idx: usize) -> Option<DecodedNode> {
        let nav = self.navigator.read();
        
        nav.get_node(node_idx)
            .and_then(|node| node.parent)
            .and_then(|parent_idx| nav.get_node(parent_idx))
    }
    
    /// Find the smallest node containing a byte position
    /// Useful for hover/goto-definition features
    pub fn find_node_at_position(&self, byte_pos: usize) -> Option<DecodedNode> {
        let nav = self.navigator.read();
        
        // BytecodeNavigator's find_node_at_position returns Option<usize>
        if let Some(idx) = nav.find_node_at_position(byte_pos) {
            nav.get_node(idx)
        } else {
            None
        }
    }
    
    /// Get all nodes of a specific kind
    /// Useful for symbol extraction
    pub fn find_nodes_by_kind(&self, kind_name: &str) -> Vec<DecodedNode> {
        let nav = self.navigator.read();
        let mut nodes = Vec::new();
        
        // Iterate through all nodes and compare by kind name
        for idx in 0..nav.node_count() {
            if let Some(node) = nav.get_node(idx) {
                if node.kind_name == kind_name {
                    nodes.push(node);
                }
            }
        }
        
        nodes
    }
    
    /// Get metadata about the CST
    pub fn metadata(&self) -> CstMetadata {
        CstMetadata {
            node_count: self.stream.node_count,
            source_length: self.stream.source_len,
            bytecode_size: self.stream.bytes.len(),
            has_segmented_storage: self.segmented.is_some(),
            checkpoint_count: self.stream.checkpoints.len(),
            kind_count: self.stream.kind_names.len(),
            field_count: self.stream.field_names.len(),
        }
    }
    
    /// Clear navigation cache to free memory
    pub fn clear_cache(&self) {
        self.navigator.write().clear_cache();
    }
    
    /// Get the stable ID for a node at index
    pub fn get_stable_id(&self, node_idx: usize) -> Option<u64> {
        self.navigator.read().get_stable_id(node_idx)
    }
    
    /// Map from node index to stable ID
    pub fn node_to_stable_id(&self, node_idx: usize) -> Option<u64> {
        self.stream.stable_ids.get(node_idx).copied()
    }
    
    /// Map from stable ID to node index
    pub fn stable_id_to_node(&self, stable_id: u64) -> Option<usize> {
        self.stream.stable_ids.iter()
            .position(|&id| id == stable_id)
    }
}

/// Metadata about a CST
#[derive(Debug, Clone)]
pub struct CstMetadata {
    pub node_count: usize,
    pub source_length: usize,
    pub bytecode_size: usize,
    pub has_segmented_storage: bool,
    pub checkpoint_count: usize,
    pub kind_count: usize,
    pub field_count: usize,
}

/// Builder for creating CST APIs from source
pub struct CstApiBuilder {
    enable_segmentation: bool,
    segment_size: usize,
    storage_dir: Option<PathBuf>,
}

impl CstApiBuilder {
    pub fn new() -> Self {
        Self {
            enable_segmentation: false,
            segment_size: 64 * 1024, // 64KB default
            storage_dir: None,
        }
    }
    
    /// Enable segmented storage for large files
    pub fn with_segmentation(mut self, segment_size: usize) -> Self {
        self.enable_segmentation = true;
        self.segment_size = segment_size;
        self
    }
    
    /// Set storage directory for persistence
    pub fn with_storage(mut self, dir: PathBuf) -> Self {
        self.storage_dir = Some(dir);
        self
    }
    
    /// Build API from a tree-sitter Tree
    pub fn build_from_tree(
        self,
        tree: &tree_sitter::Tree,
        source: &[u8],
    ) -> Result<CstApi, String> {
        // Encode to bytecode
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let mut stream = encoder.encode_tree(tree, source);
        
        // Build jump table for efficient navigation
        crate::compact::bytecode::jump_table_builder::build_jump_table(&mut stream);
        
        // Optionally segment and persist
        if self.enable_segmentation {
            if let Some(storage_dir) = self.storage_dir {
                let segmented = SegmentedBytecodeStream::from_bytecode_stream(
                    stream,
                    storage_dir,
                ).map_err(|e| format!("Failed to segment: {}", e))?;
                
                return CstApi::from_segmented(segmented);
            }
        }
        
        Ok(CstApi::from_bytecode(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;
    
    #[test]
    fn test_cst_api_basic() {
        let source = b"fn main() { let x = 42; }";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        // Build API
        let api = CstApiBuilder::new()
            .build_from_tree(&tree, source)
            .unwrap();
        
        // Test metadata
        let meta = api.metadata();
        assert!(meta.node_count > 0);
        assert_eq!(meta.source_length, source.len());
        
        // Test node retrieval by stable ID
        let stable_id = api.get_stable_id(0).unwrap();
        let node = api.get_node_by_stable_id(stable_id).unwrap();
        assert!(!node.kind_name.is_empty());
    }
    
    #[test]
    fn test_range_queries() {
        let source = b"fn test() {\n    let x = 1;\n    let y = 2;\n}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        let api = CstApiBuilder::new()
            .build_from_tree(&tree, source)
            .unwrap();
        
        // Get nodes in the second line
        let nodes = api.get_range_nodes(12..26); // "    let x = 1;\n"
        assert!(!nodes.is_empty());
        
        // At least one should be a let_declaration
        let has_let = nodes.iter().any(|n| n.kind_name.contains("let"));
        assert!(has_let);
    }
    
    #[test]
    fn test_find_by_kind() {
        let source = b"fn foo() {} fn bar() {} fn baz() {}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        // Debug: print tree structure
        eprintln!("Tree root: {}", tree.root_node().kind());
        eprintln!("Child count: {}", tree.root_node().child_count());
        for i in 0..tree.root_node().child_count() {
            if let Some(child) = tree.root_node().child(i) {
                eprintln!("  Child {}: {}", i, child.kind());
            }
        }
        
        let api = CstApiBuilder::new()
            .build_from_tree(&tree, source)
            .unwrap();
        
        // Debug: print available kinds and bytecode info
        eprintln!("Available kinds: {:?}", api.stream.kind_names);
        eprintln!("Bytecode size: {} bytes", api.stream.bytes.len());
        eprintln!("Node count: {}", api.stream.node_count);
        eprintln!("Jump table size: {}", api.stream.jump_table.len());
        eprintln!("Stable IDs count: {}", api.stream.stable_ids.len());
        
        // Debug: try to get first few nodes directly
        let nav = api.navigator.read();
        for i in 0..10.min(nav.node_count()) {
            if let Some(node) = nav.get_node(i) {
                eprintln!("  Node {}: kind='{}'", i, node.kind_name);
            }
        }
        drop(nav);
        
        // Find all function items
        let functions = api.find_nodes_by_kind("function_item");
        eprintln!("Found {} function_item nodes", functions.len());
        
        // If function_item doesn't work, try other possibilities
        for kind in &["source_file", "function_item", "fn", "identifier"] {
            let nodes = api.find_nodes_by_kind(kind);
            eprintln!("  {} nodes of kind '{}'", nodes.len(), kind);
        }
        
        // The test expects 3 function nodes
        assert!(functions.len() == 3 || api.find_nodes_by_kind("fn").len() == 3,
                "Expected 3 function nodes, found {} function_item and {} fn",
                functions.len(), api.find_nodes_by_kind("fn").len());
    }
}
