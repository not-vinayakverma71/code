/// Simple IPC Performance Test
use lapce_ai_rust::shared_memory_complete::{SharedMemoryBuffer};
use std::time::Instant;

fn main() {
    println!("🚀 IPC PERFORMANCE TEST");
    println!("======================\n");
    
    // Test SharedMemory Performance
    test_shared_memory();
    
    println!("\n✅ ALL TESTS COMPLETED!");
}

fn test_shared_memory() {
    println!("SharedMemory Performance Test:");
    
    let mut buffer = SharedMemoryBuffer::create("perf_test", 4 * 1024 * 1024).unwrap();
    
    let data = vec![0u8; 1024]; // 1KB messages
    let iterations = 1_000_000;
    
    let start = Instant::now();
    for _ in 0..iterations {
        buffer.write(&data).unwrap();
        let _ = buffer.read().unwrap();
    }
    let duration = start.elapsed();
    
    let msgs_per_sec = iterations as f64 / duration.as_secs_f64();
    let latency_us = duration.as_micros() as f64 / (iterations * 2) as f64;
    
    println!("  • Throughput: {:.2}M msg/sec", msgs_per_sec / 1_000_000.0);
    println!("  • Latency: {:.3}μs per operation", latency_us);
    println!("  • Total time: {:.2}s for {} operations", duration.as_secs_f64(), iterations * 2);
    
    if latency_us < 10.0 {
        println!("  ✅ PASS: Latency < 10μs");
    } else {
        println!("  ❌ FAIL: Latency {} > 10μs", latency_us);
    }
    
    if msgs_per_sec > 1_000_000.0 {
        println!("  ✅ PASS: Throughput > 1M msg/sec");
    } else {
        println!("  ❌ FAIL: Throughput {:.2}M < 1M msg/sec", msgs_per_sec / 1_000_000.0);
    }
}
