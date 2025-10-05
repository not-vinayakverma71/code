//! Immutable bitvector with efficient popcount operations
//! Foundation for succinct data structures

use std::fmt;

/// Immutable bitvector optimized for rank/select operations
#[derive(Clone)]
pub struct BitVec {
    /// Packed bits in 64-bit words
    words: Vec<u64>,
    /// Total number of bits
    len: usize,
}

impl BitVec {
    /// Create a new bitvector with specified length, all bits set to 0
    pub fn new(len: usize) -> Self {
        let num_words = (len + 63) / 64;
        Self {
            words: vec![0u64; num_words],
            len,
        }
    }

    /// Create from a vector of bits
    pub fn from_bits(bits: &[bool]) -> Self {
        let len = bits.len();
        let num_words = (len + 63) / 64;
        let mut words = vec![0u64; num_words];
        
        for (i, &bit) in bits.iter().enumerate() {
            if bit {
                let word_idx = i / 64;
                let bit_idx = i % 64;
                words[word_idx] |= 1u64 << bit_idx;
            }
        }
        
        Self { words, len }
    }

    /// Create from raw words and length
    pub fn from_words(words: Vec<u64>, len: usize) -> Self {
        // Clear unused bits in the last word
        let mut words = words;
        if len > 0 {
            let last_word_bits = len % 64;
            if last_word_bits != 0 {
                let mask = (1u64 << last_word_bits) - 1;
                if let Some(last) = words.last_mut() {
                    *last &= mask;
                }
            }
        }
        Self { words, len }
    }

    /// Get the bit at position i
    #[inline]
    pub fn get(&self, i: usize) -> bool {
        if i >= self.len {
            panic!("BitVec index out of bounds: {} >= {}", i, self.len);
        }
        let word_idx = i / 64;
        let bit_idx = i % 64;
        (self.words[word_idx] >> bit_idx) & 1 != 0
    }

    /// Set bit at position i (returns new bitvector - immutable)
    pub fn set(&self, i: usize, value: bool) -> Self {
        if i >= self.len {
            panic!("BitVec index out of bounds: {} >= {}", i, self.len);
        }
        let mut words = self.words.clone();
        let word_idx = i / 64;
        let bit_idx = i % 64;
        
        if value {
            words[word_idx] |= 1u64 << bit_idx;
        } else {
            words[word_idx] &= !(1u64 << bit_idx);
        }
        
        Self { words, len: self.len }
    }

    /// Count the number of 1-bits up to (but not including) position i
    #[inline]
    pub fn rank1(&self, i: usize) -> usize {
        if i == 0 {
            return 0;
        }
        if i > self.len {
            panic!("BitVec rank out of bounds: {} > {}", i, self.len);
        }

        let full_words = i / 64;
        let remaining_bits = i % 64;
        
        // Count 1s in full words
        let mut count = 0;
        for j in 0..full_words {
            count += self.words[j].count_ones() as usize;
        }
        
        // Count 1s in partial word
        if remaining_bits > 0 && full_words < self.words.len() {
            let mask = (1u64 << remaining_bits) - 1;
            count += (self.words[full_words] & mask).count_ones() as usize;
        }
        
        count
    }

    /// Count the number of 0-bits up to (but not including) position i
    #[inline]
    pub fn rank0(&self, i: usize) -> usize {
        i - self.rank1(i)
    }

    /// Find the position of the k-th 1-bit (0-indexed)
    pub fn select1(&self, k: usize) -> Option<usize> {
        if k == 0 {
            return None; // 0-indexed, so k=0 means find first 1
        }
        
        let mut count = 0;
        for (word_idx, &word) in self.words.iter().enumerate() {
            let word_ones = word.count_ones() as usize;
            if count + word_ones >= k {
                // The k-th 1 is in this word
                let target = k - count;
                let mut word = word;
                for bit_idx in 0..64 {
                    if word & 1 != 0 {
                        if target == 1 {
                            let pos = word_idx * 64 + bit_idx;
                            if pos < self.len {
                                return Some(pos);
                            }
                        }
                        count += 1;
                        if count == k {
                            let pos = word_idx * 64 + bit_idx;
                            if pos < self.len {
                                return Some(pos);
                            }
                        }
                    }
                    word >>= 1;
                }
            }
            count += word_ones;
        }
        None
    }

    /// Find the position of the k-th 0-bit (0-indexed)
    pub fn select0(&self, k: usize) -> Option<usize> {
        if k == 0 {
            return None;
        }
        
        let mut count = 0;
        for (word_idx, &word) in self.words.iter().enumerate() {
            let word_zeros = 64 - word.count_ones() as usize;
            let actual_zeros = if word_idx == self.words.len() - 1 {
                // Last word might have fewer bits
                let bits_in_word = if self.len % 64 == 0 { 64 } else { self.len % 64 };
                bits_in_word - word.count_ones() as usize
            } else {
                word_zeros
            };
            
            if count + actual_zeros >= k {
                // The k-th 0 is in this word
                let mut word = word;
                for bit_idx in 0..64 {
                    let pos = word_idx * 64 + bit_idx;
                    if pos >= self.len {
                        break;
                    }
                    if word & 1 == 0 {
                        count += 1;
                        if count == k {
                            return Some(pos);
                        }
                    }
                    word >>= 1;
                }
            }
            count += actual_zeros;
        }
        None
    }

    /// Total number of bits
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Count total number of 1-bits
    pub fn count_ones(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Count total number of 0-bits
    pub fn count_zeros(&self) -> usize {
        self.len - self.count_ones()
    }

    /// Get raw words (for serialization)
    pub fn words(&self) -> &[u64] {
        &self.words
    }
}

impl fmt::Debug for BitVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BitVec[{}](", self.len)?;
        for i in 0..self.len.min(64) {
            write!(f, "{}", if self.get(i) { '1' } else { '0' })?;
        }
        if self.len > 64 {
            write!(f, "...")?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let bv = BitVec::from_bits(&[true, false, true, true, false]);
        assert_eq!(bv.len(), 5);
        assert!(bv.get(0));
        assert!(!bv.get(1));
        assert!(bv.get(2));
        assert!(bv.get(3));
        assert!(!bv.get(4));
    }

    #[test]
    fn test_rank() {
        let bv = BitVec::from_bits(&[true, false, true, true, false, false, true]);
        assert_eq!(bv.rank1(0), 0);
        assert_eq!(bv.rank1(1), 1);
        assert_eq!(bv.rank1(2), 1);
        assert_eq!(bv.rank1(3), 2);
        assert_eq!(bv.rank1(4), 3);
        assert_eq!(bv.rank1(7), 4);
        
        assert_eq!(bv.rank0(0), 0);
        assert_eq!(bv.rank0(1), 0);
        assert_eq!(bv.rank0(2), 1);
        assert_eq!(bv.rank0(7), 3);
    }

    #[test]
    fn test_select() {
        let bv = BitVec::from_bits(&[true, false, true, true, false, false, true]);
        assert_eq!(bv.select1(1), Some(0));
        assert_eq!(bv.select1(2), Some(2));
        assert_eq!(bv.select1(3), Some(3));
        assert_eq!(bv.select1(4), Some(6));
        assert_eq!(bv.select1(5), None);
        
        assert_eq!(bv.select0(1), Some(1));
        assert_eq!(bv.select0(2), Some(4));
        assert_eq!(bv.select0(3), Some(5));
        assert_eq!(bv.select0(4), None);
    }

    #[test]
    fn test_large_bitvector() {
        let mut bits = vec![false; 1000];
        for i in 0..1000 {
            if i % 3 == 0 {
                bits[i] = true;
            }
        }
        
        let bv = BitVec::from_bits(&bits);
        assert_eq!(bv.len(), 1000);
        assert_eq!(bv.count_ones(), 334); // 0, 3, 6, ..., 999
        assert_eq!(bv.count_zeros(), 666);
        
        // Test rank at various positions
        assert_eq!(bv.rank1(100), 34); // 0, 3, 6, ..., 99 -> 34 ones
        assert_eq!(bv.rank1(500), 167); // 0, 3, 6, ..., 498 -> 167 ones
        
        // Test select
        assert_eq!(bv.select1(1), Some(0));
        assert_eq!(bv.select1(2), Some(3));
        assert_eq!(bv.select1(100), Some(297)); // 99 * 3 = 297
    }

    #[test]
    fn test_edge_cases() {
        // Empty bitvector
        let bv = BitVec::new(0);
        assert_eq!(bv.len(), 0);
        assert_eq!(bv.count_ones(), 0);
        assert_eq!(bv.rank1(0), 0);
        
        // Single bit
        let bv = BitVec::from_bits(&[true]);
        assert_eq!(bv.rank1(0), 0);
        assert_eq!(bv.rank1(1), 1);
        assert_eq!(bv.select1(1), Some(0));
        
        // All zeros
        let bv = BitVec::from_bits(&[false; 100]);
        assert_eq!(bv.count_ones(), 0);
        assert_eq!(bv.rank1(50), 0);
        assert_eq!(bv.select1(1), None);
        
        // All ones
        let bv = BitVec::from_bits(&[true; 100]);
        assert_eq!(bv.count_ones(), 100);
        assert_eq!(bv.rank1(50), 50);
        assert_eq!(bv.select1(50), Some(49));
    }

    #[test]
    fn test_word_boundary() {
        // Test around 64-bit word boundaries
        let mut bits = vec![false; 130];
        bits[63] = true;  // Last bit of first word
        bits[64] = true;  // First bit of second word
        bits[127] = true; // Last bit of second word
        bits[128] = true; // First bit of third word
        
        let bv = BitVec::from_bits(&bits);
        assert_eq!(bv.count_ones(), 4);
        assert_eq!(bv.rank1(64), 1);
        assert_eq!(bv.rank1(65), 2);
        assert_eq!(bv.rank1(128), 3);
        assert_eq!(bv.rank1(129), 4);
        
        assert_eq!(bv.select1(1), Some(63));
        assert_eq!(bv.select1(2), Some(64));
        assert_eq!(bv.select1(3), Some(127));
        assert_eq!(bv.select1(4), Some(128));
    }
}
