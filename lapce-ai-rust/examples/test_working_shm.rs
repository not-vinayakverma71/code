use lapce_ai_rust::working_shared_memory::WorkingSharedMemory;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    println!("Testing WORKING shared memory implementation\n");
    
    // Create producer
    let mut producer = WorkingSharedMemory::create("test", 4 * 1024 * 1024)?;
    println!("✓ Created 4MB shared memory at /tmp/lapce_shm_test");
    
    // Write different sizes
    let test_sizes = vec![
        (64, "64B"),
        (256, "256B"),
        (1024, "1KB"),
        (4096, "4KB"),
        (16384, "16KB"),
        (65536, "64KB"),
        (262144, "256KB"),
    ];
    
    println!("\n=== Write Test ===");
    for (size, label) in &test_sizes {
        let data = vec![0xAB; *size];
        if producer.write(&data) {
            println!("✅ {} write successful", label);
        } else {
            println!("❌ {} write FAILED", label);
        }
    }
    
    // Connect consumer to SAME memory
    println!("\n=== Read Test (Consumer) ===");
    let mut consumer = WorkingSharedMemory::connect("test")?;
    println!("✓ Consumer connected to same memory");
    
    for (size, label) in &test_sizes {
        if let Some(data) = consumer.read() {
            if data.len() == *size && data[0] == 0xAB {
                println!("✅ {} read successful", label);
            } else {
                println!("❌ {} read wrong: len={}, val={:02x}", label, data.len(), data[0]);
            }
        } else {
            println!("❌ {} read failed", label);
        }
    }
    
    // Performance test
    println!("\n=== Performance Test ===");
    let mut perf_shm = WorkingSharedMemory::create("perf", 64 * 1024 * 1024)?;
    
    let perf_sizes = vec![
        (64, 1_000_000),
        (256, 500_000),
        (1024, 200_000),
        (4096, 50_000),
    ];
    
    for (size, iterations) in perf_sizes {
        let data = vec![0xFF; size];
        let start = Instant::now();
        let mut count = 0;
        
        for _ in 0..iterations {
            if perf_shm.write(&data) {
                count += 1;
            }
        }
        
        let elapsed = start.elapsed();
        let throughput = count as f64 / elapsed.as_secs_f64();
        
        print!("{}B: {:.2}M ops/s", size, throughput / 1_000_000.0);
        if throughput > 1_000_000.0 {
            println!(" ✅ (>1M target met!)");
        } else {
            println!(" ❌");
        }
    }
    
    println!("\n✓ All tests completed!");
    Ok(())
}
