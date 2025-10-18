//! Edit delta and event API for semantic consumers
//! 
//! Provides a stream of AST changes (add/move/remove nodes) and byte-range deltas
//! for incremental semantic analysis.

use crate::compact::bytecode::BytecodeStream;
use crate::compact::bytecode::decoder::{BytecodeDecoder, DecodedNode};
use std::collections::HashMap;

/// Type of change to a node
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeChangeType {
    /// Node was added
    Added,
    /// Node was removed
    Removed,
    /// Node was moved to a different position
    Moved { from: NodePosition, to: NodePosition },
    /// Node content was modified
    Modified,
}

/// Position of a node in the tree
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodePosition {
    /// Parent node's stable ID (0 if root)
    pub parent_id: u64,
    /// Index among siblings
    pub sibling_index: usize,
    /// Byte range in source
    pub byte_range: (usize, usize),
}

/// A change to a single node
#[derive(Debug, Clone)]
pub struct NodeDelta {
    /// Stable ID of the node
    pub stable_id: u64,
    /// Type of change
    pub change_type: NodeChangeType,
    /// Node kind (from tree-sitter)
    pub kind: String,
    /// Whether node is named
    pub is_named: bool,
}

/// Byte-range change in the source
#[derive(Debug, Clone)]
pub struct ByteRangeDelta {
    /// Original byte range
    pub old_range: (usize, usize),
    /// New byte range
    pub new_range: (usize, usize),
    /// Text that was replaced
    pub old_text: String,
    /// Text that replaced it
    pub new_text: String,
}

/// Complete delta between two trees
#[derive(Debug, Clone)]
pub struct TreeDelta {
    /// Node-level changes
    pub node_deltas: Vec<NodeDelta>,
    /// Byte-range changes
    pub byte_deltas: Vec<ByteRangeDelta>,
    /// Mapping from old stable IDs to new stable IDs for moved/modified nodes
    pub id_mapping: HashMap<u64, u64>,
}

/// Compute delta between two bytecode streams
pub fn compute_delta(old_stream: &BytecodeStream, new_stream: &BytecodeStream) -> TreeDelta {
    let mut node_deltas = Vec::new();
    let mut byte_deltas = Vec::new();
    let mut id_mapping = HashMap::new();
    
    // Decode both streams to get accurate positions and parent/children
    let old_nodes: Vec<DecodedNode> = {
        let mut dec = BytecodeDecoder::new();
        dec.decode(old_stream).unwrap_or_default()
    };
    let new_nodes: Vec<DecodedNode> = {
        let mut dec = BytecodeDecoder::new();
        dec.decode(new_stream).unwrap_or_default()
    };
    
    // Build ID sets for quick lookup (stable_id -> index)
    let old_ids: HashMap<u64, usize> = old_stream.stable_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();
    
    let new_ids: HashMap<u64, usize> = new_stream.stable_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();
    
    // Find removed nodes (in old but not in new)
    for (&old_id, &old_idx) in &old_ids {
        if !new_ids.contains_key(&old_id) {
            node_deltas.push(NodeDelta {
                stable_id: old_id,
                change_type: NodeChangeType::Removed,
                kind: get_node_kind(old_stream, old_idx),
                is_named: true, // TODO: Get from flags
            });
        }
    }
    
    // Find added nodes (in new but not in old)
    for (&new_id, &new_idx) in &new_ids {
        if !old_ids.contains_key(&new_id) {
            node_deltas.push(NodeDelta {
                stable_id: new_id,
                change_type: NodeChangeType::Added,
                kind: get_node_kind(new_stream, new_idx),
                is_named: true, // TODO: Get from flags
            });
        }
    }
    
    // Find moved/modified nodes (in both but different position or content)
    for (&id, &old_idx) in &old_ids {
        if let Some(&new_idx) = new_ids.get(&id) {
            // Check if position changed (using decoded nodes)
            let old_pos = get_node_position_from_nodes(&old_nodes, old_idx);
            let new_pos = get_node_position_from_nodes(&new_nodes, new_idx);
            
            if old_pos != new_pos {
                node_deltas.push(NodeDelta {
                    stable_id: id,
                    change_type: NodeChangeType::Moved {
                        from: old_pos,
                        to: new_pos,
                    },
                    kind: get_node_kind(new_stream, new_idx),
                    is_named: true,
                });
            }
            
            // Map old ID to new ID (same in this case since we're tracking stable IDs)
            id_mapping.insert(id, id);
        }
    }
    
    // TODO: Compute byte-range deltas from actual edit operations
    
    TreeDelta {
        node_deltas,
        byte_deltas,
        id_mapping,
    }
}

// Helper functions (simplified for now)
fn get_node_kind(stream: &BytecodeStream, index: usize) -> String {
    // In a real implementation, decode from bytecode
    if index < stream.kind_names.len() {
        stream.kind_names[index].clone()
    } else {
        "unknown".to_string()
    }
}

fn get_node_position_from_nodes(nodes: &[DecodedNode], index: usize) -> NodePosition {
    if let Some(node) = nodes.get(index) {
        // Compute parent stable_id (0 if None)
        let parent_id = node.parent.and_then(|p_idx| nodes.get(p_idx).map(|p| p.stable_id)).unwrap_or(0);
        // Compute sibling index among parent's children
        let sibling_index = if let Some(p_idx) = node.parent {
            if let Some(parent) = nodes.get(p_idx) {
                parent
                    .children
                    .iter()
                    .position(|&c| c == index)
                    .unwrap_or(0)
            } else { 0 }
        } else { 0 };
        NodePosition {
            parent_id,
            sibling_index,
            byte_range: (node.start_byte, node.end_byte),
        }
    } else {
        NodePosition { parent_id: 0, sibling_index: 0, byte_range: (0, 0) }
    }
}

/// Event stream for incremental updates
#[derive(Debug, Clone)]
pub enum TreeEvent {
    /// Node was added
    NodeAdded {
        stable_id: u64,
        parent_id: u64,
        kind: String,
        byte_range: (usize, usize),
    },
    /// Node was removed
    NodeRemoved {
        stable_id: u64,
    },
    /// Node was moved
    NodeMoved {
        stable_id: u64,
        old_parent: u64,
        new_parent: u64,
        old_index: usize,
        new_index: usize,
    },
    /// Node content changed
    NodeModified {
        stable_id: u64,
        old_text: String,
        new_text: String,
    },
    /// Source text changed
    TextEdit {
        offset: usize,
        old_length: usize,
        new_length: usize,
        new_text: String,
    },
}

/// Stream processor for tree events
pub struct TreeEventStream {
    events: Vec<TreeEvent>,
    current_index: usize,
}

impl TreeEventStream {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            current_index: 0,
        }
    }
    
    /// Add an event to the stream
    pub fn push(&mut self, event: TreeEvent) {
        self.events.push(event);
    }
    
    /// Get next event from the stream
    pub fn next(&mut self) -> Option<&TreeEvent> {
        if self.current_index < self.events.len() {
            let event = &self.events[self.current_index];
            self.current_index += 1;
            Some(event)
        } else {
            None
        }
    }
    
    /// Reset stream to beginning
    pub fn reset(&mut self) {
        self.current_index = 0;
    }
    
    /// Get all events
    pub fn all_events(&self) -> &[TreeEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compute_delta_empty() {
        let old_stream = BytecodeStream::new();
        let new_stream = BytecodeStream::new();
        
        let delta = compute_delta(&old_stream, &new_stream);
        
        assert!(delta.node_deltas.is_empty());
        assert!(delta.byte_deltas.is_empty());
        assert!(delta.id_mapping.is_empty());
    }
    
    #[test]
    fn test_compute_delta_additions() {
        let old_stream = BytecodeStream::new();
        let mut new_stream = BytecodeStream::new();
        
        // Add some nodes to new stream
        new_stream.stable_ids.push(1);
        new_stream.stable_ids.push(2);
        new_stream.stable_ids.push(3);
        
        let delta = compute_delta(&old_stream, &new_stream);
        
        assert_eq!(delta.node_deltas.len(), 3);
        for node_delta in &delta.node_deltas {
            assert_eq!(node_delta.change_type, NodeChangeType::Added);
        }
    }
    
    #[test]
    fn test_compute_delta_removals() {
        let mut old_stream = BytecodeStream::new();
        let new_stream = BytecodeStream::new();
        
        // Add some nodes to old stream
        old_stream.stable_ids.push(1);
        old_stream.stable_ids.push(2);
        old_stream.stable_ids.push(3);
        
        let delta = compute_delta(&old_stream, &new_stream);
        
        assert_eq!(delta.node_deltas.len(), 3);
        for node_delta in &delta.node_deltas {
            assert_eq!(node_delta.change_type, NodeChangeType::Removed);
        }
    }
    
    #[test]
    fn test_event_stream() {
        let mut stream = TreeEventStream::new();
        
        stream.push(TreeEvent::NodeAdded {
            stable_id: 1,
            parent_id: 0,
            kind: "function".to_string(),
            byte_range: (0, 10),
        });
        
        stream.push(TreeEvent::NodeRemoved {
            stable_id: 2,
        });
        
        assert_eq!(stream.all_events().len(), 2);
        
        assert!(matches!(stream.next(), Some(TreeEvent::NodeAdded { .. })));
        assert!(matches!(stream.next(), Some(TreeEvent::NodeRemoved { .. })));
        assert!(stream.next().is_none());
        
        stream.reset();
        assert!(matches!(stream.next(), Some(TreeEvent::NodeAdded { .. })));
    }
}
