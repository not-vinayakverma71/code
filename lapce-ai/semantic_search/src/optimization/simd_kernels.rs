// SIMD Kernels for High-Performance Vector Operations
// Implements AVX2/AVX-512 acceleration with scalar fallback
// Maintains 0% quality loss with float32 precision

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

use crate::error::{Error, Result};

/// SIMD feature detection
pub struct SimdCapabilities {
    pub has_avx2: bool,
    pub has_avx512: bool,
    pub has_fma: bool,
}

impl SimdCapabilities {
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self {
                has_avx2: is_x86_feature_detected!("avx2"),
                has_avx512: is_x86_feature_detected!("avx512f"),
                has_fma: is_x86_feature_detected!("fma"),
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            Self {
                has_avx2: false,
                has_avx512: false,
                has_fma: false,
            }
        }
    }
}

/// SIMD-accelerated dot product
pub fn dot_product_simd(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let capabilities = SimdCapabilities::detect();
    
    #[cfg(target_arch = "x86_64")]
    {
        if capabilities.has_avx512 {
            unsafe { dot_product_avx512(a, b) }
        } else if capabilities.has_avx2 {
            unsafe { dot_product_avx2(a, b) }
        } else {
            dot_product_scalar(a, b)
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        dot_product_scalar(a, b)
    }
}

/// Scalar fallback for dot product
#[inline(always)]
fn dot_product_scalar(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| x * y)
        .sum()
}

/// AVX2 dot product (8 floats at a time)
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2", enable = "fma")]
unsafe fn dot_product_avx2(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len();
    let mut sum = _mm256_setzero_ps();
    
    // Process 8 elements at a time
    let chunks = len / 8;
    for i in 0..chunks {
        let a_vec = _mm256_loadu_ps(a.as_ptr().add(i * 8));
        let b_vec = _mm256_loadu_ps(b.as_ptr().add(i * 8));
        sum = _mm256_fmadd_ps(a_vec, b_vec, sum);
    }
    
    // Horizontal sum of the AVX register
    let mut result = hsum_ps_avx2(sum);
    
    // Handle remaining elements
    for i in (chunks * 8)..len {
        result += a[i] * b[i];
    }
    
    result
}

/// AVX-512 dot product (16 floats at a time)
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn dot_product_avx512(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len();
    let mut sum = _mm512_setzero_ps();
    
    // Process 16 elements at a time
    let chunks = len / 16;
    for i in 0..chunks {
        let a_vec = _mm512_loadu_ps(a.as_ptr().add(i * 16));
        let b_vec = _mm512_loadu_ps(b.as_ptr().add(i * 16));
        sum = _mm512_fmadd_ps(a_vec, b_vec, sum);
    }
    
    // Reduce to scalar
    let mut result = _mm512_reduce_add_ps(sum);
    
    // Handle remaining elements
    for i in (chunks * 16)..len {
        result += a[i] * b[i];
    }
    
    result
}

/// Horizontal sum for AVX2
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn hsum_ps_avx2(v: __m256) -> f32 {
    let v = _mm256_hadd_ps(v, v);
    let v = _mm256_hadd_ps(v, v);
    let high = _mm256_extractf128_ps(v, 1);
    let low = _mm256_castps256_ps128(v);
    let sum = _mm_add_ps(high, low);
    _mm_cvtss_f32(sum)
}

/// SIMD-accelerated L2 distance squared
pub fn l2_distance_squared_simd(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    
    let capabilities = SimdCapabilities::detect();
    
    #[cfg(target_arch = "x86_64")]
    {
        if capabilities.has_avx512 {
            unsafe { l2_distance_squared_avx512(a, b) }
        } else if capabilities.has_avx2 {
            unsafe { l2_distance_squared_avx2(a, b) }
        } else {
            l2_distance_squared_scalar(a, b)
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        l2_distance_squared_scalar(a, b)
    }
}

/// Scalar L2 distance squared
#[inline(always)]
fn l2_distance_squared_scalar(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum()
}

/// AVX2 L2 distance squared
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2", enable = "fma")]
unsafe fn l2_distance_squared_avx2(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len();
    let mut sum = _mm256_setzero_ps();
    
    let chunks = len / 8;
    for i in 0..chunks {
        let a_vec = _mm256_loadu_ps(a.as_ptr().add(i * 8));
        let b_vec = _mm256_loadu_ps(b.as_ptr().add(i * 8));
        let diff = _mm256_sub_ps(a_vec, b_vec);
        sum = _mm256_fmadd_ps(diff, diff, sum);
    }
    
    let mut result = hsum_ps_avx2(sum);
    
    for i in (chunks * 8)..len {
        let diff = a[i] - b[i];
        result += diff * diff;
    }
    
    result
}

/// AVX-512 L2 distance squared
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn l2_distance_squared_avx512(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len();
    let mut sum = _mm512_setzero_ps();
    
    let chunks = len / 16;
    for i in 0..chunks {
        let a_vec = _mm512_loadu_ps(a.as_ptr().add(i * 16));
        let b_vec = _mm512_loadu_ps(b.as_ptr().add(i * 16));
        let diff = _mm512_sub_ps(a_vec, b_vec);
        sum = _mm512_fmadd_ps(diff, diff, sum);
    }
    
    let mut result = _mm512_reduce_add_ps(sum);
    
    for i in (chunks * 16)..len {
        let diff = a[i] - b[i];
        result += diff * diff;
    }
    
    result
}

/// SIMD-accelerated cosine similarity
pub fn cosine_similarity_simd(a: &[f32], b: &[f32]) -> f32 {
    let dot = dot_product_simd(a, b);
    let norm_a = dot_product_simd(a, a).sqrt();
    let norm_b = dot_product_simd(b, b).sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Batch dot products with SIMD
pub fn batch_dot_products_simd(query: &[f32], vectors: &[Vec<f32>]) -> Vec<f32> {
    vectors
        .iter()
        .map(|v| dot_product_simd(query, v))
        .collect()
}

/// Early termination dot product with threshold
pub fn dot_product_with_early_exit(
    a: &[f32], 
    b: &[f32], 
    threshold: f32
) -> Option<f32> {
    let capabilities = SimdCapabilities::detect();
    
    #[cfg(target_arch = "x86_64")]
    {
        if capabilities.has_avx2 {
            unsafe { dot_product_early_exit_avx2(a, b, threshold) }
        } else {
            dot_product_early_exit_scalar(a, b, threshold)
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        dot_product_early_exit_scalar(a, b, threshold)
    }
}

/// Scalar early exit dot product
fn dot_product_early_exit_scalar(a: &[f32], b: &[f32], threshold: f32) -> Option<f32> {
    let mut sum = 0.0;
    let chunk_size = 64; // Check periodically
    
    for chunk in a.chunks(chunk_size).zip(b.chunks(chunk_size)) {
        for (x, y) in chunk.0.iter().zip(chunk.1.iter()) {
            sum += x * y;
        }
        
        if sum > threshold {
            return None; // Early exit
        }
    }
    
    Some(sum)
}

/// AVX2 early exit dot product
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2", enable = "fma")]
unsafe fn dot_product_early_exit_avx2(
    a: &[f32], 
    b: &[f32], 
    threshold: f32
) -> Option<f32> {
    let len = a.len();
    let mut sum = _mm256_setzero_ps();
    let mut accumulated = 0.0;
    
    // Check every 64 elements
    let check_interval = 8; // 8 AVX registers = 64 floats
    
    for chunk_idx in 0..(len / 64) {
        for i in 0..check_interval {
            let idx = chunk_idx * 64 + i * 8;
            if idx + 8 <= len {
                let a_vec = _mm256_loadu_ps(a.as_ptr().add(idx));
                let b_vec = _mm256_loadu_ps(b.as_ptr().add(idx));
                sum = _mm256_fmadd_ps(a_vec, b_vec, sum);
            }
        }
        
        accumulated += hsum_ps_avx2(sum);
        sum = _mm256_setzero_ps();
        
        if accumulated > threshold {
            return None;
        }
    }
    
    // Handle remaining
    for i in ((len / 64) * 64)..len {
        accumulated += a[i] * b[i];
    }
    
    Some(accumulated)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dot_product_correctness() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = vec![8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];
        
        let scalar_result = dot_product_scalar(&a, &b);
        let simd_result = dot_product_simd(&a, &b);
        
        assert!((scalar_result - simd_result).abs() < 1e-6);
    }
    
    #[test]
    fn test_l2_distance_correctness() {
        let a = vec![1.0; 1536];
        let b = vec![2.0; 1536];
        
        let scalar_result = l2_distance_squared_scalar(&a, &b);
        let simd_result = l2_distance_squared_simd(&a, &b);
        
        assert!((scalar_result - simd_result).abs() < 1e-4);
    }
}
