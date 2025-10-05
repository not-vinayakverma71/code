//! Dual representation system for Tree-sitter vs CompactTree
//! Allows switching between memory-optimized and standard representations

use tree_sitter::{Tree, Node};
use crate::compact::{CompactTree, CompactNode, CompactTreeBuilder};
use bytes::Bytes;
use std::fmt;

/// Dual representation of a syntax tree
/// Can be either a standard Tree-sitter tree or a CompactTree
#[derive(Clone)]
pub enum DualTree {
    /// Standard Tree-sitter representation (~90 bytes/node)
    TreeSitter(Tree),
    
    /// Compact representation (~18 bytes/node)
    Compact(Box<CompactTree>),
}

impl DualTree {
    /// Create from Tree-sitter tree
    pub fn from_tree_sitter(tree: Tree) -> Self {
        DualTree::TreeSitter(tree)
    }
    
    /// Create compact representation from Tree-sitter tree
    pub fn from_tree_sitter_compact(tree: &Tree, source: &[u8]) -> Self {
        let builder = CompactTreeBuilder::new();
        let compact_tree = builder.build(tree, source);
        DualTree::Compact(Box::new(compact_tree))
    }
    
    /// Convert to compact representation if not already
    pub fn to_compact(self, source: &[u8]) -> Self {
        match self {
            DualTree::TreeSitter(tree) => {
                let builder = CompactTreeBuilder::new();
                let compact_tree = builder.build(&tree, source);
                DualTree::Compact(Box::new(compact_tree))
            }
            compact => compact,
        }
    }
    
    /// Get root node wrapper
    pub fn root(&self) -> DualNode {
        match self {
            DualTree::TreeSitter(tree) => DualNode::TreeSitter(tree.root_node()),
            DualTree::Compact(compact) => DualNode::Compact(compact.root()),
        }
    }
    
    /// Check if this is compact representation
    pub fn is_compact(&self) -> bool {
        matches!(self, DualTree::Compact(_))
    }
    
    /// Get memory usage estimate in bytes
    pub fn memory_bytes(&self) -> usize {
        match self {
            DualTree::TreeSitter(tree) => {
                // Estimate ~90 bytes per node
                tree.root_node().descendant_count() * 90
            }
            DualTree::Compact(compact) => compact.memory_bytes(),
        }
    }
    
    /// Get node count
    pub fn node_count(&self) -> usize {
        match self {
            DualTree::TreeSitter(tree) => tree.root_node().descendant_count(),
            DualTree::Compact(compact) => compact.node_count(),
        }
    }
}

/// Dual representation of a node
/// Provides unified API over both representations
#[derive(Clone, Copy)]
pub enum DualNode<'tree> {
    TreeSitter(Node<'tree>),
    Compact(CompactNode<'tree>),
}

impl<'tree> DualNode<'tree> {
    /// Get node kind
    pub fn kind(&self) -> &str {
        match self {
            DualNode::TreeSitter(node) => node.kind(),
            DualNode::Compact(node) => node.kind(),
        }
    }
    
    /// Get start byte
    pub fn start_byte(&self) -> usize {
        match self {
            DualNode::TreeSitter(node) => node.start_byte(),
            DualNode::Compact(node) => node.start_byte(),
        }
    }
    
    /// Get end byte
    pub fn end_byte(&self) -> usize {
        match self {
            DualNode::TreeSitter(node) => node.end_byte(),
            DualNode::Compact(node) => node.end_byte(),
        }
    }
    
    /// Get child count
    pub fn child_count(&self) -> usize {
        match self {
            DualNode::TreeSitter(node) => node.child_count(),
            DualNode::Compact(node) => node.child_count(),
        }
    }
    
    /// Check if named
    pub fn is_named(&self) -> bool {
        match self {
            DualNode::TreeSitter(node) => node.is_named(),
            DualNode::Compact(node) => node.is_named(),
        }
    }
    
    /// Get parent
    pub fn parent(&self) -> Option<Self> {
        match self {
            DualNode::TreeSitter(node) => node.parent().map(DualNode::TreeSitter),
            DualNode::Compact(node) => node.parent().map(DualNode::Compact),
        }
    }
    
    /// Get first child
    pub fn first_child(&self) -> Option<Self> {
        match self {
            DualNode::TreeSitter(node) => {
                let mut cursor = node.walk();
                cursor.goto_first_child().then(|| DualNode::TreeSitter(cursor.node()))
            }
            DualNode::Compact(node) => node.first_child().map(DualNode::Compact),
        }
    }
    
    /// Get next sibling
    pub fn next_sibling(&self) -> Option<Self> {
        match self {
            DualNode::TreeSitter(node) => node.next_sibling().map(DualNode::TreeSitter),
            DualNode::Compact(node) => node.next_sibling().map(DualNode::Compact),
        }
    }
    
    /// Get text from source
    pub fn utf8_text<'a>(&self, source: &'a [u8]) -> Result<&'a str, std::str::Utf8Error> {
        match self {
            DualNode::TreeSitter(node) => node.utf8_text(source),
            DualNode::Compact(node) => node.utf8_text(source),
        }
    }
}

impl fmt::Debug for DualTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DualTree::TreeSitter(_) => write!(f, "DualTree::TreeSitter({} nodes)", self.node_count()),
            DualTree::Compact(_) => write!(f, "DualTree::Compact({} nodes, {} bytes)", 
                                            self.node_count(), self.memory_bytes()),
        }
    }
}

impl<'tree> fmt::Debug for DualNode<'tree> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DualNode({}, {}..{})", self.kind(), self.start_byte(), self.end_byte())
    }
}

/// Configuration for dual representation strategy
#[derive(Debug, Clone)]
pub struct DualRepresentationConfig {
    /// Use compact representation for files larger than this (bytes)
    pub compact_threshold: usize,
    
    /// Always use compact for hot tier
    pub compact_in_hot: bool,
    
    /// Enable automatic promotion to compact
    pub auto_compact: bool,
}

impl Default for DualRepresentationConfig {
    fn default() -> Self {
        Self {
            compact_threshold: 10_000, // Files > 10KB use compact
            compact_in_hot: true,       // Always compact in hot tier
            auto_compact: true,         // Enable auto-compaction
        }
    }
}
