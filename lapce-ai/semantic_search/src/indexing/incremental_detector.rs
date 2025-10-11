// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

//! Incremental change detection using stable IDs (CST-B05-2)
//!
//! Compares old vs new CST trees by stable IDs to identify:
//! - Unchanged nodes (can reuse cached embeddings)
//! - Modified nodes (need re-embedding)
//! - Added nodes (need embedding)
//! - Deleted nodes (remove from cache)

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use crate::processors::cst_to_ast_pipeline::CstNode;

/// Change detection result
#[derive(Debug, Clone)]
pub struct ChangeSet {
    /// Stable IDs that are unchanged (can reuse embeddings)
    pub unchanged: Vec<u64>,
    
    /// Stable IDs that were modified (need re-embedding)
    pub modified: Vec<u64>,
    
    /// New stable IDs (need embedding)
    pub added: Vec<u64>,
    
    /// Removed stable IDs (cleanup cache)
    pub deleted: Vec<u64>,
    
    /// File path
    pub file_path: PathBuf,
}

impl ChangeSet {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            unchanged: Vec::new(),
            modified: Vec::new(),
            added: Vec::new(),
            deleted: Vec::new(),
            file_path,
        }
    }
    
    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.modified.is_empty() || !self.added.is_empty() || !self.deleted.is_empty()
    }
    
    /// Get total number of changes
    pub fn total_changes(&self) -> usize {
        self.modified.len() + self.added.len() + self.deleted.len()
    }
    
    /// Get percentage of unchanged nodes
    pub fn unchanged_percentage(&self) -> f64 {
        let total = self.unchanged.len() + self.modified.len() + self.added.len();
        if total == 0 {
            0.0
        } else {
            (self.unchanged.len() as f64 / total as f64) * 100.0
        }
    }
}

/// Node snapshot for change detection
#[derive(Debug, Clone)]
struct NodeSnapshot {
    stable_id: u64,
    kind: String,
    text: String,
    start_byte: usize,
    end_byte: usize,
}

/// Incremental change detector
pub struct IncrementalDetector {
    /// Previous file snapshots: file_path → (stable_id → NodeSnapshot)
    snapshots: HashMap<PathBuf, HashMap<u64, NodeSnapshot>>,
}

impl IncrementalDetector {
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
        }
    }
    
    /// Detect changes between old and new CST
    pub fn detect_changes(
        &mut self,
        file_path: &PathBuf,
        new_cst: &CstNode,
    ) -> ChangeSet {
        let mut changeset = ChangeSet::new(file_path.clone());
        
        // Extract new tree's stable IDs and content
        let new_nodes = self.extract_nodes(new_cst);
        let new_ids: HashSet<u64> = new_nodes.keys().copied().collect();
        
        // Get old snapshot if exists
        if let Some(old_nodes) = self.snapshots.get(file_path) {
            let old_ids: HashSet<u64> = old_nodes.keys().copied().collect();
            
            // Unchanged: IDs in both old and new with same content
            for id in new_ids.intersection(&old_ids) {
                let old_node = &old_nodes[id];
                let new_node = &new_nodes[id];
                
                if self.nodes_equal(old_node, new_node) {
                    changeset.unchanged.push(*id);
                } else {
                    changeset.modified.push(*id);
                }
            }
            
            // Added: IDs only in new
            for id in new_ids.difference(&old_ids) {
                changeset.added.push(*id);
            }
            
            // Deleted: IDs only in old
            for id in old_ids.difference(&new_ids) {
                changeset.deleted.push(*id);
            }
        } else {
            // First time seeing this file - all nodes are new
            changeset.added = new_ids.into_iter().collect();
        }
        
        // Update snapshot
        self.snapshots.insert(file_path.clone(), new_nodes);
        
        changeset
    }
    
    /// Extract all nodes with stable IDs from CST
    fn extract_nodes(&self, cst: &CstNode) -> HashMap<u64, NodeSnapshot> {
        let mut nodes = HashMap::new();
        self.extract_nodes_recursive(cst, &mut nodes);
        nodes
    }
    
    fn extract_nodes_recursive(
        &self,
        node: &CstNode,
        nodes: &mut HashMap<u64, NodeSnapshot>,
    ) {
        if let Some(stable_id) = node.stable_id {
            nodes.insert(
                stable_id,
                NodeSnapshot {
                    stable_id,
                    kind: node.kind.clone(),
                    text: node.text.clone(),
                    start_byte: node.start_byte,
                    end_byte: node.end_byte,
                },
            );
        }
        
        for child in &node.children {
            self.extract_nodes_recursive(child, nodes);
        }
    }
    
    /// Check if two node snapshots are equal
    fn nodes_equal(&self, a: &NodeSnapshot, b: &NodeSnapshot) -> bool {
        a.kind == b.kind
            && a.text == b.text
            && a.start_byte == b.start_byte
            && a.end_byte == b.end_byte
    }
    
    /// Remove snapshot for a file
    pub fn remove_snapshot(&mut self, file_path: &PathBuf) {
        self.snapshots.remove(file_path);
    }
    
    /// Clear all snapshots
    pub fn clear(&mut self) {
        self.snapshots.clear();
    }
    
    /// Get number of tracked files
    pub fn tracked_files(&self) -> usize {
        self.snapshots.len()
    }
}

impl Default for IncrementalDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_node(stable_id: u64, kind: &str, text: &str) -> CstNode {
        CstNode {
            kind: kind.to_string(),
            text: text.to_string(),
            start_byte: 0,
            end_byte: text.len(),
            start_position: (0, 0),
            end_position: (0, 0),
            is_named: true,
            is_missing: false,
            is_extra: false,
            field_name: None,
            children: vec![],
            stable_id: Some(stable_id),
        }
    }
    
    #[test]
    fn test_first_parse_all_added() {
        let mut detector = IncrementalDetector::new();
        let file = PathBuf::from("/test.rs");
        
        let cst = create_test_node(1, "function", "fn test() {}");
        
        let changes = detector.detect_changes(&file, &cst);
        
        assert_eq!(changes.added.len(), 1);
        assert_eq!(changes.unchanged.len(), 0);
        assert_eq!(changes.modified.len(), 0);
        assert_eq!(changes.deleted.len(), 0);
        assert!(changes.added.contains(&1));
    }
    
    #[test]
    fn test_no_changes() {
        let mut detector = IncrementalDetector::new();
        let file = PathBuf::from("/test.rs");
        
        let cst = create_test_node(1, "function", "fn test() {}");
        
        // First parse
        detector.detect_changes(&file, &cst);
        
        // Second parse with same content
        let changes = detector.detect_changes(&file, &cst);
        
        assert_eq!(changes.unchanged.len(), 1);
        assert_eq!(changes.added.len(), 0);
        assert_eq!(changes.modified.len(), 0);
        assert_eq!(changes.deleted.len(), 0);
        assert!(!changes.has_changes());
    }
    
    #[test]
    fn test_content_modified() {
        let mut detector = IncrementalDetector::new();
        let file = PathBuf::from("/test.rs");
        
        let cst1 = create_test_node(1, "function", "fn test() {}");
        detector.detect_changes(&file, &cst1);
        
        // Modify content
        let cst2 = create_test_node(1, "function", "fn test() { println!(\"hi\"); }");
        let changes = detector.detect_changes(&file, &cst2);
        
        assert_eq!(changes.modified.len(), 1);
        assert_eq!(changes.unchanged.len(), 0);
        assert!(changes.modified.contains(&1));
        assert!(changes.has_changes());
    }
    
    #[test]
    fn test_node_added() {
        let mut detector = IncrementalDetector::new();
        let file = PathBuf::from("/test.rs");
        
        let mut cst1 = create_test_node(1, "source_file", "fn test() {}");
        detector.detect_changes(&file, &cst1);
        
        // Add a child node
        let child = create_test_node(2, "function", "fn new() {}");
        cst1.children.push(child);
        
        let changes = detector.detect_changes(&file, &cst1);
        
        assert_eq!(changes.unchanged.len(), 1);
        assert_eq!(changes.added.len(), 1);
        assert!(changes.added.contains(&2));
        assert!(changes.has_changes());
    }
    
    #[test]
    fn test_node_deleted() {
        let mut detector = IncrementalDetector::new();
        let file = PathBuf::from("/test.rs");
        
        let mut cst1 = create_test_node(1, "source_file", "");
        let child = create_test_node(2, "function", "fn old() {}");
        cst1.children.push(child);
        
        detector.detect_changes(&file, &cst1);
        
        // Remove child
        let cst2 = create_test_node(1, "source_file", "");
        let changes = detector.detect_changes(&file, &cst2);
        
        assert_eq!(changes.deleted.len(), 1);
        assert!(changes.deleted.contains(&2));
        assert!(changes.has_changes());
    }
    
    #[test]
    fn test_complex_changes() {
        let mut detector = IncrementalDetector::new();
        let file = PathBuf::from("/test.rs");
        
        // Initial tree: nodes 1, 2, 3
        let mut cst1 = create_test_node(1, "root", "");
        cst1.children.push(create_test_node(2, "fn", "fn a() {}"));
        cst1.children.push(create_test_node(3, "fn", "fn b() {}"));
        
        detector.detect_changes(&file, &cst1);
        
        // Updated tree: 
        // - Node 1 unchanged
        // - Node 2 modified
        // - Node 3 deleted
        // - Node 4 added
        let mut cst2 = create_test_node(1, "root", "");
        cst2.children.push(create_test_node(2, "fn", "fn a() { modified }"));
        cst2.children.push(create_test_node(4, "fn", "fn c() {}"));
        
        let changes = detector.detect_changes(&file, &cst2);
        
        assert_eq!(changes.unchanged.len(), 1);
        assert!(changes.unchanged.contains(&1));
        assert_eq!(changes.modified.len(), 1);
        assert!(changes.modified.contains(&2));
        assert_eq!(changes.added.len(), 1);
        assert!(changes.added.contains(&4));
        assert_eq!(changes.deleted.len(), 1);
        assert!(changes.deleted.contains(&3));
    }
    
    #[test]
    fn test_unchanged_percentage() {
        let mut detector = IncrementalDetector::new();
        let file = PathBuf::from("/test.rs");
        
        let mut cst1 = create_test_node(1, "root", "");
        for i in 2..12 {
            cst1.children.push(create_test_node(i, "fn", &format!("fn f{}() {{}}", i)));
        }
        
        detector.detect_changes(&file, &cst1);
        
        // Modify 1 out of 11 nodes
        let mut cst2 = create_test_node(1, "root", "");
        for i in 2..12 {
            let text = if i == 5 {
                format!("fn f{}() {{ modified }}", i)
            } else {
                format!("fn f{}() {{}}", i)
            };
            cst2.children.push(create_test_node(i, "fn", &text));
        }
        
        let changes = detector.detect_changes(&file, &cst2);
        
        // 10 unchanged out of 11 total = ~90.9%
        let pct = changes.unchanged_percentage();
        assert!(pct > 90.0 && pct < 91.0, "Expected ~90.9%, got {}", pct);
    }
}
