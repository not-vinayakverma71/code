/// Test shared memory IPC performance
use lapce_ai_rust::shared_memory_ipc::*;
use std::time::Instant;

fn main() {
    println!("Shared Memory IPC Performance Test");
    println!("{}", "=".repeat(60));
    
    let server = SharedMemoryIpcServer::new();
    let channel_id = server.create_channel(100 * 1024 * 1024); // 100MB buffer for larger messages
    
    server.start();
    
    // Test different message sizes
    for msg_size in [10, 100, 1000] {
        let msg_count = 1_000_000;
        let data = vec![0u8; msg_size];
        
        let start = Instant::now();
        let mut sent = 0;
        
        for _ in 0..msg_count {
            if server.send(channel_id, &data) {
                sent += 1;
            }
        }
        
        let elapsed = start.elapsed();
        let throughput = if sent > 0 { sent as f64 / elapsed.as_secs_f64() } else { 0.0 };
        let latency_ns = if sent > 0 { elapsed.as_nanos() / sent as u128 } else { 0 };
        
        println!("\n📊 {} byte messages:", msg_size);
        println!("   Sent: {}/{}", sent, msg_count);
        println!("   Time: {:?}", elapsed);
        println!("   Throughput: {:.0} msg/sec", throughput);
        println!("   Latency: {} ns", latency_ns);
        println!("   >1M msg/sec: {}", if throughput > 1_000_000.0 { "✅" } else { "❌" });
    }
    
    println!("\n{}", "=".repeat(60));
    println!("Total messages processed: {}", server.message_count());
}
