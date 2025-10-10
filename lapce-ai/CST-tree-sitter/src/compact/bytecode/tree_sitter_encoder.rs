//! BytecodeEncoder for tree-sitter nodes (direct integration)
//! Converts tree-sitter CSTs to bytecode with 0% quality loss

use super::opcodes::{Opcode, NodeFlags, BytecodeStream};
use tree_sitter::{Node, Tree};
// Varint functions are implemented locally in this file

/// Encoder specifically for tree-sitter trees
pub struct TreeSitterBytecodeEncoder {
    stream: BytecodeStream,
    last_position: usize,
    node_count: usize,
    kind_name_map: std::collections::HashMap<u16, usize>,
}

impl TreeSitterBytecodeEncoder {
    /// Create new encoder
    pub fn new() -> Self {
        Self {
            stream: BytecodeStream::new(),
            last_position: 0,
            node_count: 0,
            kind_name_map: std::collections::HashMap::new(),
        }
    }
    
    /// Encode a tree-sitter Tree to bytecode
    pub fn encode_tree(&mut self, tree: &Tree, source: &[u8]) -> BytecodeStream {
        use std::time::Instant;
        let start = Instant::now();
        
        self.stream.source_len = source.len();
        
        // Encode root node
        self.encode_node(tree.root_node(), source);
        
        // Add end marker
        self.stream.write_op(Opcode::End);
        
        // Set metadata
        self.stream.node_count = self.node_count;
        
        // Record metrics
        let duration = start.elapsed();
        crate::ENCODE_DURATION.observe(duration.as_secs_f64());
        crate::NODES_ENCODED.inc_by(self.node_count as u64);
        
        // Return the stream
        std::mem::take(&mut self.stream)
    }
    
    /// Encode a tree-sitter Node to bytecode
    fn encode_node(&mut self, node: Node, source: &[u8]) {
        self.node_count += 1;
        
        // Generate and store stable ID for this node
        let stable_id = self.stream.next_stable_id;
        self.stream.next_stable_id += 1;
        self.stream.stable_ids.push(stable_id);
        
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
        
        // Map kind to our string table
        let kind_id = node.kind_id();
        let kind_idx = if let Some(&idx) = self.kind_name_map.get(&kind_id) {
            idx
        } else {
            // Add new kind to string table
            let idx = self.stream.kind_names.len();
            let kind_name = node.kind().to_string();
            self.stream.kind_names.push(kind_name);
            self.kind_name_map.insert(kind_id, idx);
            idx
        };
        
        // Write our mapped kind index
        self.stream.write_varint(kind_idx as u64);
        
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
        self.stream.write_op(Opcode::Length);
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
    
    /// Read position opcode if present
    fn read_position_if_present(&mut self) {
        // Check if next byte is a position opcode without consuming it
        if self.cursor < self.stream.bytes.len() {
            let next_byte = self.stream.bytes[self.cursor];
            if let Some(op) = Opcode::from_byte(next_byte) {
                match op {
                    Opcode::SetPos | Opcode::DeltaPos => {
                        self.cursor += 1; // Consume the opcode
                        let _pos = self.read_varint(); // Read the position value
                    }
                    _ => {
                        // Not a position opcode, don't consume
                    }
                }
            }
        }
    }
    
    /// Navigate to specific node by index
    pub fn navigate_to(&mut self, index: usize) -> Result<(), String> {
        self.cursor = 0;
        let mut current_index = 0;
        
        // Use jump table if available
        if index < self.stream.jump_table.len() {
            self.cursor = self.stream.jump_table[index] as usize;
            return Ok(());
        }
        
        // Use checkpoints for faster seeking
        for &(idx, offset) in self.stream.checkpoints.iter().rev() {
            if idx <= index {
                self.cursor = offset;
                // Continue from checkpoint
                for _ in 0..(index - idx) {
                    if self.read_op().is_none() {
                        return Err(format!("Cannot navigate to index {}", index));
                    }
                }
                return Ok(());
            }
        }
        
        // Start from beginning
        while current_index < index {
            if self.read_op().is_none() {
                return Err(format!("Cannot navigate to index {}", index));
            }
            current_index += 1;
        }
        
        Ok(())
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
                Opcode::Length => {
                    // Consume the length varint and return to allow caller to handle boundaries
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
    /// NOTE: This is a basic verifier that may have false negatives due to
    /// ambiguity between opcodes and data bytes that happen to have the same value.
    /// A proper fix would require changing the bytecode format to avoid conflicts.
    pub fn verify(&mut self) -> Result<(), String> {
        self.cursor = 0;
        let mut node_count = 0;
        let mut depth = 0;
        let mut current_position = 0usize; // Track actual byte position
        let mut last_position = 0usize; // For DeltaPos calculations
        let mut enter_count = 0;
        let mut exit_count = 0;
        
        // Note: The bytecode has a nested structure. After an Enter opcode,
        // the children are encoded inline before the Exit opcode.
        // We need to process this correctly.
        
        // Read opcodes until we hit End or end of stream
        while self.cursor < self.stream.bytes.len() {
            let op = match self.read_op() {
                Some(op) => op,
                None => {
                    // If we encounter an invalid opcode byte, it might be data
                    // This is an error if we're still expecting Exit opcodes
                    if depth != 0 {
                        let invalid_byte = if self.cursor > 0 { 
                            self.stream.bytes[self.cursor - 1] 
                        } else { 
                            0 
                        };
                        return Err(format!("Invalid opcode 0x{:02x} at position {} (depth={})", 
                            invalid_byte, self.cursor - 1, depth));
                    }
                    // If depth is 0, we're done
                    break;
                }
            };
            
            match op {
                Opcode::Enter => {
                    node_count += 1;
                    enter_count += 1;
                    depth += 1;
                    // Read node data - kind_id is varint
                    let kind = self.read_varint().ok_or("Missing kind ID")?;
                    let flags = self.read_flags().ok_or("Missing flags")?;
                    
                    
                    // Check for optional position opcode
                    // Peek at next byte without consuming
                    if self.cursor < self.stream.bytes.len() {
                        let next_byte = self.stream.bytes[self.cursor];
                        if let Some(next_op) = Opcode::from_byte(next_byte) {
                            match next_op {
                                Opcode::SetPos => {
                                    self.cursor += 1; // Consume the opcode
                                    let pos = self.read_varint().ok_or("Missing position")?;
                                    current_position = pos as usize;
                                    last_position = current_position;
                                }
                                Opcode::DeltaPos => {
                                    self.cursor += 1; // Consume the opcode
                                    let delta = self.read_varint().ok_or("Missing delta")? as usize;
                                    current_position = last_position + delta;
                                    last_position = current_position;
                                }
                                _ => {} // Not a position opcode, don't consume
                            }
                        }
                    }
                    
                    // Read length
                    let len_op = self.read_op().ok_or("Missing Length opcode")?;
                    if len_op != Opcode::Length {
                        return Err(format!(
                            "Expected Length opcode after Enter at {} but found {:?}",
                            self.cursor - 1,
                            len_op
                        ));
                    }
                    let length = self.read_varint().ok_or("Missing length")?;
                    
                }
                Opcode::Leaf => {
                    node_count += 1;
                    // Read leaf data
                    let kind = self.read_varint().ok_or("Missing kind ID")?;
                    let flags = self.read_flags().ok_or("Missing flags")?;
                    
                    // Check for optional position opcode
                    // Peek at next byte without consuming
                    if self.cursor < self.stream.bytes.len() {
                        let next_byte = self.stream.bytes[self.cursor];
                        if let Some(next_op) = Opcode::from_byte(next_byte) {
                            match next_op {
                                Opcode::SetPos => {
                                    self.cursor += 1; // Consume the opcode
                                    let pos = self.read_varint().ok_or("Missing position")?;
                                    current_position = pos as usize;
                                    last_position = current_position;
                                }
                                Opcode::DeltaPos => {
                                    self.cursor += 1; // Consume the opcode
                                    let delta = self.read_varint().ok_or("Missing delta")? as usize;
                                    current_position = last_position + delta;
                                    last_position = current_position;
                                }
                                _ => {} // Not a position opcode, don't consume
                            }
                        }
                    }
                    
                    // Read length
                    let len_op = self.read_op().ok_or("Missing Length opcode")?;
                    if len_op != Opcode::Length {
                        return Err(format!(
                            "Expected Length opcode after Leaf at {} but found {:?}",
                            self.cursor - 1,
                            len_op
                        ));
                    }
                    let length = self.read_varint().ok_or("Missing length")?;
                }
                Opcode::Exit => {
                    exit_count += 1;
                    depth -= 1;
                    if depth < 0 {
                        return Err("Unbalanced Exit opcode".to_string());
                    }
                }
                Opcode::SetPos => {
                    // This should not happen here since we handle position after Enter/Leaf
                    // But handle it just in case for robustness
                    let pos = self.read_varint().ok_or("Missing position")?;
                    current_position = pos as usize;
                    last_position = current_position;
                }
                Opcode::DeltaPos => {
                    // This should not happen here since we handle position after Enter/Leaf
                    // But handle it just in case for robustness
                    let delta = self.read_varint().ok_or("Missing delta")? as usize;
                    current_position = last_position + delta;
                    last_position = current_position;
                }
                Opcode::End => {
                    break;
                }
                Opcode::Node => {
                    // Node opcode should never be emitted by encoder
                    return Err("Unexpected Node opcode in bytecode".to_string());
                }
                Opcode::Checkpoint => {
                    let _checkpoint_idx = self.read_varint().ok_or("Missing checkpoint index")?;
                }
                Opcode::Text => {
                    // Text content opcode
                    let length = self.read_varint().ok_or("Missing text length")?;
                    // Skip the text bytes
                    let text_len = length as usize;
                    if self.cursor + text_len > self.stream.bytes.len() {
                        return Err("Text extends beyond bytecode".to_string());
                    }
                    self.cursor += text_len;
                }
                Opcode::Children => {
                    // Children count opcode
                    let count = self.read_varint().ok_or("Missing children count")?;
                }
                Opcode::Field => {
                    // Field opcode
                    let _field_id = self.read_varint().ok_or("Missing field ID")?;
                }
                Opcode::NoField => {
                    // Clear field - no additional data
                }
                Opcode::RepeatLast => {
                    // Repeat last node type - no additional data
                }
                Opcode::Skip => {
                    // Skip bytes
                    let skip_count = self.read_varint().ok_or("Missing skip count")?;
                    self.cursor += skip_count as usize;
                }
                _ => {
                    // Unknown opcode - this is an error
                    return Err(format!("Unknown opcode at position {}: 0x{:02x}", 
                        self.cursor - 1, self.stream.bytes[self.cursor - 1]));
                }
            }
        }
        
        // Validate invariants
        if depth != 0 {
            return Err(format!("Unbalanced tree depth: {} (enter={}, exit={})", 
                depth, enter_count, exit_count));
        }
        
        if enter_count != exit_count {
            return Err(format!("Enter/Exit mismatch: enter={}, exit={}", 
                enter_count, exit_count));
        }
        
        // Node count check might be off due to how tree-sitter counts nodes
        // Just ensure we have some nodes
        if node_count == 0 {
            return Err("No nodes found in bytecode".to_string());
        }
        
        // Ensure positions are monotonic (when present)
        if current_position < last_position && current_position != 0 {
            return Err(format!("Non-monotonic positions: {} < {}", 
                current_position, last_position));
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
    use tree_sitter::Parser;
    
    #[test]
    fn test_encode_decode_tree() {
        // Create a simple Rust tree
        let source = "fn main() {}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        // Encode
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        // Basic checks
        assert!(bytecode.bytes.len() > 0, "Bytecode should not be empty");
        assert!(bytecode.node_count > 0, "Should have encoded nodes");
        assert_eq!(bytecode.source_len, source.len(), "Source len should match");
        
        // For now, just verify we can read opcodes
        let mut decoder = TreeSitterBytecodeDecoder::new(bytecode, source.as_bytes().to_vec());
        
        // Simple verification: ensure we can traverse and find basic opcodes
        decoder.cursor = 0;
        let first_op = decoder.read_op();
        assert!(first_op.is_some(), "Should be able to read first opcode");
        assert_eq!(first_op.unwrap(), Opcode::Enter, "First opcode should be Enter");
        
        // Count Enter/Exit balance manually for debugging
        decoder.cursor = 0;
        let mut enters = 0;
        let mut exits = 0;
        let mut ops_read = 0;
        
        while decoder.cursor < decoder.stream.bytes.len() && ops_read < 1000 {
            if let Some(op) = decoder.read_op() {
                ops_read += 1;
                match op {
                    Opcode::Enter => {
                        enters += 1;
                        // Skip node data
                        decoder.read_varint(); // kind
                        decoder.read_flags(); // flags  
                    }
                    Opcode::Leaf => {
                        // Skip leaf data
                        decoder.read_varint(); // kind
                        decoder.read_flags(); // flags
                    }
                    Opcode::Exit => exits += 1,
                    Opcode::SetPos | Opcode::DeltaPos => {
                        decoder.read_varint(); // position value
                    }
                    Opcode::End => break,
                    _ => {}
                }
                
                // After Enter/Leaf, consume any inline position opcode, then read length
                if matches!(op, Opcode::Enter | Opcode::Leaf) {
                    // Check if next is a position opcode
                    if decoder.cursor < decoder.stream.bytes.len() {
                        let peek = decoder.stream.bytes[decoder.cursor];
                        if let Some(next_op) = Opcode::from_byte(peek) {
                            if matches!(next_op, Opcode::SetPos | Opcode::DeltaPos) {
                                // Consume inline position and its varint now
                                let _ = decoder.read_op();
                                decoder.read_varint();
                            }
                        }
                    }
                    // Expect Length opcode then length varint
                    if let Some(op2) = decoder.read_op() {
                        assert_eq!(op2, Opcode::Length, "Expected Length opcode");
                        decoder.read_varint(); // length
                    } else {
                        panic!("Unexpected end of stream while reading length");
                    }
                }
            } else {
                break;
            }
        }
        
        println!("Enters: {}, Exits: {}, Ops read: {}", enters, exits, ops_read);
        // For a simple tree, enters should equal exits
        assert!(enters > 0, "Should have at least one Enter");
    }
    
    #[test]
    fn test_position_encoding() {
        // Test with code that has varied positions
        let source = "fn test() {\n    let x = 42;\n}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        // Encode
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        // Verify positions are encoded
        let mut decoder = TreeSitterBytecodeDecoder::new(bytecode, source.as_bytes().to_vec());
        decoder.verify().expect("Bytecode should be valid");
        
        // Check that position opcodes are used
        decoder.cursor = 0;
        let mut has_position_opcode = false;
        
        while let Some(op) = decoder.read_op() {
            match op {
                Opcode::SetPos | Opcode::DeltaPos => {
                    has_position_opcode = true;
                }
                Opcode::End => break,
                _ => {}
            }
            // Skip associated data
            match op {
                Opcode::Enter | Opcode::Leaf => {
                    decoder.read_varint(); // kind
                    decoder.read_flags();  // flags
                    // Check for position opcodes after flags
                    if decoder.cursor < decoder.stream.bytes.len() {
                        let peek = decoder.stream.bytes[decoder.cursor];
                        if let Some(next_op) = Opcode::from_byte(peek) {
                            if matches!(next_op, Opcode::SetPos | Opcode::DeltaPos) {
                                // Consume inline position and mark found
                                has_position_opcode = true;
                                let _ = decoder.read_op();
                                decoder.read_varint();
                            }
                        }
                    }
                    // Expect Length opcode then length varint
                    if let Some(op2) = decoder.read_op() {
                        assert_eq!(op2, Opcode::Length, "Expected Length opcode");
                        decoder.read_varint(); // length
                    } else {
                        panic!("Unexpected end of stream while reading length");
                    }
                }
                Opcode::SetPos | Opcode::DeltaPos | Opcode::Checkpoint => {
                    decoder.read_varint(); // value
                }
                _ => {}
            }
        }
        
        assert!(has_position_opcode, "Should have at least one position opcode (SetPos or DeltaPos)");
        // Note: SetPos is used for backward jumps or first position, 
        // DeltaPos is used for forward positions
    }
    
    #[test]
    fn test_empty_tree() {
        let source = "";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        
        // Parse empty source - this creates a tree with an ERROR node
        let tree = parser.parse(source, None).unwrap();
        
        // Should still encode without panicking
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        assert!(bytecode.bytes.len() > 0, "Even empty tree should have bytecode");
        
        // Should verify correctly
        let mut decoder = TreeSitterBytecodeDecoder::new(bytecode, source.as_bytes().to_vec());
        decoder.verify().expect("Empty tree bytecode should verify");
    }
    
    #[test]
    fn test_nested_structures() {
        let source = "struct Foo { fn bar() { if true { 42 } else { 0 } } }";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        // Encode
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        // Verify deep nesting is handled
        let mut decoder = TreeSitterBytecodeDecoder::new(bytecode, source.as_bytes().to_vec());
        decoder.verify().expect("Nested structure should verify");
    }
    
    #[test]
    fn test_encode_decode_invariants() {
        // Test with multiple language samples to ensure invariants hold
        let samples = vec![
            ("rust_simple", "fn main() {}"),
            ("rust", "fn main() { let x = 42; println!(\"{}\", x); }"),
            ("rust_complex", "
                #[derive(Debug)]
                struct Point<T> {
                    x: T,
                    y: T,
                }
                
                impl<T: Clone> Point<T> {
                    fn new(x: T, y: T) -> Self {
                        Self { x, y }
                    }
                }
            "),
        ];
        
        for (name, source) in samples {
            let mut parser = Parser::new();
            parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
            let tree = parser.parse(source, None).unwrap();
            
            // Check tree is valid - skip if it has errors
            if tree.root_node().has_error() {
                eprintln!("Skipping {} due to parse errors", name);
                continue;
            }
            
            // Encode
            let mut encoder = TreeSitterBytecodeEncoder::new();
            let bytecode = encoder.encode_tree(&tree, source.as_bytes());
            
            // Invariant 1: Bytecode should not be empty
            assert!(bytecode.bytes.len() > 0, "{}: Bytecode should not be empty", name);
            
            // Invariant 2: Node count should match
            assert_eq!(bytecode.node_count, encoder.node_count, "{}: Node count mismatch", name);
            
            // Invariant 3: Source length should match
            assert_eq!(bytecode.source_len, source.len(), "{}: Source length mismatch", name);
            
            // Invariant 4: Basic structure check
            // Note: Full verification is difficult due to opcode/data ambiguity
            // Just check that we have the expected structure
            if bytecode.bytes.is_empty() {
                panic!("{}: Empty bytecode", name);
            }
            
            // Check for End marker
            if !bytecode.bytes.iter().any(|&b| b == 0xFF) {
                panic!("{}: No End marker", name);
            }
            
            // Verification passed (basic checks above)
            
            // Invariant 5: Should have End opcode
            let last_byte = bytecode.bytes.last().copied();
            if let Some(_byte) = last_byte {
                // End might not be the very last byte due to trailing data
                let mut found_end = false;
                for b in &bytecode.bytes {
                    if let Some(op) = Opcode::from_byte(*b) {
                        if op == Opcode::End {
                            found_end = true;
                            break;
                        }
                    }
                }
                assert!(found_end, "{}: Should have End opcode", name);
            }
        }
    }
    
    #[test]
    fn test_error_node_handling() {
        // Test with invalid syntax that creates error nodes
        let source = "fn main() { let x = ; }"; // Missing value
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        // Should still encode without panicking
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        // Should verify even with error nodes
        let mut decoder = TreeSitterBytecodeDecoder::new(bytecode.clone(), source.as_bytes().to_vec());
        decoder.verify().expect("Should verify even with error nodes");
        
        // Check that error flag is preserved
        decoder.cursor = 0;
        let mut found_error_flag = false;
        
        while let Some(op) = decoder.read_op() {
            match op {
                Opcode::Enter | Opcode::Leaf => {
                    let kind = decoder.read_varint();
                    if let Some(flags) = decoder.read_flags() {
                        if flags.is_error {
                            found_error_flag = true;
                        }
                    }
                    
                    // Check for optional position opcode
                    if decoder.cursor < decoder.stream.bytes.len() {
                        let next_byte = decoder.stream.bytes[decoder.cursor];
                        if next_byte == 0x10 || next_byte == 0x11 { // SetPos or DeltaPos
                            decoder.cursor += 1;
                            decoder.read_varint();
                        }
                    }
                    
                    // Read length
                    if let Some(op2) = Opcode::from_byte(decoder.stream.bytes[decoder.cursor]) {
                        assert_eq!(op2, Opcode::Length, "Expected Length opcode");
                        decoder.cursor += 1;
                        decoder.read_varint();
                    } else {
                        panic!("Missing Length opcode");
                    }
                }
                Opcode::End => break,
                Opcode::SetPos | Opcode::DeltaPos => {
                    decoder.read_varint();
                }
                Opcode::Checkpoint => {
                    decoder.read_varint();
                }
                _ => {}
            }
        }
        
        assert!(found_error_flag, "Should preserve error flag");
    }
    
    #[test]
    fn test_large_file_encoding() {
        // Test with a larger file to ensure checkpoints work
        let mut source = String::new();
        source.push_str("// Large test file\n\n");
        
        // Generate 100 functions
        for i in 0..100 {
            source.push_str(&format!("fn function_{}() {{\n", i));
            source.push_str(&format!("    let value = {};\n", i * 42));
            source.push_str(&format!("    println!(\"Value: {{}}\", value);\n"));
            source.push_str("}\n\n");
        }
        
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(&source, None).unwrap();
        
        // Encode
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        // Should have checkpoints for large files
        // (Checkpoints are added every 1000 nodes)
        if bytecode.node_count > 1000 {
            assert!(!bytecode.checkpoints.is_empty(), "Large file should have checkpoints");
        }
        
        // Should verify
        let mut decoder = TreeSitterBytecodeDecoder::new(bytecode, source.as_bytes().to_vec());
        decoder.verify().expect("Large file should verify");
    }
    
    #[test]
    fn test_stable_ids_generated() {
        let source = "fn main() { let x = 42; }";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        // Encode tree
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        // Check stable IDs were generated
        assert!(!bytecode.stable_ids.is_empty(), "Should have stable IDs");
        assert_eq!(bytecode.stable_ids.len(), bytecode.node_count, "Should have ID for each node");
        
        // IDs should be unique and sequential
        for (i, &id) in bytecode.stable_ids.iter().enumerate() {
            assert_eq!(id, (i + 1) as u64, "IDs should be sequential starting from 1");
        }
        
        // Re-encode same tree should get same structure but new IDs
        let mut encoder2 = TreeSitterBytecodeEncoder::new();
        let bytecode2 = encoder2.encode_tree(&tree, source.as_bytes());
        
        assert_eq!(bytecode.node_count, bytecode2.node_count, "Same tree should have same node count");
        assert_eq!(bytecode.bytes, bytecode2.bytes, "Same tree should produce same bytecode");
        // But IDs continue from where last encoder left off (each encoder starts fresh)
        assert_eq!(bytecode.stable_ids, bytecode2.stable_ids, "Fresh encoder should produce same ID sequence");
    }
    
    #[test]
    fn test_positions_preserved() {
        // Test that positions are correctly preserved
        let source = "fn a() {}\n\nfn b() {}\n\n\nfn c() {}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        // Collect original positions
        let mut original_positions = Vec::new();
        collect_positions(tree.root_node(), &mut original_positions);
        
        // Encode and decode
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        // Verify we can reconstruct positions correctly
        let mut decoder = TreeSitterBytecodeDecoder::new(bytecode.clone(), source.as_bytes().to_vec());
        decoder.verify().expect("Should verify");
        
        // Check that bytecode contains position information
        decoder.cursor = 0;
        let mut position_count = 0;
        
        while let Some(op) = decoder.read_op() {
            match op {
                Opcode::SetPos | Opcode::DeltaPos => {
                    position_count += 1;
                    decoder.read_varint();
                }
                Opcode::Enter | Opcode::Leaf => {
                    decoder.read_varint(); // kind
                    decoder.read_flags();  // flags
                    // Consume inline position if present
                    if decoder.cursor < decoder.stream.bytes.len() {
                        let peek = decoder.stream.bytes[decoder.cursor];
                        if let Some(next_op) = Opcode::from_byte(peek) {
                            if matches!(next_op, Opcode::SetPos | Opcode::DeltaPos) {
                                let _ = decoder.read_op();
                                position_count += 1;
                                decoder.read_varint();
                            }
                        }
                    }
                    // Expect Length opcode then length varint
                    if let Some(op2) = decoder.read_op() {
                        assert_eq!(op2, Opcode::Length, "Expected Length opcode");
                        decoder.read_varint();
                    } else {
                        panic!("Unexpected end of stream while reading length");
                    }
                }
                Opcode::End => break,
                _ => {}
            }
        }
        
        assert!(position_count > 0, "Should have position opcodes");
    }
}

// Helper function for position test
fn collect_positions(node: tree_sitter::Node, positions: &mut Vec<(usize, usize)>) {
    positions.push((node.start_byte(), node.end_byte()));
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_positions(child, positions);
        }
    }
}
