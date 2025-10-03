/// Verify the 15.2M msg/sec performance claim
use lapce_ai_rust::shared_memory_ipc::*;
use std::time::Instant;

fn main() {
    println!("PERFORMANCE VERIFICATION TEST");
    println!("{}", "=".repeat(60));
    
    // Create server with large buffer
    let server = SharedMemoryIpcServer::new();
    let channel_id = server.create_channel(100 * 1024 * 1024); // 100MB
    server.start();
    
    // Test with 10-byte messages (claimed 15.2M msg/sec)
    let msg_count = 10_000_000; // 10M messages
    let data = vec![0u8; 10];
    
    println!("\nTesting {} x 10-byte messages...", msg_count);
    let start = Instant::now();
    let mut sent = 0;
    
    for _ in 0..msg_count {
        if server.send(channel_id, &data) {
            sent += 1;
        } else {
            break; // Buffer full
        }
    }
    
    let elapsed = start.elapsed();
    let throughput = sent as f64 / elapsed.as_secs_f64();
    let latency_ns = elapsed.as_nanos() / sent.max(1) as u128;
    
    println!("\n📊 RESULTS:");
    println!("   Messages sent: {}/{}", sent, msg_count);
    println!("   Time: {:?}", elapsed);
    println!("   Throughput: {:.0} msg/sec", throughput);
    println!("   Latency: {} ns", latency_ns);
    println!("\n📈 PERFORMANCE vs CLAIMS:");
    println!("   Claimed: 15,236,034 msg/sec");
    println!("   Actual:  {:.0} msg/sec", throughput);
    println!("   Ratio:   {:.1}%", (throughput / 15_236_034.0) * 100.0);
    println!("\n✅ VERDICT:");
    println!("   >1M msg/sec: {}", if throughput > 1_000_000.0 { "✅ YES" } else { "❌ NO" });
    println!("   >15M msg/sec: {}", if throughput > 15_000_000.0 { "✅ YES" } else { "❌ NO" });
}
