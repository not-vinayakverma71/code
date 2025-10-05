/// Test optimized performance with better buffer management
use lapce_ai_rust::shared_memory_complete::SharedMemoryBuffer as OptimizedSharedMemory;
use std::time::Instant;

fn main() {
    println!("OPTIMIZED V2 PERFORMANCE TEST");
    println!("{}", "=".repeat(60));
    
    // Use larger buffer for better throughput
    let mut server = OptimizedSharedMemory::new();
    let mut server = OptimizedIpcServer::new();
    let channel_id = server.create_channel(26); // 64MB buffer (2^26)
    
    // Pre-allocate data
    let data = vec![0u8; 10];
    let msg_count = 100_000_000; // 100M messages for accurate measurement
    
    println!("Warming up with 100K messages...");
    for _ in 0..100_000 {
        server.send(channel_id, &data);
    }
    
    println!("\nTesting {} messages...", msg_count);
    let start = Instant::now();
    let mut sent = 0;
    
    // Send in batches for better cache locality
    let batch_size = 1000;
    for _ in 0..(msg_count / batch_size) {
        for _ in 0..batch_size {
            if server.send(channel_id, &data) {
                sent += 1;
            } else {
                break;
            }
        }
    }
    
    let elapsed = start.elapsed();
    let throughput = sent as f64 / elapsed.as_secs_f64();
    
    println!("\n📊 RESULTS:");
    println!("   Messages sent: {}/{}", sent, msg_count);
    println!("   Time: {:?}", elapsed);
    println!("   Throughput: {:.0} msg/sec", throughput);
    println!("   vs Node.js (5.5M): {:.1}x", throughput / 5_527_213.0);
    
    println!("\n✅ VERDICT:");
    let is_10x = throughput >= 55_272_130.0;
    println!("   10x Node.js (55.3M): {}", if is_10x { "✅ PASS" } else { "❌ FAIL" });
    
    if !is_10x {
        let gap = 55_272_130.0 - throughput;
        println!("   Gap to 10x: {:.0} msg/sec ({:.1}%)", gap, (gap / 55_272_130.0) * 100.0);
    }
}
