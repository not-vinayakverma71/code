//! Optimized compact tree structure with aggressive packing
//! Uses u16 indices and bitfield flags to minimize memory

use super::bitvec::BitVec;
use super::bp::BP;
use super::varint::{DeltaEncoder, DeltaDecoder};
use std::collections::HashMap;

/// Node flags packed into a single byte
#[derive(Clone, Copy, Debug)]
struct NodeFlags {
    is_named: bool,
    is_missing: bool,
    is_extra: bool,
    is_error: bool,
    has_field: bool,
    // 3 bits spare for future use
}

impl NodeFlags {
    fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.is_named { byte |= 1 << 0; }
        if self.is_missing { byte |= 1 << 1; }
        if self.is_extra { byte |= 1 << 2; }
        if self.is_error { byte |= 1 << 3; }
        if self.has_field { byte |= 1 << 4; }
        byte
    }
    
    fn from_byte(byte: u8) -> Self {
        NodeFlags {
            is_named: (byte & (1 << 0)) != 0,
            is_missing: (byte & (1 << 1)) != 0,
            is_extra: (byte & (1 << 2)) != 0,
            is_error: (byte & (1 << 3)) != 0,
            has_field: (byte & (1 << 4)) != 0,
        }
    }
}

/// Compact node data using minimal space
#[derive(Clone)]
pub struct OptimizedCompactTree {
    /// Balanced parentheses for tree structure
    bp: BitVec,
    
    /// BP operations index
    bp_ops: BP,
    
    /// Node data packed efficiently:
    /// - kind_id: u16 (supports up to 65536 unique node kinds)
    /// - flags: u8 (packed bitflags)
    /// - field_id: u8 (supports up to 256 field types, 255 = no field)
    node_data: Vec<u8>, // Layout: [kind_id_low, kind_id_high, flags, field_id] * node_count
    
    /// Position information (delta-encoded varints)
    start_positions: Vec<u8>,  // Delta-encoded start byte positions
    lengths: Vec<u8>,          // Delta-encoded lengths
    
    /// String tables (using u16 indices)
    kind_names: Vec<String>,
    field_names: Vec<String>,
    
    /// Metadata
    node_count: usize,
    source: Vec<u8>,
}

impl OptimizedCompactTree {
    const NODE_DATA_SIZE: usize = 4; // bytes per node
    
    pub fn new(
        bp: BitVec,
        kind_ids: Vec<u16>,
        flags: Vec<NodeFlags>,
        field_ids: Vec<Option<u8>>,
        start_positions: Vec<usize>,
        lengths: Vec<usize>,
        kind_names: Vec<String>,
        field_names: Vec<String>,
        source: Vec<u8>,
    ) -> Self {
        let node_count = kind_ids.len();
        
        // Pack node data
        let mut node_data = Vec::with_capacity(node_count * Self::NODE_DATA_SIZE);
        for i in 0..node_count {
            let kind_id = kind_ids[i];
            node_data.push((kind_id & 0xFF) as u8);        // Low byte
            node_data.push((kind_id >> 8) as u8);          // High byte
            node_data.push(flags[i].to_byte());            // Flags
            node_data.push(field_ids[i].unwrap_or(255));   // Field ID (255 = none)
        }
        
        // Delta encode positions
        let mut pos_encoder = DeltaEncoder::new();
        for &pos in &start_positions {
            pos_encoder.encode(pos as u64);
        }
        let encoded_positions = pos_encoder.finish();
        
        // Delta encode lengths
        let mut len_encoder = DeltaEncoder::new();
        for &len in &lengths {
            len_encoder.encode(len as u64);
        }
        let encoded_lengths = len_encoder.finish();
        
        let bp_ops = BP::new(bp.clone());
        
        Self {
            bp,
            bp_ops,
            node_data,
            start_positions: encoded_positions,
            lengths: encoded_lengths,
            kind_names,
            field_names,
            node_count,
            source,
        }
    }
    
    /// Get node data at index
    pub fn get_node_info(&self, node_index: usize) -> (u16, NodeFlags, Option<u8>) {
        assert!(node_index < self.node_count);
        
        let offset = node_index * Self::NODE_DATA_SIZE;
        let kind_id = self.node_data[offset] as u16 | ((self.node_data[offset + 1] as u16) << 8);
        let flags = NodeFlags::from_byte(self.node_data[offset + 2]);
        let field_id = self.node_data[offset + 3];
        let field = if field_id == 255 { None } else { Some(field_id) };
        
        (kind_id, flags, field)
    }
    
    /// Get node position (start byte and length)
    pub fn get_node_position(&self, node_index: usize) -> (usize, usize) {
        // Decode positions up to the requested index
        let mut pos_decoder = DeltaDecoder::new(&self.start_positions);
        let mut start = 0;
        for _ in 0..=node_index {
            start = pos_decoder.decode().unwrap_or(0) as usize;
        }
        
        let mut len_decoder = DeltaDecoder::new(&self.lengths);
        let mut length = 0;
        for _ in 0..=node_index {
            length = len_decoder.decode().unwrap_or(0) as usize;
        }
        
        (start, length)
    }
    
    /// Get source text
    pub fn source(&self) -> &[u8] {
        &self.source
    }
    
    /// Get node count
    pub fn node_count(&self) -> usize {
        self.node_count
    }
    
    /// Calculate memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        let bp_size = self.bp.len() / 8; // bits to bytes
        let node_data_size = self.node_data.len();
        let positions_size = self.start_positions.len() + self.lengths.len();
        let string_tables_size: usize = self.kind_names.iter().map(|s| s.len()).sum::<usize>() +
                                        self.field_names.iter().map(|s| s.len()).sum::<usize>();
        let source_size = self.source.len();
        
        bp_size + node_data_size + positions_size + string_tables_size + source_size + 
            std::mem::size_of::<Self>()
    }
    
    /// Compare memory usage with standard CompactTree
    pub fn memory_savings_percent(&self, standard_size: usize) -> f64 {
        let optimized_size = self.memory_usage();
        ((standard_size - optimized_size) as f64 / standard_size as f64) * 100.0
    }
}

/// Builder for OptimizedCompactTree
pub struct OptimizedTreeBuilder {
    bp_bits: Vec<bool>,  // Build as Vec<bool>, convert to BitVec at the end
    kind_ids: Vec<u16>,
    flags: Vec<NodeFlags>,
    field_ids: Vec<Option<u8>>,
    start_positions: Vec<usize>,
    lengths: Vec<usize>,
    kind_names: Vec<String>,
    field_names: Vec<String>,
    kind_map: HashMap<String, u16>,
    field_map: HashMap<String, u8>,
}

impl OptimizedTreeBuilder {
    pub fn new() -> Self {
        Self {
            bp_bits: Vec::new(),
            kind_ids: Vec::new(),
            flags: Vec::new(),
            field_ids: Vec::new(),
            start_positions: Vec::new(),
            lengths: Vec::new(),
            kind_names: Vec::new(),
            field_names: Vec::new(),
            kind_map: HashMap::new(),
            field_map: HashMap::new(),
        }
    }
    
    /// Add a node to the tree
    pub fn add_node(
        &mut self,
        kind_name: &str,
        is_named: bool,
        is_missing: bool,
        is_extra: bool,
        is_error: bool,
        field_name: Option<&str>,
        start_byte: usize,
        length: usize,
    ) {
        // Get or create kind ID
        let kind_id = if let Some(&id) = self.kind_map.get(kind_name) {
            id
        } else {
            let id = self.kind_names.len() as u16;
            self.kind_names.push(kind_name.to_string());
            self.kind_map.insert(kind_name.to_string(), id);
            id
        };
        
        // Get or create field ID
        let field_id = field_name.map(|name| {
            if let Some(&id) = self.field_map.get(name) {
                id
            } else {
                let id = self.field_names.len() as u8;
                self.field_names.push(name.to_string());
                self.field_map.insert(name.to_string(), id);
                id
            }
        });
        
        // Add node data
        self.kind_ids.push(kind_id);
        self.flags.push(NodeFlags {
            is_named,
            is_missing,
            is_extra,
            is_error,
            has_field: field_id.is_some(),
        });
        self.field_ids.push(field_id);
        self.start_positions.push(start_byte);
        self.lengths.push(length);
    }
    
    /// Add opening parenthesis for tree structure
    pub fn open_node(&mut self) {
        self.bp_bits.push(true);
    }
    
    /// Add closing parenthesis for tree structure
    pub fn close_node(&mut self) {
        self.bp_bits.push(false);
    }
    
    /// Build the final optimized tree
    pub fn build(self, source: Vec<u8>) -> OptimizedCompactTree {
        // Convert bp_bits to BitVec
        let bp = BitVec::from_bits(&self.bp_bits);
        
        OptimizedCompactTree::new(
            bp,
            self.kind_ids,
            self.flags,
            self.field_ids,
            self.start_positions,
            self.lengths,
            self.kind_names,
            self.field_names,
            source,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_node_flags_packing() {
        let flags = NodeFlags {
            is_named: true,
            is_missing: false,
            is_extra: true,
            is_error: false,
            has_field: true,
        };
        
        let byte = flags.to_byte();
        assert_eq!(byte, 0b00010101);
        
        let decoded = NodeFlags::from_byte(byte);
        assert_eq!(decoded.is_named, flags.is_named);
        assert_eq!(decoded.is_missing, flags.is_missing);
        assert_eq!(decoded.is_extra, flags.is_extra);
        assert_eq!(decoded.is_error, flags.is_error);
        assert_eq!(decoded.has_field, flags.has_field);
    }
    
    #[test]
    fn test_memory_savings() {
        let mut builder = OptimizedTreeBuilder::new();
        
        // Add some sample nodes
        builder.open_node();
        builder.add_node("function", true, false, false, false, None, 0, 10);
        builder.open_node();
        builder.add_node("identifier", true, false, false, false, Some("name"), 4, 3);
        builder.close_node();
        builder.close_node();
        
        let tree = builder.build(b"fn foo()".to_vec());
        
        // Check memory usage
        let memory = tree.memory_usage();
        println!("Optimized tree memory: {} bytes", memory);
        
        // Verify node data
        let (kind_id, flags, field_id) = tree.get_node_info(0);
        assert_eq!(tree.kind_names[kind_id as usize], "function");
        assert!(flags.is_named);
        assert!(!flags.has_field);
        
        let (kind_id, flags, field_id) = tree.get_node_info(1);
        assert_eq!(tree.kind_names[kind_id as usize], "identifier");
        assert!(flags.has_field);
        assert_eq!(field_id, Some(0));
        assert_eq!(tree.field_names[0], "name");
    }
}
