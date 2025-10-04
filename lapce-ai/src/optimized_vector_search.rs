/// Optimized Vector Search Implementation
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::{Result, anyhow};

pub struct OptimizedVectorSearch {
    dimension: usize,
    vectors: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    index: Arc<RwLock<Vec<(String, Vec<f32>)>>>,
}

impl OptimizedVectorSearch {
    pub fn new(dimension: usize) -> Result<Self> {
        if dimension == 0 {
            return Err(anyhow!("Vector dimension must be greater than 0"));
        }
        
        Ok(Self {
            dimension,
            vectors: Arc::new(RwLock::new(HashMap::new())),
            index: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub fn add(&mut self, id: String, vector: Vec<f32>) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(anyhow!("Vector dimension mismatch: expected {}, got {}", 
                self.dimension, vector.len()));
        }
        
        let mut vectors = self.vectors.write();
        let mut index = self.index.write();
        
        vectors.insert(id.clone(), vector.clone());
        index.push((id, vector));
        Ok(())
    }

    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        if query.len() != self.dimension {
            return Err(anyhow!("Query dimension mismatch: expected {}, got {}", 
                self.dimension, query.len()));
        }
        
        let index = self.index.read();
        let mut scores: Vec<(String, f32)> = index
            .iter()
            .map(|(id, vec)| {
                let score = Self::cosine_similarity(query, vec);
                (id.clone(), score)
            })
            .collect();
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.truncate(k);
        Ok(scores)
    }
    
    #[cfg(target_arch = "x86_64")]
    pub fn search_simd(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        // SIMD-optimized version for x86_64
        if query.len() != self.dimension {
            return Err(anyhow!("Query dimension mismatch: expected {}, got {}", 
                self.dimension, query.len()));
        }
        
        let index = self.index.read();
        let mut scores: Vec<(String, f32)> = index
            .iter()
            .map(|(id, vec)| {
                let score = Self::cosine_similarity_simd(query, vec);
                (id.clone(), score)
            })
            .collect();
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.truncate(k);
        Ok(scores)
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    pub fn search_simd(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        // Fallback to regular search on non-x86_64
        self.search(query, k)
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        dot / (norm_a * norm_b)
    }
    
    #[cfg(target_arch = "x86_64")]
    fn cosine_similarity_simd(a: &[f32], b: &[f32]) -> f32 {
        use std::arch::x86_64::*;
        
        unsafe {
            let mut dot = 0.0f32;
            let mut norm_a = 0.0f32;
            let mut norm_b = 0.0f32;
            
            let len = a.len();
            let simd_len = len - (len % 8);
            
            // Process 8 floats at a time using AVX
            let mut i = 0;
            while i < simd_len {
                let a_vec = _mm256_loadu_ps(a.as_ptr().add(i));
                let b_vec = _mm256_loadu_ps(b.as_ptr().add(i));
                
                let dot_vec = _mm256_mul_ps(a_vec, b_vec);
                let norm_a_vec = _mm256_mul_ps(a_vec, a_vec);
                let norm_b_vec = _mm256_mul_ps(b_vec, b_vec);
                
                // Sum the vectors
                let mut dot_arr = [0.0f32; 8];
                let mut norm_a_arr = [0.0f32; 8];
                let mut norm_b_arr = [0.0f32; 8];
                
                _mm256_storeu_ps(dot_arr.as_mut_ptr(), dot_vec);
                _mm256_storeu_ps(norm_a_arr.as_mut_ptr(), norm_a_vec);
                _mm256_storeu_ps(norm_b_arr.as_mut_ptr(), norm_b_vec);
                
                for j in 0..8 {
                    dot += dot_arr[j];
                    norm_a += norm_a_arr[j];
                    norm_b += norm_b_arr[j];
                }
                
                i += 8;
            }
            
            // Process remaining elements
            for j in simd_len..len {
                dot += a[j] * b[j];
                norm_a += a[j] * a[j];
                norm_b += b[j] * b[j];
            }
            
            norm_a = norm_a.sqrt();
            norm_b = norm_b.sqrt();
            
            if norm_a == 0.0 || norm_b == 0.0 {
                return 0.0;
            }
            
            dot / (norm_a * norm_b)
        }
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    fn cosine_similarity_simd(a: &[f32], b: &[f32]) -> f32 {
        // Fallback to regular implementation on non-x86_64
        Self::cosine_similarity(a, b)
    }

    pub fn size(&self) -> usize {
        self.vectors.read().len()
    }

    pub fn clear(&mut self) {
        self.vectors.write().clear();
        self.index.write().clear();
    }
}
