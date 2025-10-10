//! Serialization and deserialization for CSTs

use tree_sitter::{Tree, Node};
use bytes::Bytes;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct CodecError {
    message: String,
}

impl CodecError {
    fn new(msg: &str) -> Self {
        CodecError {
            message: msg.to_string(),
        }
    }
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for CodecError {}

type Result<T> = std::result::Result<T, CodecError>;

/// Serialize a tree and its source to bytes
pub fn serialize_tree(tree: &Tree, source: &[u8]) -> Result<Vec<u8>> {
    let mut serialized = Vec::new();
    
    // Version header for future compatibility
    serialized.push(1u8); // Version 1
    
    // Store source length (8 bytes)
    let source_len = source.len() as u64;
    serialized.extend_from_slice(&source_len.to_le_bytes());
    
    // Store source
    serialized.extend_from_slice(source);
    
    // Serialize tree structure recursively
    serialize_node_recursive(tree.root_node(), &mut serialized)?;
    
    Ok(serialized)
}

fn serialize_node_recursive(node: Node, output: &mut Vec<u8>) -> Result<()> {
    // Node kind ID (2 bytes)
    output.extend_from_slice(&node.kind_id().to_le_bytes());
    
    // Is named (1 byte)
    output.push(if node.is_named() { 1 } else { 0 });
    
    // Start byte (4 bytes)
    output.extend_from_slice(&(node.start_byte() as u32).to_le_bytes());
    
    // End byte (4 bytes)
    output.extend_from_slice(&(node.end_byte() as u32).to_le_bytes());
    
    // Start position (row, column) (4 + 4 bytes)
    let start = node.start_position();
    output.extend_from_slice(&(start.row as u32).to_le_bytes());
    output.extend_from_slice(&(start.column as u32).to_le_bytes());
    
    // End position (row, column) (4 + 4 bytes)
    let end = node.end_position();
    output.extend_from_slice(&(end.row as u32).to_le_bytes());
    output.extend_from_slice(&(end.column as u32).to_le_bytes());
    
    // Child count (2 bytes)
    let child_count = node.child_count();
    output.extend_from_slice(&child_count.to_le_bytes());
    
    // Serialize children
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            serialize_node_recursive(child, output)?;
        } else {
            // Mark missing child
            output.push(0xFF);
            output.push(0xFF);
        }
    }
    
    Ok(())
}

/// Deserialize a tree and source from bytes
/// 
/// Note: This returns the source and a placeholder for the tree.
/// Tree-sitter doesn't support reconstructing trees from serialized data directly,
/// so we need to reparse. However, we can verify the structure matches.
pub fn deserialize_tree(data: &[u8]) -> Result<(Tree, Bytes)> {
    if data.is_empty() {
        return Err(CodecError::new("Empty data"));
    }
    
    let mut cursor = 0;
    
    // Read version
    let version = data[cursor];
    cursor += 1;
    
    if version != 1 {
        return Err(CodecError::new(&format!("Unsupported version: {}", version)));
    }
    
    // Read source length
    if cursor + 8 > data.len() {
        return Err(CodecError::new("Invalid data: missing source length"));
    }
    let source_len = u64::from_le_bytes(data[cursor..cursor+8].try_into().unwrap());
    cursor += 8;
    
    // Read source
    let source_len_usize = source_len as usize;
    if cursor + source_len_usize > data.len() {
        return Err(CodecError::new("Invalid data: source truncated"));
    }
    let source = &data[cursor..cursor + source_len_usize];
    cursor += source_len_usize;
    
    // For now, we can't reconstruct the tree directly from serialized data
    // because tree-sitter doesn't expose this functionality.
    // Instead, we'll need to reparse the source.
    // This is a limitation of tree-sitter's API.
    
    // We could verify the structure matches, but for performance,
    // we'll just return a marker that forces reparsing.
    
    Err(CodecError::new("Tree deserialization requires reparsing - this is a tree-sitter limitation"))
}

/// Alternative: Store only the source and reparse on demand
pub fn serialize_source_only(source: &[u8]) -> Vec<u8> {
    let mut serialized = Vec::with_capacity(9 + source.len());
    
    // Version header
    serialized.push(2u8); // Version 2: source-only
    
    // Source length
    let source_len = source.len() as u64;
    serialized.extend_from_slice(&source_len.to_le_bytes());
    
    // Source
    serialized.extend_from_slice(source);
    
    serialized
}

pub fn deserialize_source_only(data: &[u8]) -> Result<Bytes> {
    if data.is_empty() {
        return Err(CodecError::new("Empty data"));
    }
    
    let mut cursor = 0;
    
    // Read version
    let version = data[cursor];
    cursor += 1;
    
    if version != 2 {
        return Err(CodecError::new("Not a source-only format"));
    }
    
    // Read source length
    if cursor + 8 > data.len() {
        return Err(CodecError::new("Invalid data: missing source length"));
    }
    let source_len = u64::from_le_bytes(data[cursor..cursor+8].try_into().unwrap());
    cursor += 8;
    
    // Read source
    let source_len_usize = source_len as usize;
    if cursor + source_len_usize > data.len() {
        return Err(CodecError::new("Invalid data: source truncated"));
    }
    
    Ok(Bytes::copy_from_slice(&data[cursor..cursor + source_len_usize]))
}
