//! Compact tree structure holding all succinct data
//! Memory-efficient representation of CST with O(1) operations

use super::bitvec::BitVec;
use super::rank_select::RankSelect;
use super::bp::BP;
use super::packed_array::PackedArray;
use super::varint::PrefixSumIndex;
use super::node::CompactNode;

/// Compact tree representation
/// Stores entire CST in ~50-70 KB instead of ~768 KB
#[derive(Clone)]
pub struct CompactTree {
    /// Balanced parentheses for tree structure
    pub(crate) bp: BitVec,
    
    /// BP operations index (created from bp)
    pub(crate) bp_ops: BP,
    
    /// Node kind IDs (packed)
    pub(crate) kind_ids: PackedArray,
    
    /// Node flags (bitvectors)
    pub(crate) is_named: BitVec,
    pub(crate) is_missing: BitVec,
    pub(crate) is_extra: BitVec,
    pub(crate) is_error: BitVec,
    
    /// Field information
    pub(crate) field_present: BitVec,
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
    
    /// Metadata
    pub(crate) node_count: usize,
    pub(crate) source: Vec<u8>,
}

impl CompactTree {
    /// Create new CompactTree
    pub fn new(
        bp: BitVec,
        kind_ids: PackedArray,
        is_named: BitVec,
        is_missing: BitVec,
        is_extra: BitVec,
        is_error: BitVec,
        field_present: BitVec,
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
        let bp_ops = BP::new(bp.clone());
        
        Self {
            bp,
            bp_ops,
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
    
    /// Get node at BP position
    pub fn node_at(&self, bp_pos: usize) -> Option<CompactNode> {
        if bp_pos < self.bp.len() && self.bp.get(bp_pos) {
            Some(CompactNode::new(self, bp_pos))
        } else {
            None
        }
    }
    
    /// Total number of nodes
    pub fn node_count(&self) -> usize {
        self.node_count
    }
    
    /// Get source text
    pub fn source(&self) -> &[u8] {
        &self.source
    }
    
    /// Get node index from BP position (preorder rank)
    pub(crate) fn node_index(&self, bp_pos: usize) -> usize {
        // Count open parentheses up to and including this position
        // Since rank1 counts up to (but not including) the position,
        // we need rank1(bp_pos + 1) - 1 for 0-based indexing
        if bp_pos >= self.bp.len() || !self.bp.get(bp_pos) {
            0 // Invalid position or not an open paren
        } else {
            RankSelect::new(self.bp.clone()).rank1(bp_pos + 1) - 1
        }
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
        if self.field_present.get(node_idx) {
            // Count how many fields before this node
            let field_idx = self.field_present.rank1(node_idx);
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
        let bp_bytes = (self.bp.len() + 7) / 8;
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
    
    /// Bytes per node
    pub fn bytes_per_node(&self) -> f64 {
        self.memory_bytes() as f64 / self.node_count as f64
    }
    
    /// Get BP bitvector (for debugging)
    pub fn bp_bitvec(&self) -> &BitVec {
        &self.bp
    }
    
    /// Get BP operations (for debugging)
    pub fn bp_operations(&self) -> &BP {
        &self.bp_ops
    }
    
    /// Validate tree structure
    pub fn validate(&self) -> Result<(), String> {
        // Check BP is balanced
        let opens = self.bp.count_ones();
        let closes = self.bp.count_zeros();
        if opens != closes {
            return Err(format!("Unbalanced parentheses: {} opens, {} closes", opens, closes));
        }
        
        // Check node count
        if opens != self.node_count {
            return Err(format!("Node count mismatch: {} vs {}", opens, self.node_count));
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
