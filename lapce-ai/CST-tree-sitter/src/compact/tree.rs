//! Compact tree structure - simplified for bytecode representation
//! Phase 1 & 3 optimizations

use super::packed_array::PackedArray;
use super::varint::PrefixSumIndex;
use super::node::CompactNode;

/// Compact tree representation (simplified)
/// Uses bytecode representation internally
#[derive(Clone)]
pub struct CompactTree {
    /// Node kind IDs (packed)
    pub(crate) kind_ids: PackedArray,
    
    /// Node flags (simple vectors for now)
    pub(crate) is_named: Vec<bool>,
    pub(crate) is_missing: Vec<bool>,
    pub(crate) is_extra: Vec<bool>,
    pub(crate) is_error: Vec<bool>,
    
    /// Field information
    pub(crate) field_present: Vec<bool>,
    pub(crate) field_ids: PackedArray,
    
    /// Position information (delta-encoded)
    pub(crate) start_bytes_encoded: Vec<u8>,
    pub(crate) len_bytes_encoded: Vec<u8>,
    pub(crate) position_index: PrefixSumIndex,
    
    /// Subtree sizes for range queries
    pub(crate) subtree_sizes: PackedArray,
    
    /// String tables
    pub(crate) kind_names: Vec<String>,
    pub(crate) field_names: Vec<String>,
    
    pub(crate) node_count: usize,
    pub(crate) source: Vec<u8>,
}

impl CompactTree {
    /// Create new CompactTree from all components
    pub fn new(
        kind_ids: PackedArray,
        is_named: Vec<bool>,
        is_missing: Vec<bool>,
        is_extra: Vec<bool>,
        is_error: Vec<bool>,
        field_present: Vec<bool>,
        field_ids: PackedArray,
        start_bytes_encoded: Vec<u8>,
        len_bytes_encoded: Vec<u8>,
        position_index: PrefixSumIndex,
        subtree_sizes: PackedArray,
        kind_names: Vec<String>,
        field_names: Vec<String>,
        node_count: usize,
        source: Vec<u8>,
    ) -> Self {
        Self {
            kind_ids,
            is_named,
            is_missing,
            is_extra,
            is_error,
            field_present,
            field_ids,
            start_bytes_encoded,
            len_bytes_encoded,
            position_index,
            subtree_sizes,
            kind_names,
            field_names,
            node_count,
            source,
        }
    }
    
    /// Get root node
    pub fn root(&self) -> CompactNode {
        CompactNode::new(self, 0)
    }
    
    /// Get node at index
    pub fn node_at(&self, index: usize) -> Option<CompactNode> {
        if index < self.node_count {
            Some(CompactNode::new(self, index))
        } else {
            None
        }
    }
    pub fn node_count(&self) -> usize {
        self.node_count
    }
    
    /// Get source text
    pub fn source(&self) -> &[u8] {
        &self.source
    }
    
    /// Get node index (simplified - just returns the index)
    pub(crate) fn node_index(&self, index: usize) -> usize {
        index
    }
    
    /// Get start byte for node
    pub fn start_byte(&self, node_idx: usize) -> usize {
        // Decode directly from delta-encoded stream
        use super::varint::DeltaDecoder;
        let mut decoder = DeltaDecoder::new(&self.start_bytes_encoded);
        
        // Decode up to node_idx
        let mut value = 0;
        for i in 0..=node_idx {
            value = decoder.decode().unwrap_or(0);
            if i == node_idx {
                return value as usize;
            }
        }
        0
    }
    
    /// Get end byte for node
    pub fn end_byte(&self, node_idx: usize) -> usize {
        let start = self.start_byte(node_idx);
        let len = self.len_bytes(node_idx);
        start + len
    }
    
    /// Get length in bytes for node
    pub(crate) fn len_bytes(&self, node_idx: usize) -> usize {
        // Decode from varint stream (not delta-encoded)
        use super::varint::VarInt;
        
        let mut pos = 0;
        for i in 0..=node_idx {
            let (value, consumed) = VarInt::decode_u64(&self.len_bytes_encoded[pos..])
                .unwrap_or((0, 1));
            pos += consumed;
            if i == node_idx {
                return value as usize;
            }
        }
        0
    }
    
    /// Get kind ID for node
    pub(crate) fn kind_id(&self, node_idx: usize) -> u16 {
        self.kind_ids.get(node_idx) as u16
    }
    
    /// Get kind name for node
    pub(crate) fn kind_name(&self, node_idx: usize) -> &str {
        let kind_id = self.kind_id(node_idx);
        &self.kind_names[kind_id as usize]
    }
    
    /// Get field ID for node (if present)
    pub(crate) fn field_id(&self, node_idx: usize) -> Option<u8> {
        if node_idx < self.field_present.len() && self.field_present[node_idx] {
            // Count how many fields before this node
            let field_idx = self.field_present[..node_idx]
                .iter()
                .filter(|&&x| x)
                .count();
            Some(self.field_ids.get(field_idx) as u8)
        } else {
            None
        }
    }
    
    /// Get field name for node
    pub(crate) fn field_name(&self, node_idx: usize) -> Option<&str> {
        self.field_id(node_idx).map(|id| &self.field_names[id as usize] as &str)
    }
    
    /// Get subtree size for node
    pub(crate) fn subtree_size(&self, node_idx: usize) -> usize {
        self.subtree_sizes.get(node_idx) as usize
    }
    
    /// Memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        let bp_bytes = 0; // No BP anymore
        let kind_bytes = self.kind_ids.memory_bytes();
        let flag_bytes = (self.is_named.len() + self.is_missing.len() 
                         + self.is_extra.len() + self.is_error.len() 
                         + self.field_present.len() + 7) / 8;
        let field_bytes = self.field_ids.memory_bytes();
        let position_bytes = self.start_bytes_encoded.len() + self.len_bytes_encoded.len();
        let subtree_bytes = self.subtree_sizes.memory_bytes();
        let string_bytes: usize = self.kind_names.iter().map(|s| s.len()).sum::<usize>()
                                + self.field_names.iter().map(|s| s.len()).sum::<usize>();
        
        bp_bytes + kind_bytes + flag_bytes + field_bytes + position_bytes 
        + subtree_bytes + string_bytes + std::mem::size_of::<Self>()
    }
    
    /// Memory usage alias
    pub fn memory_usage(&self) -> usize {
        self.memory_bytes()
    }
    
    /// Bytes per node
    pub fn bytes_per_node(&self) -> f64 {
        self.memory_bytes() as f64 / self.node_count as f64
    }
    
    /// Validate tree structure (simplified)
    pub fn validate(&self) -> Result<(), String> {
        // Check node count
        if self.node_count == 0 {
            return Err("Empty tree".to_string());
        }
        
        // Check array lengths
        if self.kind_ids.len() != self.node_count {
            return Err(format!("Kind IDs length mismatch: {} vs {}", 
                              self.kind_ids.len(), self.node_count));
        }
        
        Ok(())
    }
}

impl std::fmt::Debug for CompactTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CompactTree({} nodes, {:.2} bytes/node, {} KB total)",
               self.node_count,
               self.bytes_per_node(),
               self.memory_bytes() / 1024)
    }
}
