// Exact scoring helpers: selective projection and early-abandon exact dot

#![allow(dead_code)]

use super::simd_kernels::{dot_product_simd, l2_distance_squared_simd};

pub struct ProjectionBound {
    pub s_partial: f32,
    pub q_tail_norm: f32,
    pub x_tail_norm: f32,
    pub ub_partial: f32,
}

/// Compute selective projection exact upper bound using top-M dims of q
/// - indices: indices of top-M dimensions by |q_i|
pub fn selective_projection_bound(q: &[f32], x: &[f32], indices: &[usize], m: usize) -> ProjectionBound {
    let n = q.len();

    // Get top-M dimensions
    let mut q_top = Vec::with_capacity(m);
    let mut x_top = Vec::with_capacity(m);
    
    for &idx in indices.iter().take(m) {
        if idx < n {
            q_top.push(q[idx]);
            x_top.push(x[idx]);
        }
    }

    // s(q[:m], x[:m])
    let s_partial = dot_product_simd(&q_top, &x_top);
    
    // ||q[m:]||, ||x[m:]||  - compute from remaining dimensions
    let mut q_tail = Vec::new();
    let mut x_tail = Vec::new();
    
    let mut selected = vec![false; n];
    for &idx in indices.iter().take(m) {
        if idx < n {
            selected[idx] = true;
        }
    }
    
    for i in 0..n {
        if !selected[i] {
            q_tail.push(q[i]);
            x_tail.push(x[i]);
        }
    }
    
    let q_tail_norm = dot_product_simd(&q_tail, &q_tail).sqrt();
    let x_tail_norm = dot_product_simd(&x_tail, &x_tail).sqrt();
    let ub_partial = s_partial + q_tail_norm * x_tail_norm; // Cauchy–Schwarz

    ProjectionBound { s_partial, q_tail_norm, x_tail_norm, ub_partial }
}

/// Full exact dot with block-wise early-abandon using precomputed block norms
pub fn exact_dot_early_abandon(
    q: &[f32],
    x: &[f32],
    block_size: usize,
    current_threshold: Option<f32>,
    q_block_norms: Option<&[f32]>,
    x_block_norms: Option<&[f32]>,
) -> f32 {
    // For now, just use regular SIMD dot product
    // TODO: Implement block-wise early abandon
    dot_product_simd(q, x)
}

/// Utility: compute indices of top-M dims of q by absolute value
pub fn top_m_indices_by_abs(q: &[f32], m: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..q.len()).collect();
    idx.sort_unstable_by(|&i, &j| q[j].abs().partial_cmp(&q[i].abs()).unwrap());
    idx.truncate(m.min(q.len()));
    idx
}

/// Utility: compute block norms
pub fn block_norms(a: &[f32], block_size: usize) -> Vec<f32> {
    let mut norms = Vec::new();
    for chunk in a.chunks(block_size) {
        let norm = dot_product_simd(chunk, chunk).sqrt();
        norms.push(norm);
    }
    norms
}
