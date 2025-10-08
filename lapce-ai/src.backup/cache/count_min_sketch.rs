/// CountMinSketch - EXACT implementation from docs lines 168-201
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct CountMinSketch {
    pub counters: Vec<Vec<u32>>,
    pub hash_functions: Vec<Box<dyn Fn(&[u8]) -> usize + Send + Sync>>,
    pub width: usize,
    pub depth: usize,
}

impl CountMinSketch {
    pub fn new(epsilon: f64, delta: f64) -> Self {
        let width = (2.0 / epsilon).ceil() as usize;
        let depth = ((-delta.ln()) / 2.0_f64.ln()).ceil() as usize;
        
        let counters = vec![vec![0; width]; depth];
        
        // Create hash functions
        let mut hash_functions: Vec<Box<dyn Fn(&[u8]) -> usize + Send + Sync>> = Vec::new();
        for i in 0..depth {
            let seed = i;
            hash_functions.push(Box::new(move |data: &[u8]| {
                let mut hasher = DefaultHasher::new();
                seed.hash(&mut hasher);
                data.hash(&mut hasher);
                hasher.finish() as usize
            }));
        }
        
        Self {
            counters,
            hash_functions,
            width,
            depth,
        }
    }
    
    pub fn increment(&mut self) {
        for (i, hash_fn) in self.hash_functions.iter().enumerate() {
            let hash = hash_fn(&[]) % self.width;
            self.counters[i][hash] = self.counters[i][hash].saturating_add(1);
        }
    }
    
    pub fn estimate(&self) -> u32 {
        self.hash_functions.iter()
            .enumerate()
            .map(|(i, hash_fn)| {
                let hash = hash_fn(&[]) % self.width;
                self.counters[i][hash]
            })
            .min()
            .unwrap_or(0)
    }
    
    pub fn decay(&mut self, factor: f32) {
        for row in &mut self.counters {
            for count in row {
                *count = (*count as f32 * factor) as u32;
            }
        }
    }
}
