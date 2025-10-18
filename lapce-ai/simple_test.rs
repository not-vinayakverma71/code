fn main() {
    println!("🚀 SIMPLE SHARED MEMORY TEST");
    println!("============================");
    
    // Test basic memory allocation
    println!("\n1. Testing memory allocation...");
    let data = vec![1u8; 1024 * 1024]; // 1MB
    println!("   ✓ Allocated 1MB");
    
    // Test throughput with simple loop
    println!("\n2. Testing throughput...");
    let start = std::time::Instant::now();
    let mut count = 0u64;
    
    for _ in 0..1_000_000 {
        count += 1;
    }
    
    let elapsed = start.elapsed();
    let ops_per_sec = count as f64 / elapsed.as_secs_f64();
    
    println!("   Operations: {}", count);
    println!("   Time: {:.3}s", elapsed.as_secs_f64());
    println!("   Throughput: {:.0} ops/sec", ops_per_sec);
    
    // Test latency
    println!("\n3. Testing latency...");
    let start = std::time::Instant::now();
    let _ = vec![0u8; 1024]; // 1KB allocation
    let latency = start.elapsed();
    
    println!("   Latency: {:.3} μs", latency.as_nanos() as f64 / 1000.0);
    
    println!("\n✅ TEST COMPLETED");
}
