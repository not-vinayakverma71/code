//! O(1) rank and select operations with auxiliary indexes
//! Uses two-level indexing: superblocks (512 bits) and blocks (64 bits)

use super::bitvec::BitVec;

/// Rank/Select structure for O(1) operations
#[derive(Clone)]
pub struct RankSelect {
    /// Original bitvector
    bitvec: BitVec,
    
    /// Superblock ranks (every 512 bits)
    /// Stores cumulative rank up to the start of each superblock
    superblocks: Vec<u32>,
    
    /// Block ranks within superblocks (every 64 bits)
    /// Stores rank relative to containing superblock (uses 9 bits each)
    blocks: Vec<u16>,
    
    /// Select samples for O(log n) select
    /// Stores position of every K-th one-bit
    select_samples: Vec<u32>,
    
    /// Sample interval for select (typically sqrt(n))
    select_sample_interval: usize,
}

impl RankSelect {
    const SUPERBLOCK_SIZE: usize = 512; // bits
    const BLOCK_SIZE: usize = 64;       // bits (word size)
    const BLOCKS_PER_SUPERBLOCK: usize = 8;
    
    /// Build rank/select index from bitvector
    pub fn new(bitvec: BitVec) -> Self {
        let n = bitvec.len();
        let num_superblocks = (n + Self::SUPERBLOCK_SIZE - 1) / Self::SUPERBLOCK_SIZE;
        let num_blocks = (n + Self::BLOCK_SIZE - 1) / Self::BLOCK_SIZE;
        
        let mut superblocks = Vec::with_capacity(num_superblocks);
        let mut blocks = Vec::with_capacity(num_blocks);
        
        let mut cumulative_rank = 0u32;
        let mut superblock_rank = 0u32;
        
        // Build rank indexes
        for i in 0..num_blocks {
            let block_start = i * Self::BLOCK_SIZE;
            let block_end = ((i + 1) * Self::BLOCK_SIZE).min(n);
            
            // Start of new superblock?
            if i % Self::BLOCKS_PER_SUPERBLOCK == 0 {
                superblocks.push(cumulative_rank);
                superblock_rank = cumulative_rank;
            }
            
            // Store block rank relative to superblock
            let relative_rank = cumulative_rank - superblock_rank;
            blocks.push(relative_rank as u16);
            
            // Count ones in this block
            if block_start < n {
                let block_ones = bitvec.rank1(block_end) - bitvec.rank1(block_start);
                cumulative_rank += block_ones as u32;
            }
        }
        
        // Build select samples
        let total_ones = bitvec.count_ones();
        let select_sample_interval = ((n as f64).sqrt() as usize).max(1);
        let mut select_samples = Vec::new();
        
        let mut current_rank = select_sample_interval;
        while current_rank <= total_ones {
            // Binary search for position of current_rank-th one
            let pos = Self::binary_search_select(&bitvec, current_rank);
            if let Some(p) = pos {
                select_samples.push(p as u32);
            }
            current_rank += select_sample_interval;
        }
        
        Self {
            bitvec,
            superblocks,
            blocks,
            select_samples,
            select_sample_interval,
        }
    }
    
    /// Count ones up to position i (exclusive)
    #[inline]
    pub fn rank1(&self, i: usize) -> usize {
        if i == 0 {
            return 0;
        }
        if i > self.bitvec.len() {
            panic!("rank1: position {} exceeds length {}", i, self.bitvec.len());
        }
        
        let superblock_idx = i / Self::SUPERBLOCK_SIZE;
        let block_idx = i / Self::BLOCK_SIZE;
        let bit_offset = i % Self::BLOCK_SIZE;
        
        // Get cumulative rank from indexes
        let mut rank = 0usize;
        
        // Add superblock rank
        if superblock_idx < self.superblocks.len() {
            rank += self.superblocks[superblock_idx] as usize;
        }
        
        // Add block rank (relative to superblock)
        if block_idx < self.blocks.len() {
            let superblock_start_block = (superblock_idx * Self::BLOCKS_PER_SUPERBLOCK);
            if block_idx > superblock_start_block {
                rank += self.blocks[block_idx] as usize;
            }
        }
        
        // Add remaining bits in current block
        if bit_offset > 0 {
            let block_start = block_idx * Self::BLOCK_SIZE;
            let partial_rank = self.bitvec.rank1(block_start + bit_offset) 
                             - self.bitvec.rank1(block_start);
            rank += partial_rank;
        }
        
        rank
    }
    
    /// Count zeros up to position i (exclusive)
    #[inline]
    pub fn rank0(&self, i: usize) -> usize {
        i - self.rank1(i)
    }
    
    /// Find position of k-th one (1-indexed)
    pub fn select1(&self, k: usize) -> Option<usize> {
        if k == 0 || k > self.bitvec.count_ones() {
            return None;
        }
        
        // Use samples to narrow search range
        let sample_idx = (k - 1) / self.select_sample_interval;
        let mut left = if sample_idx > 0 && sample_idx <= self.select_samples.len() {
            self.select_samples[sample_idx - 1] as usize
        } else {
            0
        };
        
        let mut right = if sample_idx < self.select_samples.len() {
            self.select_samples[sample_idx] as usize
        } else {
            self.bitvec.len()
        };
        
        // Binary search in narrowed range
        while left < right {
            let mid = left + (right - left) / 2;
            let rank_mid = self.rank1(mid + 1);
            
            if rank_mid < k {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        
        // Verify result
        if left < self.bitvec.len() && self.bitvec.get(left) && self.rank1(left + 1) == k {
            Some(left)
        } else {
            None
        }
    }
    
    /// Find position of k-th zero (1-indexed)
    pub fn select0(&self, k: usize) -> Option<usize> {
        if k == 0 || k > self.bitvec.count_zeros() {
            return None;
        }
        
        // Binary search for position
        let mut left = 0;
        let mut right = self.bitvec.len();
        
        while left < right {
            let mid = left + (right - left) / 2;
            let rank_mid = self.rank0(mid + 1);
            
            if rank_mid < k {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        
        // Verify result
        if left < self.bitvec.len() && !self.bitvec.get(left) && self.rank0(left + 1) == k {
            Some(left)
        } else {
            None
        }
    }
    
    /// Helper for building select samples
    fn binary_search_select(bitvec: &BitVec, k: usize) -> Option<usize> {
        let mut left = 0;
        let mut right = bitvec.len();
        
        while left < right {
            let mid = left + (right - left) / 2;
            let rank_mid = bitvec.rank1(mid + 1);
            
            if rank_mid < k {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        
        if left < bitvec.len() && bitvec.get(left) {
            Some(left)
        } else {
            None
        }
    }
    
    /// Get underlying bitvector
    pub fn bitvec(&self) -> &BitVec {
        &self.bitvec
    }
    
    /// Memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        let bitvec_bytes = (self.bitvec.len() + 7) / 8;
        let superblock_bytes = self.superblocks.len() * 4;
        let block_bytes = self.blocks.len() * 2;
        let select_bytes = self.select_samples.len() * 4;
        
        bitvec_bytes + superblock_bytes + block_bytes + select_bytes
    }
    
    /// Space overhead as percentage of original bitvector
    pub fn overhead_percent(&self) -> f64 {
        let bitvec_bytes = (self.bitvec.len() + 7) / 8;
        let index_bytes = self.memory_bytes() - bitvec_bytes;
        
        (index_bytes as f64 / bitvec_bytes as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rank_basic() {
        let bv = BitVec::from_bits(&[true, false, true, true, false, false, true]);
        let rs = RankSelect::new(bv);
        
        assert_eq!(rs.rank1(0), 0);
        assert_eq!(rs.rank1(1), 1);
        assert_eq!(rs.rank1(2), 1);
        assert_eq!(rs.rank1(3), 2);
        assert_eq!(rs.rank1(4), 3);
        assert_eq!(rs.rank1(7), 4);
        
        assert_eq!(rs.rank0(0), 0);
        assert_eq!(rs.rank0(1), 0);
        assert_eq!(rs.rank0(2), 1);
        assert_eq!(rs.rank0(7), 3);
    }

    #[test]
    fn test_select_basic() {
        let bv = BitVec::from_bits(&[true, false, true, true, false, false, true]);
        let rs = RankSelect::new(bv);
        
        assert_eq!(rs.select1(1), Some(0));
        assert_eq!(rs.select1(2), Some(2));
        assert_eq!(rs.select1(3), Some(3));
        assert_eq!(rs.select1(4), Some(6));
        assert_eq!(rs.select1(5), None);
        
        assert_eq!(rs.select0(1), Some(1));
        assert_eq!(rs.select0(2), Some(4));
        assert_eq!(rs.select0(3), Some(5));
        assert_eq!(rs.select0(4), None);
    }

    #[test]
    fn test_large_bitvector() {
        // Create large bitvector with pattern
        let mut bits = vec![false; 10000];
        for i in 0..10000 {
            if i % 3 == 0 || i % 7 == 0 {
                bits[i] = true;
            }
        }
        
        let bv = BitVec::from_bits(&bits);
        let rs = RankSelect::new(bv.clone());
        
        // Compare with naive implementation
        for i in (0..10000).step_by(100) {
            assert_eq!(rs.rank1(i), bv.rank1(i), "rank1({}) mismatch", i);
            assert_eq!(rs.rank0(i), bv.rank0(i), "rank0({}) mismatch", i);
        }
        
        // Test select operations
        for k in (1..100).step_by(10) {
            assert_eq!(rs.select1(k), bv.select1(k), "select1({}) mismatch", k);
            assert_eq!(rs.select0(k), bv.select0(k), "select0({}) mismatch", k);
        }
        
        // Check space overhead
        let overhead = rs.overhead_percent();
        assert!(overhead < 5.0, "Space overhead too high: {:.2}%", overhead);
    }

    #[test]
    fn test_edge_cases() {
        // Empty
        let bv = BitVec::new(0);
        let rs = RankSelect::new(bv);
        assert_eq!(rs.rank1(0), 0);
        assert_eq!(rs.select1(1), None);
        
        // All zeros
        let bv = BitVec::from_bits(&[false; 1000]);
        let rs = RankSelect::new(bv);
        assert_eq!(rs.rank1(500), 0);
        assert_eq!(rs.select1(1), None);
        assert_eq!(rs.select0(500), Some(499));
        
        // All ones
        let bv = BitVec::from_bits(&[true; 1000]);
        let rs = RankSelect::new(bv);
        assert_eq!(rs.rank1(500), 500);
        assert_eq!(rs.rank0(500), 0);
        assert_eq!(rs.select1(500), Some(499));
        assert_eq!(rs.select0(1), None);
    }

    #[test]
    fn test_superblock_boundaries() {
        // Test around superblock boundaries (512 bits)
        let mut bits = vec![false; 2048];
        
        // Set bits at boundaries
        bits[511] = true;  // Last bit of first superblock
        bits[512] = true;  // First bit of second superblock
        bits[1023] = true; // Last bit of second superblock
        bits[1024] = true; // First bit of third superblock
        
        let bv = BitVec::from_bits(&bits);
        let rs = RankSelect::new(bv);
        
        assert_eq!(rs.rank1(512), 1);
        assert_eq!(rs.rank1(513), 2);
        assert_eq!(rs.rank1(1024), 3);
        assert_eq!(rs.rank1(1025), 4);
        
        assert_eq!(rs.select1(1), Some(511));
        assert_eq!(rs.select1(2), Some(512));
        assert_eq!(rs.select1(3), Some(1023));
        assert_eq!(rs.select1(4), Some(1024));
    }
}
