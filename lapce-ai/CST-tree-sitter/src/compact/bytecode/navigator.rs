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
    
    /// Get node by index
    pub fn get_node(&self, index: usize) -> Option<DecodedNode> {
        // Check cache first
        if let Some(node) = self.cache.borrow().get(&index) {
            return Some(node.clone());
        }
        
        // If jump table is empty, build it first
        if self.stream.jump_table.is_empty() {
            // For now, just decode sequentially from start
            // In production, we'd build the full jump table
            return self.decode_node_by_scanning(index);
        }
        
        // Use jump table for O(1) access
        let offset = *self.stream.jump_table.get(index)? as usize;
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
    
    /// Decode node by scanning from start (fallback when no jump table)
    fn decode_node_by_scanning(&self, target_index: usize) -> Option<DecodedNode> {
        let mut reader = BytecodeReader::new(&self.stream.bytes);
        let mut current_index = 0;
        
        while reader.pos < self.stream.bytes.len() {
            let node_start = reader.pos;
            let op = reader.read_op()?;
            
            // Handle non-node opcodes
            match op {
                Opcode::Checkpoint => {
                    reader.read_varint(); // checkpoint index
                    continue;
                }
                Opcode::DeltaPos | Opcode::SetPos | Opcode::Length => {
                    // These are handled within node decoding
                    continue;
                }
                Opcode::End => return None,
                _ => {}
            }
            
            if current_index == target_index {
                reader.seek(node_start);
                reader.read_op(); // Re-read the opcode
                let mut node = match op {
                    Opcode::Enter => self.decode_enter_node(&mut reader),
                    Opcode::Leaf => self.decode_leaf_node(&mut reader),
                    _ => None,
                }?;
                
                // Set stable ID from stream
                if target_index < self.stream.stable_ids.len() {
                    node.stable_id = self.stream.stable_ids[target_index];
                } else {
                    node.stable_id = (target_index as u64) + 1;
                }
                
                return Some(node);
            }
            
            // Skip this node
            match op {
                Opcode::Enter | Opcode::Leaf => {
                    reader.read_varint(); // kind_idx
                    reader.read_byte(); // flags
                    
                    // Skip position and length opcodes that follow
                    while reader.pos < self.stream.bytes.len() {
                        let peek_pos = reader.pos;
                        if let Some(next_op) = reader.read_op() {
                            match next_op {
                                Opcode::DeltaPos | Opcode::SetPos | Opcode::Length => {
                                    reader.read_varint();
                                }
                                Opcode::Enter | Opcode::Leaf => {
                                    // Next node starts here, rewind
                                    reader.seek(peek_pos);
                                    break;
                                }
                                Opcode::Exit => {
                                    if op == Opcode::Enter {
                                        // This Enter node is complete
                                        break;
                                    }
                                }
                                _ => {
                                    // Rewind for unexpected opcode
                                    reader.seek(peek_pos);
                                    break;
                                }
                            }
                        }
                    }
                    
                    // For Enter nodes, skip children recursively
                    if op == Opcode::Enter {
                        let mut depth = 1;
                        while depth > 0 && reader.pos < self.stream.bytes.len() {
                            if let Some(inner_op) = reader.read_op() {
                                match inner_op {
                                    Opcode::Enter => {
                                        depth += 1;
                                        reader.read_varint(); // kind
                                        reader.read_byte(); // flags
                                    }
                                    Opcode::Leaf => {
                                        reader.read_varint(); // kind
                                        reader.read_byte(); // flags
                                    }
                                    Opcode::Exit => depth -= 1,
                                    Opcode::Length | Opcode::DeltaPos | Opcode::SetPos => {
                                        reader.read_varint();
                                    }
                                    Opcode::Checkpoint => {
                                        reader.read_varint();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    current_index += 1;
                }
                _ => {}
            }
        }
        None
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
        let kind_idx = reader.read_varint()? as usize;
        let flags = NodeFlags::from_byte(reader.read_byte()?);
        
        let mut start_byte = 0;
        let mut end_byte = 0;
        
        // Read position and length opcodes that follow
        while reader.pos < reader.bytes.len() {
            let peek_pos = reader.pos;
            if let Some(op) = reader.read_op() {
                match op {
                    Opcode::SetPos => {
                        start_byte = reader.read_varint()? as usize;
                    }
                    Opcode::DeltaPos => {
                        let delta = reader.read_varint()? as usize;
                        start_byte += delta;
                    }
                    Opcode::Length => {
                        let length = reader.read_varint()? as usize;
                        end_byte = start_byte + length;
                    }
                    _ => {
                        // Not a position/length opcode, rewind
                        reader.seek(peek_pos);
                        break;
                    }
                }
            } else {
                break;
            }
        }
        
        let kind_name = self.stream.kind_names.get(kind_idx)
            .cloned()
            .unwrap_or_else(|| format!("kind_{}", kind_idx));
        
        // For internal nodes, we need to scan for children and exit
        let mut children = Vec::new();
        let mut depth = 1;
        
        while depth > 0 {
            let op = reader.read_op()?;
            match op {
                Opcode::Enter => {
                    children.push(reader.pos - 1);
                    depth += 1;
                    // Skip kind and flags
                    reader.read_varint()?;
                    reader.read_byte()?;
                    // Skip position and length opcodes
                    while reader.pos < reader.bytes.len() {
                        let peek_pos = reader.pos;
                        if let Some(next_op) = reader.read_op() {
                            match next_op {
                                Opcode::SetPos | Opcode::DeltaPos | Opcode::Length => {
                                    reader.read_varint();
                                }
                                _ => {
                                    reader.seek(peek_pos);
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
                Opcode::Exit => {
                    depth -= 1;
                }
                Opcode::Leaf => {
                    children.push(reader.pos - 1);
                    // Skip kind, flags
                    reader.read_varint()?;
                    reader.read_byte()?;
                    // Skip position and length opcodes
                    while reader.pos < reader.bytes.len() {
                        let peek_pos = reader.pos;
                        if let Some(next_op) = reader.read_op() {
                            match next_op {
                                Opcode::SetPos | Opcode::DeltaPos | Opcode::Length => {
                                    reader.read_varint();
                                }
                                _ => {
                                    reader.seek(peek_pos);
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
                _ => {
                    // Handle other opcodes as needed
                }
            }
        }
        
        Some(DecodedNode {
            kind_name,
            kind_id: kind_idx as u16,
            field_name: None,
            is_named: flags.is_named,
            is_missing: flags.is_missing,
            is_extra: flags.is_extra,
            is_error: flags.is_error,
            start_byte,
            end_byte,
            children,
            parent: None,
            stable_id: 0
        })
    }
    
    /// Decode a Leaf node
    fn decode_leaf_node(&self, reader: &mut BytecodeReader) -> Option<DecodedNode> {
        let kind_idx = reader.read_varint()? as usize;
        let flags = NodeFlags::from_byte(reader.read_byte()?);
        
        let mut start_byte = 0;
        let mut end_byte = 0;
        
        // Read position and length opcodes that follow
        while reader.pos < reader.bytes.len() {
            let peek_pos = reader.pos;
            if let Some(op) = reader.read_op() {
                match op {
                    Opcode::SetPos => {
                        start_byte = reader.read_varint()? as usize;
                    }
                    Opcode::DeltaPos => {
                        let delta = reader.read_varint()? as usize;
                        start_byte += delta;
                    }
                    Opcode::Length => {
                        let length = reader.read_varint()? as usize;
                        end_byte = start_byte + length;
                    }
                    _ => {
                        // Not a position/length opcode, rewind
                        reader.seek(peek_pos);
                        break;
                    }
                }
            } else {
                break;
            }
        }
        
        let kind_name = self.stream.kind_names.get(kind_idx)
            .cloned()
            .unwrap_or_else(|| format!("kind_{}", kind_idx));
        
        Some(DecodedNode {
            kind_name,
            kind_id: kind_idx as u16,
            field_name: None,
            is_named: flags.is_named,
            is_missing: flags.is_missing,
            is_extra: flags.is_extra,
            is_error: flags.is_error,
            start_byte,
            end_byte,
            children: Vec::new(),
            parent: None,
            stable_id: 0,
        })
    }
    
    /// Decode a Repeat node
    fn decode_repeat_node(&self, reader: &mut BytecodeReader) -> Option<DecodedNode> {
        let length = reader.read_varint()? as usize;
        
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
