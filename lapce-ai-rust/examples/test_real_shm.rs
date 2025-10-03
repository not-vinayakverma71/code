use lapce_ai_rust::real_shared_memory::RealSharedMemory;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    println!("Testing Real Shared Memory\n");
    
    // Create producer
    let mut producer = RealSharedMemory::create("test", 64 * 1024 * 1024)?;
    println!("Created 64MB shared memory at /dev/shm/lapce_test");
    
    // Test different message sizes
    let test_sizes = vec![
        (64, "64B"),
        (256, "256B"),
        (1024, "1KB"),
        (4096, "4KB"),
        (16384, "16KB"),
        (65536, "64KB"),
        (262144, "256KB"),
        (1048576, "1MB"),
    ];
    
    println!("\nWrite test:");
    for (size, label) in &test_sizes {
        let data = vec![0xAB; *size];
        if producer.write(&data) {
            println!("✓ {} write successful", label);
        } else {
            println!("✗ {} write failed", label);
        }
    }
    
    // Stats
    let (used, free) = producer.stats();
    println!("\nBuffer stats: {}KB used, {}KB free", used/1024, free/1024);
    
    // Read test (connect as consumer)
    println!("\nConnecting consumer...");
    let mut consumer = RealSharedMemory::connect("test")?;
    
    println!("Read test:");
    for (size, label) in &test_sizes {
        if let Some(data) = consumer.read() {
            if data.len() == *size && data[0] == 0xAB {
                println!("✓ {} read successful", label);
            } else {
                println!("✗ {} read incorrect (len={}, val={})", label, data.len(), data[0]);
            }
        } else {
            println!("✗ {} read failed", label);
        }
    }
    
    // Performance test
    println!("\n=== Performance Test ===");
    let mut shm = RealSharedMemory::create("bench", 128 * 1024 * 1024)?;
    
    let perf_sizes = vec![
        (64, 1_000_000),
        (256, 500_000),
        (1024, 200_000),
        (4096, 50_000),
        (16384, 10_000),
    ];
    
    for (size, iterations) in perf_sizes {
        let data = vec![0xFF; size];
        let start = Instant::now();
        let mut count = 0;
        
        for _ in 0..iterations {
            if shm.write(&data) {
                count += 1;
            }
        }
        
        let elapsed = start.elapsed();
        let throughput = count as f64 / elapsed.as_secs_f64();
        let throughput_mb = (size * count) as f64 / elapsed.as_secs_f64() / (1024.0 * 1024.0);
        
        print!("{}B: {:.2}M ops/s, {:.2} MB/s", size, throughput / 1_000_000.0, throughput_mb);
        if throughput > 1_000_000.0 {
            println!(" ✅");
        } else {
            println!(" ❌");
        }
    }
    
    println!("\nAll tests completed!");
    Ok(())
}
