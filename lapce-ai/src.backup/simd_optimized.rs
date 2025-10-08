// DAY 25: SIMD Optimizations for Ultra-High Performance
// Target: Further improve already excellent performance with SIMD

use std::arch::x86_64::*;
use std::mem;

/// SIMD-optimized message copying
#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub unsafe fn simd_copy(dst: &mut [u8], src: &[u8]) {
    if dst.len() != src.len() || dst.len() < 32 {
        // Fallback to regular copy for small sizes
        dst.copy_from_slice(src);
        return;
    }
    
    let len = dst.len();
    let mut i = 0;
    
    // Use AVX2 for 32-byte chunks
    if is_x86_feature_detected!("avx2") {
        while i + 32 <= len {
            let src_ptr = src.as_ptr().add(i) as *const __m256i;
            let dst_ptr = dst.as_mut_ptr().add(i) as *mut __m256i;
            let data = _mm256_loadu_si256(src_ptr);
            _mm256_storeu_si256(dst_ptr, data);
            i += 32;
        }
    }
    
    // Use SSE2 for 16-byte chunks
    while i + 16 <= len {
        let src_ptr = src.as_ptr().add(i) as *const __m128i;
        let dst_ptr = dst.as_mut_ptr().add(i) as *mut __m128i;
        let data = _mm_loadu_si128(src_ptr);
        _mm_storeu_si128(dst_ptr, data);
        i += 16;
    }
    
    // Copy remaining bytes
    while i < len {
        dst[i] = src[i];
        i += 1;
    }
}

/// SIMD-optimized XOR for encryption/checksums
#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub unsafe fn simd_xor(data: &mut [u8], key: &[u8]) {
    if data.len() < 32 || key.is_empty() {
        // Fallback for small sizes
        for i in 0..data.len() {
            data[i] ^= key[i % key.len()];
        }
        return;
    }
    
    let mut i = 0;
    let len = data.len();
    
    // Prepare key pattern
    let key_pattern = if key.len() >= 32 {
        key[..32].to_vec()
    } else {
        let mut pattern = Vec::with_capacity(32);
        while pattern.len() < 32 {
            pattern.extend_from_slice(&key[..(32 - pattern.len()).min(key.len())]);
        }
        pattern
    };
    
    // AVX2 processing
    if is_x86_feature_detected!("avx2") {
        let key_vec = _mm256_loadu_si256(key_pattern.as_ptr() as *const __m256i);
        
        while i + 32 <= len {
            let data_ptr = data.as_mut_ptr().add(i) as *mut __m256i;
            let data_vec = _mm256_loadu_si256(data_ptr);
            let result = _mm256_xor_si256(data_vec, key_vec);
            _mm256_storeu_si256(data_ptr, result);
            i += 32;
        }
    }
    
    // Handle remainder
    while i < len {
        data[i] ^= key_pattern[i % 32];
        i += 1;
    }
}

/// SIMD-optimized memory comparison
#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub unsafe fn simd_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let len = a.len();
    let mut i = 0;
    
    // AVX2 comparison for 32-byte chunks
    if is_x86_feature_detected!("avx2") {
        while i + 32 <= len {
            let a_vec = _mm256_loadu_si256(a.as_ptr().add(i) as *const __m256i);
            let b_vec = _mm256_loadu_si256(b.as_ptr().add(i) as *const __m256i);
            let cmp = _mm256_cmpeq_epi8(a_vec, b_vec);
            let mask = _mm256_movemask_epi8(cmp);
            if mask != -1 {
                return false;
            }
            i += 32;
        }
    }
    
    // SSE2 for 16-byte chunks
    while i + 16 <= len {
        let a_vec = _mm_loadu_si128(a.as_ptr().add(i) as *const __m128i);
        let b_vec = _mm_loadu_si128(b.as_ptr().add(i) as *const __m128i);
        let cmp = _mm_cmpeq_epi8(a_vec, b_vec);
        let mask = _mm_movemask_epi8(cmp);
        if mask != 0xFFFF {
            return false;
        }
        i += 16;
    }
    
    // Compare remaining bytes
    while i < len {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    
    true
}

/// SIMD-optimized memory zeroing
#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub unsafe fn simd_zero(data: &mut [u8]) {
    let len = data.len();
    let mut i = 0;
    
    // AVX2 zeroing for 32-byte chunks
    if is_x86_feature_detected!("avx2") {
        let zero = _mm256_setzero_si256();
        
        while i + 32 <= len {
            let ptr = data.as_mut_ptr().add(i) as *mut __m256i;
            _mm256_storeu_si256(ptr, zero);
            i += 32;
        }
    }
    
    // SSE2 for 16-byte chunks
    let zero = _mm_setzero_si128();
    while i + 16 <= len {
        let ptr = data.as_mut_ptr().add(i) as *mut __m128i;
        _mm_storeu_si128(ptr, zero);
        i += 16;
    }
    
    // Zero remaining bytes
    while i < len {
        data[i] = 0;
        i += 1;
    }
}

/// Cache-line aligned buffer for optimal performance
#[repr(align(64))] // 64-byte cache line alignment
pub struct AlignedBuffer {
    data: [u8; 4096],
}

impl AlignedBuffer {
    #[inline(always)]
    pub const fn new() -> Self {
        Self { data: [0u8; 4096] }
    }
    
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
    
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

/// Branch prediction hints
#[inline(always)]
pub fn likely(b: bool) -> bool {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        std::intrinsics::likely(b)
    }
    #[cfg(not(target_arch = "x86_64"))]
    b
}

#[inline(always)]
pub fn unlikely(b: bool) -> bool {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        std::intrinsics::unlikely(b)
    }
    #[cfg(not(target_arch = "x86_64"))]
    b
}

/// Prefetch data for better cache utilization
#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub unsafe fn prefetch_read(addr: *const u8) {
    _mm_prefetch(addr as *const i8, _MM_HINT_T0);
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub unsafe fn prefetch_write(addr: *const u8) {
    _mm_prefetch(addr as *const i8, _MM_HINT_ET0);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simd_copy() {
        let src = vec![1u8; 1024];
        let mut dst = vec![0u8; 1024];
        
        unsafe {
            simd_copy(&mut dst, &src);
        }
        
        assert_eq!(dst, src);
    }
    
    #[test]
    fn test_simd_xor() {
        let mut data = vec![0xFFu8; 64];
        let key = vec![0xAAu8; 8];
        
        unsafe {
            simd_xor(&mut data, &key);
        }
        
        for byte in data {
            assert_eq!(byte, 0xFF ^ 0xAA);
        }
    }
    
    #[test]
    fn test_simd_compare() {
        let a = vec![1u8; 256];
        let b = vec![1u8; 256];
        let c = vec![2u8; 256];
        
        unsafe {
            assert!(simd_compare(&a, &b));
            assert!(!simd_compare(&a, &c));
        }
    }
    
    #[test]
    fn test_aligned_buffer() {
        let buffer = AlignedBuffer::new();
        let addr = buffer.as_slice().as_ptr() as usize;
        assert_eq!(addr % 64, 0); // Check 64-byte alignment
    }
    
    #[test]
    fn test_simd_performance() {
        use std::time::Instant;
        
        let src = vec![1u8; 1_000_000];
        let mut dst = vec![0u8; 1_000_000];
        
        // Regular copy
        let start = Instant::now();
        dst.copy_from_slice(&src);
        let regular_time = start.elapsed();
        
        // SIMD copy
        let start = Instant::now();
        unsafe {
            simd_copy(&mut dst, &src);
        }
        let simd_time = start.elapsed();
        
        println!("Regular: {:?}, SIMD: {:?}", regular_time, simd_time);
        // SIMD should be faster for large buffers
    }
}
