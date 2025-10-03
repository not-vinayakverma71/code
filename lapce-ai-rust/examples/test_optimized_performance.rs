/// Test optimized lock-free performance - targeting 55M msg/sec
use lapce_ai_rust::shared_memory_complete::*;
use std::time::Instant;

fn main() {
    println!("OPTIMIZED LOCK-FREE PERFORMANCE TEST");
    println!("{}", "=".repeat(60));
    println!("Target: 55,272,130 msg/sec (10x Node.js)");
    
    let mut server = OptimizedIpcServer::new();
    let channel_id = server.create_channel(24); // 16MB buffer (2^24)
    
    // Test with 10-byte messages
    let msg_count = 10_000_000;
    let data = vec![0u8; 10];
    
    println!("\nWarming up...");
    for _ in 0..10000 {
        server.send(channel_id, &data);
    }
    
    println!("Testing {} messages...", msg_count);
    let start = Instant::now();
    let mut sent = 0;
    
    for _ in 0..msg_count {
        if server.send(channel_id, &data) {
            sent += 1;
        }
    }
    
    let elapsed = start.elapsed();
    let throughput = sent as f64 / elapsed.as_secs_f64();
    let latency_ns = elapsed.as_nanos() / sent as u128;
    
    println!("\n📊 RESULTS:");
    println!("   Messages sent: {}", sent);
    println!("   Time: {:?}", elapsed);
    println!("   Throughput: {:.0} msg/sec", throughput);
    println!("   Latency: {} ns", latency_ns);
    
    println!("\n📈 PERFORMANCE vs TARGETS:");
    println!("   Node.js: 5,527,213 msg/sec");
    println!("   Required (10x): 55,272,130 msg/sec");
    println!("   Achieved: {:.0} msg/sec", throughput);
    println!("   vs Node.js: {:.1}x", throughput / 5_527_213.0);
    
    println!("\n✅ VERDICT:");
    println!("   >1M msg/sec: {}", if throughput > 1_000_000.0 { "✅ PASS" } else { "❌ FAIL" });
    println!("   >55M msg/sec: {}", if throughput > 55_000_000.0 { "✅ PASS" } else { "❌ FAIL" });
    println!("   10x Node.js: {}", if throughput > 55_272_130.0 { "✅ PASS" } else { "❌ FAIL" });
}
