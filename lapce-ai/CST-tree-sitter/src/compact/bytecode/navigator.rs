//! Bytecode navigator - O(1) tree traversal using jump tables and checkpoints
//! Provides same API as CompactTree for compatibility

use super::opcodes::{BytecodeStream, BytecodeReader, Opcode, NodeFlags};
use super::decoder::DecodedNode;

/// Navigator for efficient bytecode tree traversal
pub struct BytecodeNavigator<'a> {
    stream: &'a BytecodeStream,
    cache: std::cell::RefCell<HashMap<usize, DecodedNode>>,
}

use std::collections::HashMap;

impl<'a> BytecodeNavigator<'a> {
    pub fn new(stream: &'a BytecodeStream) -> Self {
        Self {
            stream,
            cache: std::cell::RefCell::new(HashMap::new()),
        }
    }
    
    /// Get node at index using jump table
    pub fn get_node(&self, index: usize) -> Option<DecodedNode> {
        // Check cache first
        if let Some(node) = self.cache.borrow().get(&index) {
            return Some(node.clone());
        }
        
        // Use jump table for O(1) access
        let _offset = *self.stream.jump_table.get(index)? as usize;
        let mut node = self.decode_node_at(offset)?;
        
        // Add stable ID from stream
        if index < self.stream.stable_ids.len() {
            node.stable_id = self.stream.stable_ids[index];
        } else {
            node.stable_id = (index as u64) + 1;
        }
        
        // Cache the result
        self.cache.borrow_mut().insert(index, node.clone());
        
        Some(node)
    }
    
    /// Get stable ID for node at index
    pub fn get_stable_id(&self, index: usize) -> Option<u64> {
        if index < self.stream.stable_ids.len() {
            Some(self.stream.stable_ids[index])
        } else {
            // Fallback for nodes without stable IDs
            Some((index as u64) + 1)
        }
    }
    
    /// Decode node at specific byte offset
    fn decode_node_at(&self, offset: usize) -> Option<DecodedNode> {
        let mut reader = BytecodeReader::new(&self.stream.bytes);
        reader.seek(offset);
        
        // Decode the node at this position
        let op = reader.read_op()?;
        
        match op {
            Opcode::Enter => self.decode_enter_node(&mut reader),
            Opcode::Leaf => self.decode_leaf_node(&mut reader),
            Opcode::RepeatLast => self.decode_repeat_node(&mut reader),
            _ => None,
        }
    }
    
    /// Decode an Enter node
    fn decode_enter_node(&self, reader: &mut BytecodeReader) -> Option<DecodedNode> {
        let kind_id = reader.read_varint()? as u16;
        let _flags = NodeFlags::from_byte(reader.read_byte()?);
        
        // Optional position opcode
        if !reader.is_at_end() {
            if let Some(next_op) = Opcode::from_byte(reader.bytes[reader.pos]) {
                match next_op {
                    Opcode::SetPos | Opcode::DeltaPos => {
                        reader.read_op();
                        reader.read_varint();
                    }
                    _ => {}
                }
            }
        }
        // Length: support new Length opcode or legacy inline varint
        if let Some(next) = Opcode::from_byte(*reader.bytes.get(reader.position()).unwrap_or(&0)) {
            if next == Opcode::Length {
                reader.read_op();
                reader.read_varint();
            } else {
                // Legacy
                reader.read_varint();
            }
        } else {
            // Legacy
            reader.read_varint();
        }
        
        let kind_name = self.stream.kind_names.get(kind_id as usize)?.clone();
        
        // For internal nodes, we need to scan for children and exit
        let _start_pos = reader.position();
        let mut children = Vec::new();
        let mut depth = 1;
        
        while depth > 0 {
            let op = reader.read_op()?;
            match op {
                Opcode::Enter => {
                    children.push(reader.position() - 1);
                    depth += 1;
                    // Skip kind and flags
                    reader.read_varint()?;
                    reader.read_byte()?;
                    // Optional position
                    if let Some(next) = Opcode::from_byte(*reader.bytes.get(reader.position()).unwrap_or(&0)) {
                        if matches!(next, Opcode::SetPos | Opcode::DeltaPos) {
                            reader.read_op();
                            reader.read_varint();
                        }
                    }
                    // Length handling (Length opcode or legacy varint)
                    if let Some(next2) = Opcode::from_byte(*reader.bytes.get(reader.position()).unwrap_or(&0)) {
                        if next2 == Opcode::Length {
                            reader.read_op();
                            reader.read_varint()?;
                        } else {
                            reader.read_varint()?;
                        }
                    } else {
                        reader.read_varint()?;
                    }
                }
                Opcode::Exit => {
                    depth -= 1;
                }
                Opcode::Leaf => {
                    children.push(reader.position() - 1);
                    // Skip kind, flags, optional position, and length
                    reader.read_varint()?;
                    reader.read_byte()?;
                    if let Some(next) = Opcode::from_byte(*reader.bytes.get(reader.position()).unwrap_or(&0)) {
                        if matches!(next, Opcode::SetPos | Opcode::DeltaPos) {
                            reader.read_op();
                            reader.read_varint();
                        }
                    }
                    if reader.read_op()? != Opcode::Length { return None; }
                    reader.read_varint()?;
                }
                _ => {
                    // Handle other opcodes as needed
                }
            }
        }
        
        Some(DecodedNode {
            kind_name,
            kind_id,
            field_name: None, // TODO: Track fields
            is_named: flags.is_named,
            is_missing: flags.is_missing,
            is_extra: flags.is_extra,
            is_error: flags.is_error,
            start_byte: 0, // TODO: Track position
            end_byte: 0,
            children,
            parent: None,
            stable_id: 0, // Will be set by get_node
        })
    }
    
    /// Decode a Leaf node
    fn decode_leaf_node(&self, reader: &mut BytecodeReader) -> Option<DecodedNode> {
        let kind_id = reader.read_varint()? as u16;
        let _flags = NodeFlags::from_byte(reader.read_byte()?);
        // Optional position
        if let Some(next) = Opcode::from_byte(*reader.bytes.get(reader.position()).unwrap_or(&0)) {
            if matches!(next, Opcode::SetPos | Opcode::DeltaPos) {
                reader.read_op();
                reader.read_varint();
            }
        }
        // Expect Length opcode or legacy varint
        let _length = if let Some(next) = Opcode::from_byte(*reader.bytes.get(reader.position()).unwrap_or(&0)) {
            if next == Opcode::Length {
                reader.read_op();
                reader.read_varint()? as usize
            } else {
                reader.read_varint()? as usize
            }
        } else {
            reader.read_varint()? as usize
        };
        
        let kind_name = self.stream.kind_names.get(kind_id as usize)?.clone();
        
        Some(DecodedNode {
            kind_name,
            kind_id,
            field_name: None,
            is_named: flags.is_named,
            is_missing: flags.is_missing,
            is_extra: flags.is_extra,
            is_error: flags.is_error,
            start_byte: 0, // TODO: Track position
            end_byte: length,
            children: Vec::new(),
            parent: None,
            stable_id: 0, // Will be set by get_node
        })
    }
    
    /// Decode a Repeat node
    fn decode_repeat_node(&self, reader: &mut BytecodeReader) -> Option<DecodedNode> {
        let _length = reader.read_varint()? as usize;
        
        // TODO: Track last kind properly
        // For now, return a placeholder
        Some(DecodedNode {
            kind_name: "repeat".to_string(),
            kind_id: 0,
            field_name: None,
            is_named: true,
            is_missing: false,
            is_extra: false,
            is_error: false,
            start_byte: 0,
            end_byte: length,
            children: Vec::new(),
            parent: None,
            stable_id: 0, // Will be set by get_node
        })
    }
    
    /// Find node by position using checkpoints
    pub fn find_node_at_position(&self, byte_pos: usize) -> Option<usize> {
        // Binary search checkpoints for faster seeking
        let checkpoint_idx = self.stream.checkpoints
            .binary_search_by_key(&byte_pos, |&(_, offset)| offset)
            .unwrap_or_else(|i| i.saturating_sub(1));
        
        if checkpoint_idx >= self.stream.checkpoints.len() {
            return None;
        }
        
        let (start_node_idx, _) = self.stream.checkpoints[checkpoint_idx];
        
        // Linear search from checkpoint
        for node_idx in start_node_idx..self.stream.node_count {
            if let Some(node) = self.get_node(node_idx) {
                if node.start_byte <= byte_pos && byte_pos < node.end_byte {
                    return Some(node_idx);
                }
            }
        }
        
        None
    }
    
    /// Get total node count
    pub fn node_count(&self) -> usize {
        self.stream.node_count
    }
    
    /// Get source length
    pub fn source_len(&self) -> usize {
        self.stream.source_len
    }
    
    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }
}
