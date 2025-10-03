use std::time::Instant;

fn main() {
    // Test compression
    let vec = vec![0.5f32; 1536];
    
    let compress_start = Instant::now();
    for _ in 0..100 {
        let data: Vec<u8> = vec.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let _ = zstd::encode_all(&data[..], 3);
    }
    let compress_time = compress_start.elapsed() / 100;
    
    // Test decompression
    let compressed = zstd::encode_all(
        vec.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>().as_slice(), 
        3
    ).unwrap();
    
    let decompress_start = Instant::now();
    for _ in 0..100 {
        let _ = zstd::decode_all(&compressed[..]);
    }
    let decompress_time = decompress_start.elapsed() / 100;
    
    println!("\n=== CURRENT PERFORMANCE BENCHMARK ===");
    println!("Compression (1536-dim): {:?}", compress_time);
    println!("Decompression (1536-dim): {:?}", decompress_time);
    println!("Compression ratio: {:.1}%", compressed.len() as f32 / (1536 * 4) as f32 * 100.0);
    
    // Test query simulation
    let query_start = Instant::now();
    let mut sum = 0.0f32;
    for _ in 0..1000 {
        for i in 0..1536 {
            sum += vec[i] * vec[i];
        }
    }
    let query_time = query_start.elapsed() / 1000;
    println!("\nDot product (1536-dim): {:?}", query_time);
    println!("Throughput: {:.0} ops/sec", 1.0 / query_time.as_secs_f64());
    
    // Prevent optimization
    println!("\n(debug: {})", sum);
}
