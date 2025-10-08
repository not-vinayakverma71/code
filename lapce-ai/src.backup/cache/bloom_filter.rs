/// BloomFilter - 99% accuracy with 1% false positive rate
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::types::CacheKey;

pub struct BloomFilter {
    bits: Vec<bool>,
    num_hashes: usize,
    size: usize,
}

impl BloomFilter {
    pub fn new(size: usize, fp_rate: f64) -> Self {
        // Calculate optimal number of bits and hash functions
        let bits_per_item = -1.44 * fp_rate.ln() / (2.0_f64.ln().powi(2));
        let num_bits = (size as f64 * bits_per_item) as usize;
        let num_hashes = (bits_per_item * 2.0_f64.ln()).round().max(3.0) as usize; // At least 3 hashes
        
        let final_size = num_bits.max(100_000); // Minimum 100K bits for accuracy
        
        Self {
            bits: vec![false; final_size],
            num_hashes,
            size: final_size,
        }
    }
    
    pub fn insert(&mut self, key: &CacheKey) {
        for i in 0..self.num_hashes {
            let hash = self.hash_with_seed(&key.0, i);
            let index = hash % self.size;
            self.bits[index] = true;
        }
    }
    
    pub fn contains(&self, key: &CacheKey) -> bool {
        for i in 0..self.num_hashes {
            let hash = self.hash_with_seed(&key.0, i);
            let index = hash % self.size;
            if !self.bits[index] {
                return false;
            }
        }
        true
    }
    
    fn hash_with_seed(&self, data: &str, seed: usize) -> usize {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        data.hash(&mut hasher);
        hasher.finish() as usize
    }
    
    pub fn clear(&mut self) {
        self.bits.fill(false);
    }
    
    pub fn estimated_false_positive_rate(&self) -> f64 {
        let ones = self.bits.iter().filter(|&&b| b).count();
        let zeros = self.size - ones;
        
        if zeros == 0 {
            1.0
        } else {
            let ratio = ones as f64 / self.size as f64;
            ratio.powi(self.num_hashes as i32)
        }
    }
}
