//! Bytecode decoder - reconstructs tree structure from bytecode
//! Validates every operation to ensure 0% quality loss

use super::opcodes::{Opcode, NodeFlags, BytecodeStream, BytecodeReader};
use std::collections::HashMap;

/// Decoded node representation
#[derive(Debug, Clone)]
pub struct DecodedNode {
    pub kind_name: String,
    pub kind_id: u16,
    pub field_name: Option<String>,
    pub is_named: bool,
    pub is_missing: bool,
    pub is_extra: bool,
    pub is_error: bool,
    pub start_byte: usize,
    pub end_byte: usize,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
}

/// Bytecode decoder
pub struct BytecodeDecoder {
    nodes: Vec<DecodedNode>,
    current_position: usize,
    node_stack: Vec<usize>,
    field_stack: Vec<Option<u8>>,
    last_kind: Option<u16>,
}

impl BytecodeDecoder {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            current_position: 0,
            node_stack: Vec::new(),
            field_stack: Vec::new(),
            last_kind: None,
        }
    }
    
    /// Decode bytecode stream to nodes
    pub fn decode(&mut self, stream: &BytecodeStream) -> Result<Vec<DecodedNode>, String> {
        let mut reader = BytecodeReader::new(&stream.bytes);
        
        while !reader.is_at_end() {
            let op = reader.read_op()
                .ok_or_else(|| "Failed to read opcode".to_string())?;
            
            match op {
                Opcode::Enter => self.handle_enter(&mut reader, stream)?,
                Opcode::Exit => self.handle_exit()?,
                Opcode::Leaf => self.handle_leaf(&mut reader, stream)?,
                Opcode::SetPos => self.handle_set_pos(&mut reader)?,
                Opcode::DeltaPos => self.handle_delta_pos(&mut reader)?,
                Opcode::Field => self.handle_field(&mut reader)?,
                Opcode::NoField => self.handle_no_field()?,
                Opcode::RepeatLast => self.handle_repeat(&mut reader, stream)?,
                Opcode::Skip => self.handle_skip(&mut reader)?,
                Opcode::Checkpoint => self.handle_checkpoint(&mut reader)?,
                Opcode::End => break,
            }
        }
        
        // Validate final state
        if !self.node_stack.is_empty() {
            return Err(format!("Unclosed nodes: {} remaining", self.node_stack.len()));
        }
        
        Ok(std::mem::take(&mut self.nodes))
    }
    
    /// Handle Enter opcode
    fn handle_enter(&mut self, reader: &mut BytecodeReader, stream: &BytecodeStream) -> Result<(), String> {
        let kind_id = reader.read_varint()
            .ok_or("Failed to read kind_id")? as u16;
        let flags_byte = reader.read_byte()
            .ok_or("Failed to read flags")?;
        
        let flags = NodeFlags::from_byte(flags_byte);
        let kind_name = stream.kind_names.get(kind_id as usize)
            .ok_or_else(|| format!("Invalid kind_id: {}", kind_id))?
            .clone();
        
        let field_name = if flags.has_field {
            self.field_stack.pop().flatten().map(|id| {
                stream.field_names.get(id as usize)
                    .cloned()
                    .unwrap_or_else(|| format!("field_{}", id))
            })
        } else {
            None
        };
        
        let node_idx = self.nodes.len();
        self.nodes.push(DecodedNode {
            kind_name,
            kind_id,
            field_name,
            is_named: flags.is_named,
            is_missing: flags.is_missing,
            is_extra: flags.is_extra,
            is_error: flags.is_error,
            start_byte: self.current_position,
            end_byte: 0, // Will be set on exit
            children: Vec::new(),
            parent: self.node_stack.last().cloned(),
        });
        
        // Add to parent's children
        if let Some(parent_idx) = self.node_stack.last() {
            self.nodes[*parent_idx].children.push(node_idx);
        }
        
        self.node_stack.push(node_idx);
        self.last_kind = Some(kind_id);
        
        Ok(())
    }
    
    /// Handle Exit opcode
    fn handle_exit(&mut self) -> Result<(), String> {
        let node_idx = self.node_stack.pop()
            .ok_or("Exit without matching Enter")?;
        
        self.nodes[node_idx].end_byte = self.current_position;
        
        Ok(())
    }
    
    /// Handle Leaf opcode
    fn handle_leaf(&mut self, reader: &mut BytecodeReader, stream: &BytecodeStream) -> Result<(), String> {
        let kind_id = reader.read_varint()
            .ok_or("Failed to read kind_id")? as u16;
        let flags_byte = reader.read_byte()
            .ok_or("Failed to read flags")?;
        let length = reader.read_varint()
            .ok_or("Failed to read length")? as usize;
        
        let flags = NodeFlags::from_byte(flags_byte);
        let kind_name = stream.kind_names.get(kind_id as usize)
            .ok_or_else(|| format!("Invalid kind_id: {}", kind_id))?
            .clone();
        
        let field_name = if flags.has_field {
            self.field_stack.pop().flatten().map(|id| {
                stream.field_names.get(id as usize)
                    .cloned()
                    .unwrap_or_else(|| format!("field_{}", id))
            })
        } else {
            None
        };
        
        let node_idx = self.nodes.len();
        self.nodes.push(DecodedNode {
            kind_name,
            kind_id,
            field_name,
            is_named: flags.is_named,
            is_missing: flags.is_missing,
            is_extra: flags.is_extra,
            is_error: flags.is_error,
            start_byte: self.current_position,
            end_byte: self.current_position + length,
            children: Vec::new(),
            parent: self.node_stack.last().cloned(),
        });
        
        // Add to parent's children
        if let Some(parent_idx) = self.node_stack.last() {
            self.nodes[*parent_idx].children.push(node_idx);
        }
        
        self.current_position += length;
        self.last_kind = Some(kind_id);
        
        Ok(())
    }
    
    /// Handle SetPos opcode
    fn handle_set_pos(&mut self, reader: &mut BytecodeReader) -> Result<(), String> {
        self.current_position = reader.read_varint()
            .ok_or("Failed to read position")? as usize;
        Ok(())
    }
    
    /// Handle DeltaPos opcode
    fn handle_delta_pos(&mut self, reader: &mut BytecodeReader) -> Result<(), String> {
        let delta = reader.read_signed_varint()
            .ok_or("Failed to read delta")?;
        
        if delta < 0 && (-delta as usize) > self.current_position {
            return Err(format!("Invalid negative delta: {} at position {}", delta, self.current_position));
        }
        
        self.current_position = (self.current_position as i64 + delta) as usize;
        Ok(())
    }
    
    /// Handle Field opcode
    fn handle_field(&mut self, reader: &mut BytecodeReader) -> Result<(), String> {
        let field_id = reader.read_byte()
            .ok_or("Failed to read field_id")?;
        self.field_stack.push(Some(field_id));
        Ok(())
    }
    
    /// Handle NoField opcode
    fn handle_no_field(&mut self) -> Result<(), String> {
        self.field_stack.push(None);
        Ok(())
    }
    
    /// Handle RepeatLast opcode
    fn handle_repeat(&mut self, reader: &mut BytecodeReader, stream: &BytecodeStream) -> Result<(), String> {
        let length = reader.read_varint()
            .ok_or("Failed to read length")? as usize;
        
        let kind_id = self.last_kind
            .ok_or("RepeatLast without previous node")?;
        
        if length == 0 {
            // Enter node (length 0 means internal node)
            let kind_name = stream.kind_names.get(kind_id as usize)
                .ok_or_else(|| format!("Invalid kind_id: {}", kind_id))?
                .clone();
            
            let node_idx = self.nodes.len();
            self.nodes.push(DecodedNode {
                kind_name,
                kind_id,
                field_name: None, // Repeat doesn't have field
                is_named: true,   // Assume named for repeat
                is_missing: false,
                is_extra: false,
                is_error: false,
                start_byte: self.current_position,
                end_byte: 0,
                children: Vec::new(),
                parent: self.node_stack.last().cloned(),
            });
            
            if let Some(parent_idx) = self.node_stack.last() {
                self.nodes[*parent_idx].children.push(node_idx);
            }
            
            self.node_stack.push(node_idx);
        } else {
            // Leaf node
            let kind_name = stream.kind_names.get(kind_id as usize)
                .ok_or_else(|| format!("Invalid kind_id: {}", kind_id))?
                .clone();
            
            let node_idx = self.nodes.len();
            self.nodes.push(DecodedNode {
                kind_name,
                kind_id,
                field_name: None,
                is_named: true,
                is_missing: false,
                is_extra: false,
                is_error: false,
                start_byte: self.current_position,
                end_byte: self.current_position + length,
                children: Vec::new(),
                parent: self.node_stack.last().cloned(),
            });
            
            if let Some(parent_idx) = self.node_stack.last() {
                self.nodes[*parent_idx].children.push(node_idx);
            }
            
            self.current_position += length;
        }
        
        Ok(())
    }
    
    /// Handle Skip opcode
    fn handle_skip(&mut self, reader: &mut BytecodeReader) -> Result<(), String> {
        let count = reader.read_varint()
            .ok_or("Failed to read skip count")? as usize;
        self.current_position += count;
        Ok(())
    }
    
    /// Handle Checkpoint opcode
    fn handle_checkpoint(&mut self, reader: &mut BytecodeReader) -> Result<(), String> {
        // Just read the checkpoint index, navigation uses this
        let _checkpoint_idx = reader.read_varint()
            .ok_or("Failed to read checkpoint index")?;
        Ok(())
    }
}

/// Compare decoded nodes with original for validation
pub fn validate_decoding(original: &[DecodedNode], decoded: &[DecodedNode]) -> Result<(), String> {
    if original.len() != decoded.len() {
        return Err(format!(
            "Node count mismatch: {} vs {}",
            original.len(),
            decoded.len()
        ));
    }
    
    for (i, (orig, dec)) in original.iter().zip(decoded.iter()).enumerate() {
        if orig.kind_name != dec.kind_name {
            return Err(format!(
                "Node {} kind mismatch: {} vs {}",
                i, orig.kind_name, dec.kind_name
            ));
        }
        
        if orig.start_byte != dec.start_byte || orig.end_byte != dec.end_byte {
            return Err(format!(
                "Node {} position mismatch: [{}, {}) vs [{}, {})",
                i, orig.start_byte, orig.end_byte, dec.start_byte, dec.end_byte
            ));
        }
        
        if orig.is_named != dec.is_named ||
           orig.is_missing != dec.is_missing ||
           orig.is_extra != dec.is_extra ||
           orig.is_error != dec.is_error {
            return Err(format!("Node {} flags mismatch", i));
        }
        
        if orig.field_name != dec.field_name {
            return Err(format!(
                "Node {} field mismatch: {:?} vs {:?}",
                i, orig.field_name, dec.field_name
            ));
        }
        
        if orig.children.len() != dec.children.len() {
            return Err(format!(
                "Node {} children count mismatch: {} vs {}",
                i, orig.children.len(), dec.children.len()
            ));
        }
    }
    
    Ok(())
}
