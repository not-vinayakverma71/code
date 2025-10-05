//! Balanced Parentheses operations for tree navigation
//! Core of succinct tree representation

use super::rank_select::RankSelect;
use super::bitvec::BitVec;

/// Balanced Parentheses structure for O(1) tree navigation
#[derive(Clone)]
pub struct BP {
    /// The BP bitvector: '(' = 1, ')' = 0
    rs: RankSelect,
}

impl BP {
    /// Create from a balanced parentheses bitvector
    pub fn new(bitvec: BitVec) -> Self {
        Self {
            rs: RankSelect::new(bitvec),
        }
    }
    
    /// Create from a sequence of parentheses
    pub fn from_sequence(sequence: &[bool]) -> Self {
        let bitvec = BitVec::from_bits(sequence);
        Self::new(bitvec)
    }
    
    /// Find the matching closing parenthesis for open at position i
    /// Simple scanning approach with depth tracking
    #[inline]
    pub fn find_close(&self, i: usize) -> Option<usize> {
        if i >= self.rs.bitvec().len() || !self.rs.bitvec().get(i) {
            return None; // Not an open parenthesis
        }
        
        let mut depth = 1;
        let mut pos = i + 1;
        
        while pos < self.rs.bitvec().len() {
            if self.rs.bitvec().get(pos) {
                depth += 1; // Open paren
            } else {
                depth -= 1; // Close paren
                if depth == 0 {
                    return Some(pos); // Found matching close
                }
            }
            pos += 1;
        }
        
        None
    }
    
    /// Find the opening parenthesis matching close at position i
    /// Inverse of find_close
    #[inline]
    pub fn find_open(&self, i: usize) -> Option<usize> {
        if i >= self.rs.bitvec().len() || self.rs.bitvec().get(i) {
            return None; // Not a close parenthesis
        }
        
        // Calculate excess after this close
        let excess_after = 2 * self.rs.rank1(i + 1) - (i + 1);
        let target_excess = excess_after + 1;
        
        // Binary search backward for opening position
        let mut left = 0;
        let mut right = i;
        let mut result = None;
        
        while left <= right {
            let mid = left + (right - left) / 2;
            let excess_mid = 2 * self.rs.rank1(mid + 1) - (mid + 1);
            
            if self.rs.bitvec().get(mid) && excess_mid == target_excess {
                result = Some(mid);
                if mid == 0 {
                    break;
                }
                right = mid - 1;
            } else if excess_mid < target_excess {
                left = mid + 1;
            } else {
                if mid == 0 {
                    break;
                }
                right = mid - 1;
            }
        }
        
        result
    }
    
    /// Find the tightest enclosing pair of parentheses
    /// Returns the position of the open parenthesis that encloses position i
    #[inline]
    pub fn enclose(&self, i: usize) -> Option<usize> {
        if i == 0 || i >= self.rs.bitvec().len() {
            return None;
        }
        
        // If i is a close paren, find its open first
        let node_open = if self.rs.bitvec().get(i) {
            i
        } else {
            self.find_open(i)?
        };
        
        // Find the previous unmatched open parenthesis
        if node_open == 0 {
            return None; // Root node has no parent
        }
        
        // Count excess to find enclosing open
        let mut pos = node_open - 1;
        let mut excess = 0;
        
        loop {
            if self.rs.bitvec().get(pos) {
                if excess == 0 {
                    return Some(pos);
                }
                excess += 1;
            } else {
                excess -= 1;
            }
            
            if pos == 0 {
                break;
            }
            pos -= 1;
        }
        
        None
    }
    
    /// Find the k-th child of the node starting at position i
    /// Children are nodes whose enclosing parent is i
    #[inline]
    pub fn kth_child(&self, i: usize, k: usize) -> Option<usize> {
        if k == 0 || !self.rs.bitvec().get(i) {
            return None;
        }
        
        let close = self.find_close(i)?;
        
        // The first child is at position i+1 if it's an open paren
        let mut pos = i + 1;
        let mut child_count = 0;
        let mut depth = 0;
        
        while pos < close {
            if self.rs.bitvec().get(pos) {
                // Open paren - potential child
                if depth == 0 {
                    // Direct child
                    child_count += 1;
                    if child_count == k {
                        return Some(pos);
                    }
                }
                depth += 1;
            } else {
                // Close paren
                depth -= 1;
            }
            pos += 1;
        }
        
        None
    }
    
    /// Find the next sibling of the node starting at position i
    #[inline]
    pub fn next_sibling(&self, i: usize) -> Option<usize> {
        if !self.rs.bitvec().get(i) {
            return None;
        }
        
        // Find where this node ends
        let close = self.find_close(i)?;
        
        // Check if there's a node right after
        let next = close + 1;
        if next < self.rs.bitvec().len() && self.rs.bitvec().get(next) {
            // Verify it's a sibling (same parent)
            if let Some(parent) = self.enclose(i) {
                if let Some(next_parent) = self.enclose(next) {
                    if parent == next_parent {
                        return Some(next);
                    }
                }
            } else {
                // Both are root-level nodes
                if self.enclose(next).is_none() {
                    return Some(next);
                }
            }
        }
        
        None
    }
    
    /// Find the previous sibling of the node starting at position i  
    #[inline]
    pub fn previous_sibling(&self, i: usize) -> Option<usize> {
        if i == 0 || !self.rs.bitvec().get(i) {
            return None;
        }
        
        // Find parent to verify sibling relationship
        let parent = self.enclose(i);
        
        // Look backward for an open parenthesis at the same level
        let mut pos = i - 1;
        let mut excess = 0;
        
        loop {
            if self.rs.bitvec().get(pos) {
                if excess == 0 {
                    // Found a potential sibling
                    if let Some(p) = parent {
                        if self.enclose(pos) == Some(p) {
                            return Some(pos);
                        }
                    } else if self.enclose(pos).is_none() {
                        return Some(pos);
                    }
                }
                excess += 1;
            } else {
                excess -= 1;
            }
            
            if pos == 0 {
                break;
            }
            pos -= 1;
        }
        
        None
    }
    
    /// Get parent of node at position i (wrapper around enclose)
    #[inline]
    pub fn parent(&self, i: usize) -> Option<usize> {
        self.enclose(i)
    }
    
    /// Get first child of node at position i
    #[inline]
    pub fn first_child(&self, i: usize) -> Option<usize> {
        self.kth_child(i, 1)
    }
    
    /// Count children of node at position i
    pub fn child_count(&self, i: usize) -> usize {
        if !self.rs.bitvec().get(i) {
            return 0;
        }
        
        let close = match self.find_close(i) {
            Some(c) => c,
            None => return 0,
        };
        
        let mut count = 0;
        let mut pos = i + 1;
        let mut depth = 0;
        
        while pos < close {
            if self.rs.bitvec().get(pos) {
                // Open paren - potential child
                if depth == 0 {
                    // Direct child
                    count += 1;
                }
                depth += 1;
            } else {
                // Close paren
                depth -= 1;
            }
            pos += 1;
        }
        
        count
    }
    
    /// Get the subtree size (number of nodes) rooted at position i
    pub fn subtree_size(&self, i: usize) -> usize {
        if !self.rs.bitvec().get(i) {
            return 0;
        }
        
        if let Some(close) = self.find_close(i) {
            // Number of nodes = number of open parens in range
            self.rs.rank1(close + 1) - self.rs.rank1(i)
        } else {
            0
        }
    }
    
    /// Get underlying bitvector
    pub fn bitvec(&self) -> &BitVec {
        self.rs.bitvec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tree() {
        // Tree: (()())
        // Structure: root with 2 leaf children
        let bp = BP::from_sequence(&[
            true,  // ( root
            true,  // ( child1
            false, // )
            true,  // ( child2  
            false, // )
            false, // ) root
        ]);
        
        // Test find_close
        assert_eq!(bp.find_close(0), Some(5)); // root
        assert_eq!(bp.find_close(1), Some(2)); // child1
        assert_eq!(bp.find_close(3), Some(4)); // child2
        
        // Test parent (enclose)
        assert_eq!(bp.parent(0), None); // root has no parent
        assert_eq!(bp.parent(1), Some(0)); // child1's parent is root
        assert_eq!(bp.parent(3), Some(0)); // child2's parent is root
        
        // Test children
        assert_eq!(bp.first_child(0), Some(1));
        assert_eq!(bp.kth_child(0, 1), Some(1));
        assert_eq!(bp.kth_child(0, 2), Some(3));
        assert_eq!(bp.child_count(0), 2);
        
        // Test siblings
        assert_eq!(bp.next_sibling(1), Some(3));
        assert_eq!(bp.previous_sibling(3), Some(1));
        assert_eq!(bp.next_sibling(3), None);
    }

    #[test]
    fn test_nested_tree() {
        // Tree: ((()()))
        // Structure: root -> child -> 2 grandchildren
        let bp = BP::from_sequence(&[
            true,  // ( root
            true,  // ( child
            true,  // ( grandchild1
            false, // )
            true,  // ( grandchild2
            false, // )
            false, // ) child
            false, // ) root
        ]);
        
        // Test navigation
        assert_eq!(bp.parent(1), Some(0)); // child's parent
        assert_eq!(bp.parent(2), Some(1)); // grandchild1's parent
        assert_eq!(bp.parent(4), Some(1)); // grandchild2's parent
        
        assert_eq!(bp.child_count(0), 1); // root has 1 child
        assert_eq!(bp.child_count(1), 2); // child has 2 children
        
        assert_eq!(bp.subtree_size(0), 3); // root + child + 2 grandchildren
        assert_eq!(bp.subtree_size(1), 2); // child + 2 grandchildren
    }

    #[test]
    fn test_complex_tree() {
        // Tree: (()((())())())
        // Multiple levels and siblings
        let bp = BP::from_sequence(&[
            true,  // 0: ( root
            true,  // 1: ( child1 (leaf)
            false, // 2: )
            true,  // 3: ( child2
            true,  // 4: ( grandchild1
            true,  // 5: ( great-grandchild
            false, // 6: )
            false, // 7: ) grandchild1
            true,  // 8: ( grandchild2 (leaf)
            false, // 9: )
            false, // 10: ) child2
            true,  // 11: ( child3 (leaf)
            false, // 12: )
            false, // 13: ) root
        ]);
        
        // Test root children
        assert_eq!(bp.child_count(0), 3);
        assert_eq!(bp.kth_child(0, 1), Some(1));
        assert_eq!(bp.kth_child(0, 2), Some(3));
        assert_eq!(bp.kth_child(0, 3), Some(11));
        
        // Test siblings
        assert_eq!(bp.next_sibling(1), Some(3));
        assert_eq!(bp.next_sibling(3), Some(11));
        assert_eq!(bp.previous_sibling(11), Some(3));
        
        // Test nested structure
        assert_eq!(bp.parent(5), Some(4));
        assert_eq!(bp.parent(4), Some(3));
        assert_eq!(bp.parent(3), Some(0));
        
        // Test subtree sizes
        assert_eq!(bp.subtree_size(0), 6); // entire tree
        assert_eq!(bp.subtree_size(3), 3); // child2 subtree
        assert_eq!(bp.subtree_size(4), 2); // grandchild1 subtree
    }

    #[test]
    fn test_edge_cases() {
        // Single node tree: ()
        let bp = BP::from_sequence(&[true, false]);
        assert_eq!(bp.find_close(0), Some(1));
        assert_eq!(bp.parent(0), None);
        assert_eq!(bp.child_count(0), 0);
        assert_eq!(bp.subtree_size(0), 1);
        
        // Empty tree
        let bp = BP::from_sequence(&[]);
        assert_eq!(bp.find_close(0), None);
        
        // Forest: ()()()
        let bp = BP::from_sequence(&[
            true, false,  // tree1
            true, false,  // tree2  
            true, false,  // tree3
        ]);
        
        assert_eq!(bp.parent(0), None);
        assert_eq!(bp.parent(2), None);
        assert_eq!(bp.parent(4), None);
        
        // These are siblings at root level
        assert_eq!(bp.next_sibling(0), Some(2));
        assert_eq!(bp.next_sibling(2), Some(4));
        assert_eq!(bp.previous_sibling(4), Some(2));
    }
}
