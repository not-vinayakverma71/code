//! Node interface over compact tree data (simplified)

use super::tree::CompactTree;
use std::fmt;

/// Reference to a node in CompactTree (simplified)
pub struct CompactNode<'tree> {
    tree: &'tree CompactTree,
    index: usize, // Node index
}

impl<'tree> CompactNode<'tree> {
    /// Create new node reference
    pub(crate) fn new(tree: &'tree CompactTree, index: usize) -> Self {
        Self { tree, index }
    }
    
    /// Get node index
    pub fn index(&self) -> usize {
        self.index
    }
    
    /// Get parent node (simplified - returns None)
    pub fn parent(&self) -> Option<Self> {
        None // Simplified - tree structure not maintained
    }
    
    /// Get first child (simplified - returns None)
    pub fn first_child(&self) -> Option<Self> {
        None // Simplified
    }
    
    /// Get next sibling (simplified - returns None)
    pub fn next_sibling(&self) -> Option<Self> {
        None // Simplified
    }
    
    /// Get previous sibling (simplified - returns None)
    pub fn previous_sibling(&self) -> Option<Self> {
        None // Simplified
    }
    
    /// Get k-th child (simplified - returns None)
    pub fn child(&self, _k: usize) -> Option<Self> {
        None // Simplified
    }
    
    /// Count children (simplified - returns 0)
    pub fn child_count(&self) -> usize {
        0 // Simplified
    }
    
    /// Get subtree size (number of nodes in subtree)
    pub fn subtree_size(&self) -> usize {
        let idx = self.index();
        self.tree.subtree_size(idx)
    }
    
    /// Get kind ID
    pub fn kind_id(&self) -> u16 {
        let idx = self.index();
        self.tree.kind_id(idx)
    }
    
    /// Get kind name
    pub fn kind(&self) -> &str {
        let idx = self.index();
        self.tree.kind_name(idx)
    }
    
    /// Check if named
    pub fn is_named(&self) -> bool {
        let idx = self.index();
        self.tree.is_named.get(idx)
    }
    
    /// Check if missing
    pub fn is_missing(&self) -> bool {
        let idx = self.index();
        self.tree.is_missing.get(idx)
    }
    
    /// Check if extra
    pub fn is_extra(&self) -> bool {
        let idx = self.index();
        self.tree.is_extra.get(idx)
    }
    
    /// Check if error
    pub fn is_error(&self) -> bool {
        let idx = self.index();
        self.tree.is_error.get(idx)
    }
    
    /// Get field name
    pub fn field_name(&self) -> Option<&str> {
        let idx = self.index();
        self.tree.field_name(idx)
    }
    
    /// Get start byte
    pub fn start_byte(&self) -> usize {
        let idx = self.index();
        self.tree.start_byte(idx)
    }
    
    /// Get end byte
    pub fn end_byte(&self) -> usize {
        let idx = self.index();
        self.tree.end_byte(idx)
    }
    
    /// Get byte range
    pub fn byte_range(&self) -> std::ops::Range<usize> {
        self.start_byte()..self.end_byte()
    }
    
    /// Get text from source
    pub fn utf8_text<'a>(&self, source: &'a [u8]) -> Result<&'a str, std::str::Utf8Error> {
        std::str::from_utf8(&source[self.byte_range()])
    }
    
    /// Get start position (row, column)
    pub fn start_position(&self) -> Position {
        let byte_pos = self.start_byte();
        self.byte_to_position(byte_pos)
    }
    
    /// Get end position (row, column)
    pub fn end_position(&self) -> Position {
        let byte_pos = self.end_byte();
        self.byte_to_position(byte_pos)
    }
    
    /// Convert byte position to (row, column)
    fn byte_to_position(&self, byte_pos: usize) -> Position {
        let source = self.tree.source();
        let mut row = 0;
        let mut col = 0;
        
        for (i, &byte) in source.iter().enumerate() {
            if i >= byte_pos {
                break;
            }
            if byte == b'\n' {
                row += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        
        Position { row, column: col }
    }
    
    /// Iterate over children
    pub fn children(&self) -> ChildIterator<'tree> {
        ChildIterator {
            tree: self.tree,
            current: self.first_child(),
        }
    }
    
    /// Iterate over named children only
    pub fn named_children(&self) -> NamedChildIterator<'tree> {
        NamedChildIterator {
            inner: self.children(),
        }
    }
    
    /// Walk tree in preorder
    pub fn walk(&self) -> TreeWalker<'tree> {
        TreeWalker::new(*self)
    }
}

/// Position in source (row, column)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

/// Iterator over child nodes
pub struct ChildIterator<'tree> {
    tree: &'tree CompactTree,
    current: Option<CompactNode<'tree>>,
}

impl<'tree> Iterator for ChildIterator<'tree> {
    type Item = CompactNode<'tree>;
    
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.current?;
        self.current = node.next_sibling();
        Some(node)
    }
}

/// Iterator over named children only
pub struct NamedChildIterator<'tree> {
    inner: ChildIterator<'tree>,
}

impl<'tree> Iterator for NamedChildIterator<'tree> {
    type Item = CompactNode<'tree>;
    
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.inner.next() {
            if node.is_named() {
                return Some(node);
            }
        }
        None
    }
}

/// Tree walker for traversal
pub struct TreeWalker<'tree> {
    stack: Vec<CompactNode<'tree>>,
}

impl<'tree> TreeWalker<'tree> {
    fn new(root: CompactNode<'tree>) -> Self {
        Self {
            stack: vec![root],
        }
    }
}

impl<'tree> Iterator for TreeWalker<'tree> {
    type Item = CompactNode<'tree>;
    
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        
        // Add children in reverse order for preorder traversal
        let mut children: Vec<_> = node.children().collect();
        children.reverse();
        self.stack.extend(children);
        
        Some(node)
    }
}

impl<'tree> fmt::Debug for CompactNode<'tree> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CompactNode({}, {}..{})",
               self.kind(),
               self.start_byte(),
               self.end_byte())
    }
}

impl<'tree> fmt::Display for CompactNode<'tree> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(field) = self.field_name() {
            write!(f, "{}: {}", field, self.kind())
        } else {
            write!(f, "{}", self.kind())
        }
    }
}
