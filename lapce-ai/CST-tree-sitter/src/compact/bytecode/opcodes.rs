//! Bytecode opcodes for tree representation
//! 
//! Each opcode is a single byte for maximum efficiency.
//! Data follows opcodes using variable-length encoding.

use crate::compact::varint::{DeltaEncoder, DeltaDecoder};
use std::collections::HashMap;

/// Bytecode opcodes for tree operations
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    // Structure opcodes
    Enter = 0x01,       // Enter node (followed by kind_id, flags)
    Exit = 0x02,        // Exit node
    Leaf = 0x03,        // Leaf node (followed by kind_id, flags, length)
    
    // Position opcodes
    SetPos = 0x10,      // Set absolute position (followed by varint)
    DeltaPos = 0x11,    // Delta position (followed by signed varint)
    
    // Field opcodes
    Field = 0x20,       // Set field (followed by field_id)
    NoField = 0x21,     // Clear field
    
    // Optimization opcodes
    RepeatLast = 0x30,  // Repeat last node type
    Skip = 0x31,        // Skip bytes (followed by count)
    
    // Special opcodes
    Checkpoint = 0xF0,  // Navigation checkpoint (followed by offset table entry)
    End = 0xFF,         // End of stream
}

impl Opcode {
    /// Get opcode from byte
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(Opcode::Enter),
            0x02 => Some(Opcode::Exit),
            0x03 => Some(Opcode::Leaf),
            0x10 => Some(Opcode::SetPos),
            0x11 => Some(Opcode::DeltaPos),
            0x20 => Some(Opcode::Field),
            0x21 => Some(Opcode::NoField),
            0x30 => Some(Opcode::RepeatLast),
            0x31 => Some(Opcode::Skip),
            0xF0 => Some(Opcode::Checkpoint),
            0xFF => Some(Opcode::End),
            _ => None,
        }
    }
    
    /// Convert to byte
    pub fn to_byte(self) -> u8 {
        self as u8
    }
}

/// Node flags packed into a single byte
#[derive(Debug, Clone, Copy, Default)]
pub struct NodeFlags {
    pub is_named: bool,
    pub is_missing: bool,
    pub is_extra: bool,
    pub is_error: bool,
    pub has_field: bool,
}

impl NodeFlags {
    /// Pack into byte
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.is_named { byte |= 1 << 0; }
        if self.is_missing { byte |= 1 << 1; }
        if self.is_extra { byte |= 1 << 2; }
        if self.is_error { byte |= 1 << 3; }
        if self.has_field { byte |= 1 << 4; }
        byte
    }
    
    /// Unpack from byte
    pub fn from_byte(byte: u8) -> Self {
        Self {
            is_named: (byte & (1 << 0)) != 0,
            is_missing: (byte & (1 << 1)) != 0,
            is_extra: (byte & (1 << 2)) != 0,
            is_error: (byte & (1 << 3)) != 0,
            has_field: (byte & (1 << 4)) != 0,
        }
    }
}

/// Bytecode stream with navigation aids
#[derive(Default)]
pub struct BytecodeStream {
    /// Raw bytecode
    pub bytes: Vec<u8>,
    
    /// Jump table for O(1) navigation
    /// Maps node index to byte offset
    pub jump_table: Vec<u32>,
    
    /// Checkpoint offsets for fast seeking
    /// Every N nodes, store offset
    pub checkpoints: Vec<(usize, usize)>, // (node_index, byte_offset)
    
    /// String tables
    pub kind_names: Vec<String>,
    pub field_names: Vec<String>,
    
    /// Metadata
    pub node_count: usize,
    pub source_len: usize,
}

impl BytecodeStream {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            jump_table: Vec::new(),
            checkpoints: Vec::new(),
            kind_names: Vec::new(),
            field_names: Vec::new(),
            node_count: 0,
            source_len: 0,
        }
    }
    
    /// Write opcode
    pub fn write_op(&mut self, op: Opcode) {
        self.bytes.push(op.to_byte());
    }
    
    /// Write varint
    pub fn write_varint(&mut self, value: u64) {
        // LEB128 encoding
        let mut val = value;
        loop {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            self.bytes.push(byte);
            if val == 0 {
                break;
            }
        }
    }
    
    /// Write signed varint (for deltas)
    pub fn write_signed_varint(&mut self, value: i64) {
        // ZigZag encoding for signed values
        let encoded = if value < 0 {
            ((-value as u64) << 1) | 1
        } else {
            (value as u64) << 1
        };
        self.write_varint(encoded);
    }
    
    /// Write byte
    pub fn write_byte(&mut self, byte: u8) {
        self.bytes.push(byte);
    }
    
    /// Add checkpoint for navigation
    pub fn add_checkpoint(&mut self, node_index: usize) {
        self.checkpoints.push((node_index, self.bytes.len()));
        self.write_op(Opcode::Checkpoint);
        self.write_varint(self.checkpoints.len() as u64 - 1);
    }
    
    /// Calculate memory usage
    pub fn memory_usage(&self) -> usize {
        self.bytes.len() +
        self.jump_table.len() * 4 +
        self.checkpoints.len() * 16 +
        self.kind_names.iter().map(|s| s.len()).sum::<usize>() +
        self.field_names.iter().map(|s| s.len()).sum::<usize>() +
        std::mem::size_of::<Self>()
    }
}

/// Bytecode reader for decoding
pub struct BytecodeReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> BytecodeReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
    
    /// Read next opcode
    pub fn read_op(&mut self) -> Option<Opcode> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        let byte = self.bytes[self.pos];
        self.pos += 1;
        Opcode::from_byte(byte)
    }
    
    /// Read varint
    pub fn read_varint(&mut self) -> Option<u64> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        // LEB128 decoding
        let mut result = 0u64;
        let mut shift = 0;
        loop {
            if self.pos >= self.bytes.len() {
                return None;
            }
            let byte = self.bytes[self.pos];
            self.pos += 1;
            
            result |= ((byte & 0x7F) as u64) << shift;
            
            if byte & 0x80 == 0 {
                return Some(result);
            }
            
            shift += 7;
            if shift > 63 {
                return None; // Overflow
            }
        }
    }
    
    /// Read signed varint
    pub fn read_signed_varint(&mut self) -> Option<i64> {
        self.read_varint().map(|encoded| {
            // ZigZag decoding
            if encoded & 1 == 1 {
                -((encoded >> 1) as i64)
            } else {
                (encoded >> 1) as i64
            }
        })
    }
    
    /// Read byte
    pub fn read_byte(&mut self) -> Option<u8> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        let byte = self.bytes[self.pos];
        self.pos += 1;
        Some(byte)
    }
    
    /// Seek to position
    pub fn seek(&mut self, pos: usize) {
        self.pos = pos.min(self.bytes.len());
    }
    
    /// Get current position
    pub fn position(&self) -> usize {
        self.pos
    }
    
    /// Check if at end
    pub fn is_at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }
}
