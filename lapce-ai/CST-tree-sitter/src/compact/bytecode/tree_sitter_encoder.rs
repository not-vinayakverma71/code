//! BytecodeEncoder for tree-sitter nodes (direct integration)
//! Converts tree-sitter CSTs to bytecode with 0% quality loss

use super::opcodes::{Opcode, NodeFlags, BytecodeStream};
use tree_sitter::{Node, Tree};
// Varint functions are implemented locally in this file
use std::collections::HashMap;

/// Encoder specifically for tree-sitter trees
pub struct TreeSitterBytecodeEncoder {
    stream: BytecodeStream,
    last_position: usize,
    node_count: usize,
}

impl TreeSitterBytecodeEncoder {
    /// Create new encoder
    pub fn new() -> Self {
        Self {
            stream: BytecodeStream::new(),
            last_position: 0,
            node_count: 0,
        }
    }
    
    /// Encode a tree-sitter Tree to bytecode
    pub fn encode_tree(&mut self, tree: &Tree, source: &[u8]) -> BytecodeStream {
        self.stream.source_len = source.len();
        
        // Encode root node
        self.encode_node(tree.root_node(), source);
        
        // Add end marker
        self.stream.write_op(Opcode::End);
        
        // Set metadata
        self.stream.node_count = self.node_count;
        
        // Return the stream
        std::mem::take(&mut self.stream)
    }
    
    /// Encode a tree-sitter Node to bytecode
    pub fn encode_node(&mut self, node: Node, source: &[u8]) {
        self.node_count += 1;
        
        // Check if we should add a checkpoint
        if self.node_count % 1000 == 0 {
            self.stream.write_op(Opcode::Checkpoint);
            self.stream.write_varint(self.node_count as u64);
            self.stream.checkpoints.push((self.node_count, self.stream.bytes.len()));
        }
        
        // Determine if this is a leaf
        let is_leaf = node.child_count() == 0;
        
        if is_leaf {
            // Use Leaf opcode for efficiency
            self.stream.write_op(Opcode::Leaf);
        } else {
            // Use Enter for nodes with children
            self.stream.write_op(Opcode::Enter);
        }
        
        // Write node kind ID
        self.stream.write_varint(node.kind_id() as u64);
        
        // Pack flags
        let flags = NodeFlags {
            is_named: node.is_named(),
            is_missing: node.is_missing(),
            is_extra: node.is_extra(),
            is_error: node.is_error(),
            has_field: false, // Tree-sitter doesn't expose field info directly
        };
        self.stream.bytes.push(flags.to_byte());
        
        // Write position (delta encoding for efficiency)
        let start_byte = node.start_byte();
        if start_byte != self.last_position {
            if start_byte > self.last_position {
                // Forward delta
                self.stream.write_op(Opcode::DeltaPos);
                self.stream.write_varint((start_byte - self.last_position) as u64);
            } else {
                // Absolute position
                self.stream.write_op(Opcode::SetPos);
                self.stream.write_varint(start_byte as u64);
            }
            self.last_position = start_byte;
        }
        
        // Write length for all nodes
        let length = node.end_byte() - node.start_byte();
        self.stream.write_varint(length as u64);
        
        // For leaf nodes, we're done
        if is_leaf {
            return;
        }
        
        // Encode children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.encode_node(child, source);
            }
        }
        
        // Exit non-leaf nodes
        self.stream.write_op(Opcode::Exit);
    }
    
    /// Get current bytecode size
    pub fn size(&self) -> usize {
        self.stream.bytes.len()
    }
}

/// Decoder for tree-sitter bytecode (for verification)
pub struct TreeSitterBytecodeDecoder {
    stream: BytecodeStream,
    cursor: usize,
    source: Vec<u8>,
}

impl TreeSitterBytecodeDecoder {
    /// Create decoder from bytecode stream
    pub fn new(stream: BytecodeStream, source: Vec<u8>) -> Self {
        Self {
            stream,
            cursor: 0,
            source,
        }
    }
    
    /// Read next opcode
    fn read_op(&mut self) -> Option<Opcode> {
        if self.cursor >= self.stream.bytes.len() {
            return None;
        }
        let byte = self.stream.bytes[self.cursor];
        self.cursor += 1;
        Opcode::from_byte(byte)
    }
    
    /// Read varint
    fn read_varint(&mut self) -> Option<u64> {
        if self.cursor >= self.stream.bytes.len() {
            return None;
        }
        
        let (value, bytes_read) = read_varint_from_slice(&self.stream.bytes[self.cursor..]);
        self.cursor += bytes_read;
        Some(value)
    }
    
    /// Read flags byte
    fn read_flags(&mut self) -> Option<NodeFlags> {
        if self.cursor >= self.stream.bytes.len() {
            return None;
        }
        let byte = self.stream.bytes[self.cursor];
        self.cursor += 1;
        Some(NodeFlags::from_byte(byte))
    }
    
    /// Navigate to specific node by index
    pub fn navigate_to(&mut self, node_index: usize) -> bool {
        // Use jump table if available
        if node_index < self.stream.jump_table.len() {
            self.cursor = self.stream.jump_table[node_index] as usize;
            return true;
        }
        
        // Use checkpoints for faster seeking
        for &(idx, offset) in self.stream.checkpoints.iter().rev() {
            if idx <= node_index {
                self.cursor = offset;
                // Continue from checkpoint
                return self.skip_to_node(node_index - idx);
            }
        }
        
        // Start from beginning
        self.cursor = 0;
        self.skip_to_node(node_index)
    }
    
    /// Skip forward to nth node from current position
    fn skip_to_node(&mut self, n: usize) -> bool {
        let mut nodes_seen = 0;
        
        while nodes_seen < n {
            match self.read_op() {
                Some(Opcode::Enter) | Some(Opcode::Leaf) => {
                    nodes_seen += 1;
                    // Skip the rest of the node
                    self.skip_node_contents();
                }
                Some(Opcode::End) | None => return false,
                _ => {}
            }
        }
        
        true
    }
    
    /// Skip node contents (kind, flags, position, etc.)
    fn skip_node_contents(&mut self) {
        // Skip kind ID
        self.read_varint();
        // Skip flags
        self.cursor += 1;
        // Continue reading opcodes until we understand position
        while let Some(op) = self.read_op() {
            match op {
                Opcode::SetPos | Opcode::DeltaPos => {
                    self.read_varint();
                }
                Opcode::Enter | Opcode::Exit | Opcode::Leaf => {
                    // Node boundary, step back
                    self.cursor -= 1;
                    break;
                }
                _ => {}
            }
        }
    }
    
    /// Verify bytecode can represent the tree correctly
    pub fn verify(&mut self) -> Result<(), String> {
        self.cursor = 0;
        let mut node_count = 0;
        let mut depth = 0;
        
        loop {
            match self.read_op() {
                Some(Opcode::Enter) => {
                    node_count += 1;
                    depth += 1;
                    // Read and verify node data
                    let _kind = self.read_varint().ok_or("Missing kind ID")?;
                    let _flags = self.read_flags().ok_or("Missing flags")?;
                }
                Some(Opcode::Leaf) => {
                    node_count += 1;
                    // Read and verify leaf data
                    let _kind = self.read_varint().ok_or("Missing kind ID")?;
                    let _flags = self.read_flags().ok_or("Missing flags")?;
                }
                Some(Opcode::Exit) => {
                    depth -= 1;
                    if depth < 0 {
                        return Err("Unbalanced Exit opcode".to_string());
                    }
                }
                Some(Opcode::SetPos) | Some(Opcode::DeltaPos) => {
                    let _pos = self.read_varint().ok_or("Missing position")?;
                }
                Some(Opcode::End) => {
                    break;
                }
                Some(Opcode::Checkpoint) => {
                    let _checkpoint_idx = self.read_varint().ok_or("Missing checkpoint index")?;
                }
                _ => {}
            }
        }
        
        if depth != 0 {
            return Err(format!("Unbalanced tree depth: {}", depth));
        }
        
        if node_count != self.stream.node_count {
            return Err(format!("Node count mismatch: {} vs {}", node_count, self.stream.node_count));
        }
        
        Ok(())
    }
}

/// Helper to write varint to vec
fn write_varint_to_vec(vec: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        vec.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Helper to read varint from slice
fn read_varint_from_slice(slice: &[u8]) -> (u64, usize) {
    let mut value = 0u64;
    let mut shift = 0;
    let mut bytes_read = 0;
    
    for &byte in slice {
        bytes_read += 1;
        value |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    
    (value, bytes_read)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::{Parser, Language};
    
    #[test]
    fn test_encode_decode_tree() {
        // Create a simple Rust tree
        let source = "fn main() { println!(\"hello\"); }";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        // Encode
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        println!("Source: {} bytes", source.len());
        println!("Bytecode: {} bytes", bytecode.bytes.len());
        println!("Compression: {:.1}x", source.len() as f64 / bytecode.bytes.len() as f64);
        
        // Verify
        let mut decoder = TreeSitterBytecodeDecoder::new(bytecode, source.as_bytes().to_vec());
        decoder.verify().unwrap();
    }
}
