// Int8 quantization and fast dot helpers with exact error bound support

#![allow(dead_code)]

pub fn quantize_per_vector_i8(x: &[f32]) -> (Vec<i8>, f32, f32) {
    // symmetric per-vector scaling to maximize dynamic range
    let mut max_abs = 0.0f32;
    for &v in x { max_abs = max_abs.max(v.abs()); }
    let scale = if max_abs > 0.0 { max_abs / 127.0 } else { 1e-12 };

    let mut q = Vec::with_capacity(x.len());
    let mut err2 = 0.0f32;
    for &v in x {
        let qi = (v / scale).round().clamp(-127.0, 127.0) as i8;
        let v_hat = (qi as f32) * scale;
        err2 += (v - v_hat) * (v - v_hat);
        q.push(qi);
    }
    (q, scale, err2.sqrt())
}

pub fn dequantize_per_vector_i8(q: &[i8], scale: f32) -> Vec<f32> {
    q.iter().map(|&qi| (qi as f32) * scale).collect()
}

pub fn dot_i8_i8(a: &[i8], b: &[i8]) -> i32 {
    assert_eq!(a.len(), b.len(), "dot_i8_i8: length mismatch");
    let mut acc: i32 = 0;
    for i in 0..a.len() { acc += (a[i] as i32) * (b[i] as i32); }
    acc
}
