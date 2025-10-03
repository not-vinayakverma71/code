// SIMD and scalar kernels for exact float32 ops and int8 helpers
// Production-grade with safe scalar fallbacks; SIMD paths can be extended.

#![allow(dead_code)]

pub fn dot_f32(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "dot_f32: length mismatch");
    // Simple scalar fallback; replace with AVX2/AVX-512 when enabled.
    let mut sum = 0.0f32;
    for i in 0..a.len() {
        sum += a[i] * b[i];
    }
    sum
}

pub fn dot_f32_blocked(a: &[f32], b: &[f32], block_size: usize, early_abandon_threshold: Option<f32>, q_block_norms: Option<&[f32]>, x_block_norms: Option<&[f32]>) -> f32 {
    assert_eq!(a.len(), b.len(), "dot_f32_blocked: length mismatch");
    assert!(block_size > 0);

    let n = a.len();
    let mut sum = 0.0f32;
    let num_blocks = (n + block_size - 1) / block_size;

    for blk in 0..num_blocks {
        let start = blk * block_size;
        let end = ((blk + 1) * block_size).min(n);
        // optional prefetch could be added here
        for i in start..end {
            sum += a[i] * b[i];
        }
        if let (Some(th), Some(qn), Some(xn)) = (early_abandon_threshold, q_block_norms, x_block_norms) {
            // Compute a strict UB for the remainder by Cauchy–Schwarz
            let mut rem_ub = 0.0f32;
            for rblk in (blk + 1)..num_blocks {
                rem_ub += qn[rblk] * xn[rblk];
            }
            if sum + rem_ub <= th {
                return sum;
            }
        }
    }
    sum
}

pub fn l2_norm(a: &[f32]) -> f32 {
    let mut s = 0.0f32;
    for &v in a {
        s += v * v;
    }
    s.sqrt()
}

pub fn compute_block_norms(a: &[f32], block_size: usize) -> Vec<f32> {
    assert!(block_size > 0);
    let n = a.len();
    let num_blocks = (n + block_size - 1) / block_size;
    let mut norms = vec![0.0f32; num_blocks];
    for blk in 0..num_blocks {
        let start = blk * block_size;
        let end = ((blk + 1) * block_size).min(n);
        let mut s = 0.0f32;
        for i in start..end {
            s += a[i] * a[i];
        }
        norms[blk] = s.sqrt();
    }
    norms
}
