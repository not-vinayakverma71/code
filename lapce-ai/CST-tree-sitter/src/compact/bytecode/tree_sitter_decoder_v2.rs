//! Fixed bytecode decoder that handles the ambiguity between opcodes and data

use super::{SegmentedBytecodeStream, Opcode, NodeFlags};

pub struct BytecodeDecoderV2 {
    stream: SegmentedBytecodeStream,
    cursor: usize,
}

impl BytecodeDecoderV2 {
    pub fn new(stream: SegmentedBytecodeStream) -> Self {
        Self { stream, cursor: 0 }
    }
    
    /// Verify bytecode with proper state tracking
    pub fn verify(&mut self) -> Result<(), String> {
        self.cursor = 0;
        let mut stack = Vec::new(); // Track Enter nodes that need Exit
        let mut node_count = 0;
        
        while self.cursor < self.stream.bytes.len() {
            // Read the next byte
            let byte = self.stream.bytes[self.cursor];
            
            // Check if it's the End marker
            if byte == 0xFF {
                break;
            }
            
            // Try to interpret as opcode
            match Opcode::from_byte(byte) {
                Some(Opcode::Enter) => {
                    self.cursor += 1;
                    node_count += 1;
                    
                    // Read Enter node data
                    let _kind = self.read_varint().ok_or("Missing kind ID")?;
                    let _flags = self.read_byte().ok_or("Missing flags")?;
                    self.skip_position_if_present();
                    let _length = self.read_varint().ok_or("Missing length")?;
                    
                    // Push to stack - we expect an Exit later
                    stack.push(node_count);
                }
                Some(Opcode::Leaf) => {
                    self.cursor += 1;
                    node_count += 1;
                    
                    // Read Leaf node data
                    let _kind = self.read_varint().ok_or("Missing kind ID")?;
                    let _flags = self.read_byte().ok_or("Missing flags")?;
                    self.skip_position_if_present();
                    let _length = self.read_varint().ok_or("Missing length")?;
                    
                    // Leaf nodes don't need Exit
                }
                Some(Opcode::Exit) => {
                    self.cursor += 1;
                    
                    // Pop from stack
                    if stack.is_empty() {
                        return Err("Unexpected Exit opcode".to_string());
                    }
                    stack.pop();
                }
                Some(Opcode::SetPos) | Some(Opcode::DeltaPos) => {
                    // These should only appear after flags, not as top-level opcodes
                    // This indicates a misalignment
                    return Err(format!("Unexpected position opcode at {}", self.cursor));
                }
                Some(Opcode::End) => {
                    break;
                }
                _ => {
                    // Unknown opcode or data byte misinterpreted as opcode
                    // This is the ambiguity we need to handle
                    
                    // If we have unmatched Enter nodes, this is an error
                    if !stack.is_empty() {
                        return Err(format!(
                            "Invalid byte 0x{:02x} at position {} (expecting Exit for {} Enter nodes)",
                            byte, self.cursor, stack.len()
                        ));
                    }
                    
                    // Otherwise, we might be done
                    break;
                }
            }
        }
        
        // Check for unmatched Enter nodes
        if !stack.is_empty() {
            return Err(format!("Unmatched Enter nodes: {}", stack.len()));
        }
        
        // Verify we processed some nodes
        if node_count == 0 {
            return Err("No nodes found in bytecode".to_string());
        }
        
        Ok(())
    }
    
    fn read_byte(&mut self) -> Option<u8> {
        if self.cursor >= self.stream.bytes.len() {
            return None;
        }
        let byte = self.stream.bytes[self.cursor];
        self.cursor += 1;
        Some(byte)
    }
    
    fn read_varint(&mut self) -> Option<u64> {
        let mut value = 0u64;
        let mut shift = 0;
        
        loop {
            let byte = self.read_byte()?;
            value |= ((byte & 0x7F) as u64) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        
        Some(value)
    }
    
    fn skip_position_if_present(&mut self) {
        // Check if next byte is a position opcode
        if self.cursor >= self.stream.bytes.len() {
            return;
        }
        
        let byte = self.stream.bytes[self.cursor];
        if byte == 0x10 || byte == 0x11 { // SetPos or DeltaPos
            self.cursor += 1; // Skip opcode
            let _ = self.read_varint(); // Skip position value
        }
    }
}
