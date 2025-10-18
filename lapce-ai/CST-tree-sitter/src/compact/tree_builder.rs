//! Builder for converting Tree-sitter trees to compact format
//! Walks tree in preorder, extracting structure and attributes

use tree_sitter::{Tree, TreeCursor};
use super::packed_array::PackedArray;
use super::varint::{DeltaEncoder, PrefixSumIndex};
use super::tree::CompactTree;
use std::collections::HashMap;

/// Builder for creating CompactTree from Tree-sitter Tree
pub struct CompactTreeBuilder {
    /// BP sequence being built
    bp_sequence: Vec<bool>,
    
    /// Node attributes
    kind_ids: Vec<u16>,
    is_named: Vec<bool>,
    is_missing: Vec<bool>,
    is_extra: Vec<bool>,
    is_error: Vec<bool>,
    
    /// Field information
    field_present: Vec<bool>,
    field_ids: Vec<u8>,
    
    /// Position information
    start_bytes: Vec<u64>,
    len_bytes: Vec<u64>,
    
    /// Subtree sizes for O(1) range queries
    subtree_sizes: Vec<u32>,
    
    /// String interning
    kind_map: HashMap<String, u16>,
    field_map: HashMap<String, u8>,
    
    /// Statistics
    node_count: usize,
}

impl CompactTreeBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            bp_sequence: Vec::new(),
            kind_ids: Vec::new(),
            is_named: Vec::new(),
            is_missing: Vec::new(),
            is_extra: Vec::new(),
            is_error: Vec::new(),
            field_present: Vec::new(),
            field_ids: Vec::new(),
            start_bytes: Vec::new(),
            len_bytes: Vec::new(),
            subtree_sizes: Vec::new(),
            kind_map: HashMap::new(),
            field_map: HashMap::new(),
            node_count: 0,
        }
    }
    
    /// Build CompactTree from Tree-sitter Tree
    pub fn build(mut self, tree: &Tree, source: &[u8]) -> CompactTree {
        let mut cursor = tree.walk();
        self.visit_node(&mut cursor, source, None);
        
        // Pack kind_ids (typically <256 kinds, so 8 bits suffice)
        let max_kind = *self.kind_ids.iter().max().unwrap_or(&0);
        let kind_bits = if max_kind == 0 {
            1
        } else {
            // Calculate bits needed to store values 0..=max_kind
            (max_kind as u32 + 1).next_power_of_two().trailing_zeros() as usize
        };
        let mut kind_array = PackedArray::new(kind_bits);
        for &kid in &self.kind_ids {
            kind_array.push(kid as u64);
        }
        
        // Keep flags as vectors for now (simplified)
        let is_named_vec = self.is_named.clone();
        let is_missing_vec = self.is_missing.clone();
        let is_extra_vec = self.is_extra.clone();
        let is_error_vec = self.is_error.clone();
        let field_present_vec = self.field_present.clone();
        
        // Pack field_ids (sparse, only for nodes with fields)
        let mut field_array = PackedArray::new(8);
        for &fid in &self.field_ids {
            field_array.push(fid as u64);
        }
        
        // Import VarInt
        use super::varint::VarInt;
        
        // Delta-encode positions (only start_bytes are monotonic)
        let mut start_encoder = DeltaEncoder::new();
        start_encoder.encode_batch(&self.start_bytes);
        let start_bytes_encoded = start_encoder.finish();
        
        // Just varint encode lengths (not delta, since they're not monotonic)
        let mut len_bytes_encoded = Vec::new();
        for &len in &self.len_bytes {
            VarInt::encode_u64(len, &mut len_bytes_encoded);
        }
        
        // Create prefix sum index for fast position access
        let position_index = PrefixSumIndex::from_values(&self.start_bytes, 256);
        
        // Pack subtree sizes
        let max_subtree = *self.subtree_sizes.iter().max().unwrap_or(&0);
        let subtree_bits = if max_subtree == 0 {
            1
        } else {
            (max_subtree + 1).next_power_of_two().trailing_zeros() as usize
        };
        let mut subtree_array = PackedArray::new(subtree_bits);
        for &size in &self.subtree_sizes {
            subtree_array.push(size as u64);
        }
        
        // Create reverse maps for string tables
        let mut kind_names = vec![String::new(); self.kind_map.len()];
        for (name, &id) in &self.kind_map {
            kind_names[id as usize] = name.clone();
        }
        
        let mut field_names = vec![String::new(); self.field_map.len()];
        for (name, &id) in &self.field_map {
            field_names[id as usize] = name.clone();
        }
        
        CompactTree::new(
            kind_array,
            is_named_vec,
            is_missing_vec,
            is_extra_vec,
            is_error_vec,
            field_present_vec,
            field_array,
            start_bytes_encoded,
            len_bytes_encoded,
            position_index,
            subtree_array,
            kind_names,
            field_names,
            self.node_count,
            source.to_vec(),
        )
    }
    
    /// Visit node recursively in preorder
    fn visit_node(&mut self, cursor: &mut TreeCursor, source: &[u8], field_name: Option<&str>) {
        let node = cursor.node();
        
        // Open parenthesis
        self.bp_sequence.push(true);
        
        // Get or assign kind ID
        let kind_name = node.kind();
        let kind_id = self.get_or_create_kind_id(kind_name);
        self.kind_ids.push(kind_id);
        
        // Store flags
        self.is_named.push(node.is_named());
        self.is_missing.push(node.is_missing());
        self.is_extra.push(node.is_extra());
        self.is_error.push(node.is_error());
        
        // Store field information
        if let Some(field) = field_name {
            self.field_present.push(true);
            let field_id = self.get_or_create_field_id(field);
            self.field_ids.push(field_id);
        } else {
            self.field_present.push(false);
        }
        
        // Store positions
        self.start_bytes.push(node.start_byte() as u64);
        self.len_bytes.push((node.end_byte() - node.start_byte()) as u64);
        
        // Count nodes in subtree (will update later)
        let subtree_start_idx = self.node_count;
        self.node_count += 1;
        
        // Visit children
        if cursor.goto_first_child() {
            loop {
                let field_name = cursor.field_name();
                self.visit_node(cursor, source, field_name);
                
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        
        // Calculate and store subtree size
        let subtree_size = (self.node_count - subtree_start_idx) as u32;
        self.subtree_sizes.push(subtree_size);
        
        // Close parenthesis
        self.bp_sequence.push(false);
    }
    
    /// Get or create kind ID
    fn get_or_create_kind_id(&mut self, kind: &str) -> u16 {
        if let Some(&id) = self.kind_map.get(kind) {
            id
        } else {
            let id = self.kind_map.len() as u16;
            self.kind_map.insert(kind.to_string(), id);
            id
        }
    }
    
    /// Get or create field ID
    fn get_or_create_field_id(&mut self, field: &str) -> u8 {
        if let Some(&id) = self.field_map.get(field) {
            id
        } else {
            let id = self.field_map.len() as u8;
            self.field_map.insert(field.to_string(), id);
            id
        }
    }
}

impl Default for CompactTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
