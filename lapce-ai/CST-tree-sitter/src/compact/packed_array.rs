//! Bit-packed arrays for efficient storage of fixed-width values
//! Stores values using exactly B bits each, minimizing space

use std::fmt;

/// Packed array storing values with exactly B bits each
#[derive(Clone)]
pub struct PackedArray {
    /// Packed data in 64-bit words
    words: Vec<u64>,
    /// Number of bits per value
    bits_per_value: usize,
    /// Number of values stored
    len: usize,
}

impl PackedArray {
    /// Create a new packed array with specified bits per value
    pub fn new(bits_per_value: usize) -> Self {
        assert!(bits_per_value > 0 && bits_per_value <= 64, 
                "bits_per_value must be between 1 and 64");
        Self {
            words: Vec::new(),
            bits_per_value,
            len: 0,
        }
    }
    
    /// Create with capacity for n values
    pub fn with_capacity(bits_per_value: usize, capacity: usize) -> Self {
        assert!(bits_per_value > 0 && bits_per_value <= 64,
                "bits_per_value must be between 1 and 64");
        
        let total_bits = capacity * bits_per_value;
        let num_words = (total_bits + 63) / 64;
        
        Self {
            words: Vec::with_capacity(num_words),
            bits_per_value,
            len: 0,
        }
    }
    
    /// Create from a slice of values
    pub fn from_slice(values: &[u64], bits_per_value: usize) -> Self {
        let mut packed = Self::with_capacity(bits_per_value, values.len());
        for &value in values {
            packed.push(value);
        }
        packed
    }
    
    /// Get value at index
    #[inline]
    pub fn get(&self, index: usize) -> u64 {
        assert!(index < self.len, "Index {} out of bounds for length {}", index, self.len);
        
        if self.bits_per_value == 64 {
            return self.words[index];
        }
        
        let bit_offset = index * self.bits_per_value;
        let word_index = bit_offset / 64;
        let bit_index = bit_offset % 64;
        
        let mask = if self.bits_per_value == 64 {
            !0u64
        } else {
            (1u64 << self.bits_per_value) - 1
        };
        
        if bit_index + self.bits_per_value <= 64 {
            // Value fits in single word
            (self.words[word_index] >> bit_index) & mask
        } else {
            // Value spans two words
            let bits_from_first = 64 - bit_index;
            let bits_from_second = self.bits_per_value - bits_from_first;
            
            let first_part = self.words[word_index] >> bit_index;
            let second_part = if word_index + 1 < self.words.len() {
                (self.words[word_index + 1] & ((1u64 << bits_from_second) - 1)) << bits_from_first
            } else {
                0
            };
            
            (first_part | second_part) & mask
        }
    }
    
    /// Set value at index
    pub fn set(&mut self, index: usize, value: u64) {
        assert!(index < self.len, "Index {} out of bounds for length {}", index, self.len);
        
        let mask = if self.bits_per_value == 64 {
            !0u64
        } else {
            (1u64 << self.bits_per_value) - 1
        };
        
        let value = value & mask; // Ensure value fits in bits_per_value bits
        
        if self.bits_per_value == 64 {
            self.words[index] = value;
            return;
        }
        
        let bit_offset = index * self.bits_per_value;
        let word_index = bit_offset / 64;
        let bit_index = bit_offset % 64;
        
        if bit_index + self.bits_per_value <= 64 {
            // Value fits in single word
            let clear_mask = !(mask << bit_index);
            self.words[word_index] = (self.words[word_index] & clear_mask) | (value << bit_index);
        } else {
            // Value spans two words
            let bits_from_first = 64 - bit_index;
            let bits_from_second = self.bits_per_value - bits_from_first;
            
            // Update first word
            let first_mask = !(!0u64 << bit_index);
            self.words[word_index] = (self.words[word_index] & first_mask) | (value << bit_index);
            
            // Update second word
            if word_index + 1 < self.words.len() {
                let second_mask = !((1u64 << bits_from_second) - 1);
                self.words[word_index + 1] = (self.words[word_index + 1] & second_mask) 
                                            | (value >> bits_from_first);
            }
        }
    }
    
    /// Push a value to the end
    pub fn push(&mut self, value: u64) {
        let mask = if self.bits_per_value == 64 {
            !0u64
        } else {
            (1u64 << self.bits_per_value) - 1
        };
        
        assert!(value <= mask, 
                "Value {} exceeds maximum for {} bits", value, self.bits_per_value);
        
        let bit_offset = self.len * self.bits_per_value;
        let word_index = bit_offset / 64;
        let bit_index = bit_offset % 64;
        
        // Ensure we have enough words
        while self.words.len() <= word_index + 1 {
            self.words.push(0);
        }
        
        if bit_index + self.bits_per_value <= 64 {
            // Value fits in single word
            self.words[word_index] |= value << bit_index;
        } else {
            // Value spans two words
            let bits_from_first = 64 - bit_index;
            self.words[word_index] |= value << bit_index;
            self.words[word_index + 1] = value >> bits_from_first;
        }
        
        self.len += 1;
    }
    
    /// Number of values
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
    
    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    
    /// Bits per value
    #[inline]
    pub fn bits_per_value(&self) -> usize {
        self.bits_per_value
    }
    
    /// Memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        self.words.len() * 8
    }
    
    /// Space efficiency (% of theoretical minimum)
    pub fn efficiency(&self) -> f64 {
        if self.len == 0 {
            return 0.0;
        }
        
        let theoretical_bits = self.len * self.bits_per_value;
        let actual_bits = self.words.len() * 64;
        
        (theoretical_bits as f64 / actual_bits as f64) * 100.0
    }
    
    /// Create iterator over values
    pub fn iter(&self) -> PackedArrayIter {
        PackedArrayIter {
            array: self,
            index: 0,
        }
    }
    
    /// Clear all values
    pub fn clear(&mut self) {
        self.words.clear();
        self.len = 0;
    }
}

/// Iterator over packed array values
pub struct PackedArrayIter<'a> {
    array: &'a PackedArray,
    index: usize,
}

impl<'a> Iterator for PackedArrayIter<'a> {
    type Item = u64;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.array.len() {
            let value = self.array.get(self.index);
            self.index += 1;
            Some(value)
        } else {
            None
        }
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.array.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl fmt::Debug for PackedArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PackedArray[{} bits, {} values](", self.bits_per_value, self.len)?;
        for (i, value) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            if i >= 10 {
                write!(f, "...")?;
                break;
            }
            write!(f, "{}", value)?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut pa = PackedArray::new(5);
        
        // 5-bit values can store 0-31
        pa.push(0);
        pa.push(15);
        pa.push(31);
        pa.push(7);
        
        assert_eq!(pa.len(), 4);
        assert_eq!(pa.get(0), 0);
        assert_eq!(pa.get(1), 15);
        assert_eq!(pa.get(2), 31);
        assert_eq!(pa.get(3), 7);
        
        // Test set
        pa.set(1, 20);
        assert_eq!(pa.get(1), 20);
    }

    #[test]
    fn test_various_bit_widths() {
        for bits in 1..=64 {
            let max_value = if bits == 64 {
                !0u64
            } else {
                (1u64 << bits) - 1
            };
            
            let mut pa = PackedArray::new(bits);
            pa.push(0);
            pa.push(max_value);
            pa.push(max_value / 2);
            
            assert_eq!(pa.get(0), 0);
            assert_eq!(pa.get(1), max_value);
            assert_eq!(pa.get(2), max_value / 2);
        }
    }

    #[test]
    fn test_word_boundary_crossing() {
        // 10-bit values will cross word boundaries
        let mut pa = PackedArray::new(10);
        
        // Push enough values to cross multiple word boundaries
        let values: Vec<u64> = (0..20).map(|i| i * 3).collect();
        
        for &v in &values {
            pa.push(v);
        }
        
        for (i, &expected) in values.iter().enumerate() {
            assert_eq!(pa.get(i), expected, "Mismatch at index {}", i);
        }
    }

    #[test]
    fn test_from_slice() {
        let values = vec![10, 20, 30, 40, 50];
        let pa = PackedArray::from_slice(&values, 6);
        
        assert_eq!(pa.len(), 5);
        for (i, &v) in values.iter().enumerate() {
            assert_eq!(pa.get(i), v);
        }
    }

    #[test]
    fn test_efficiency() {
        // 3-bit values in array of 100 elements
        let mut pa = PackedArray::new(3);
        for _i in 0..100 {
            pa.push(i % 8);
        }
        
        let efficiency = pa.efficiency();
        assert!(efficiency > 70.0, "Efficiency {} is too low", efficiency);
    }

    #[test]
    fn test_iterator() {
        let values = vec![5, 10, 15, 20, 25];
        let pa = PackedArray::from_slice(&values, 5);
        
        let collected: Vec<u64> = pa.iter().collect();
        assert_eq!(collected, values);
    }

    #[test]
    fn test_edge_cases() {
        // 1-bit array (basically a bitvector)
        let mut pa = PackedArray::new(1);
        pa.push(1);
        pa.push(0);
        pa.push(1);
        pa.push(1);
        
        assert_eq!(pa.get(0), 1);
        assert_eq!(pa.get(1), 0);
        assert_eq!(pa.get(2), 1);
        assert_eq!(pa.get(3), 1);
        
        // 64-bit array (full word per value)
        let mut pa = PackedArray::new(64);
        pa.push(!0u64);
        pa.push(0);
        pa.push(0x123456789ABCDEF0);
        
        assert_eq!(pa.get(0), !0u64);
        assert_eq!(pa.get(1), 0);
        assert_eq!(pa.get(2), 0x123456789ABCDEF0);
    }

    #[test]
    #[should_panic(expected = "Value 32 exceeds maximum for 5 bits")]
    fn test_value_overflow() {
        let mut pa = PackedArray::new(5);
        pa.push(32); // Max for 5 bits is 31
    }
}
