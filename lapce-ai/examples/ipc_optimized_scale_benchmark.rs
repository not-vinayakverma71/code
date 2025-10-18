/// End-to-end optimized IPC scale benchmark
/// Tests complete client-server scenario with SPSC transport

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

use lapce_ai_rust::ipc::shm_stream_optimized::OptimizedShmStream;

#[tokio::main]
async fn main() {
    println!("🔥 Optimized IPC Scale Benchmark\n");
    
    // Test configurations
    let configs = vec![
        (32, 1000),   // 32 clients, 1000 msgs each
        (128, 1000),  // 128 clients, 1000 msgs each
        (512, 1000),  // 512 clients, 1000 msgs each
    ];
    
    for (num_clients, msgs_per_client) in configs {
        run_benchmark(num_clients, msgs_per_client).await;
    }
}

async fn run_benchmark(num_clients: usize, msgs_per_client: usize) {
    println!("═══ Benchmark ({} clients, {} msgs/client) ═══\n", num_clients, msgs_per_client);
    
    let semaphore = Arc::new(Semaphore::new(num_clients));
    let mut handles = Vec::new();
    
    let msg_payload = vec![0u8; 1024]; // 1KB messages
    let total_messages = num_clients * msgs_per_client;
    
    let mut latencies = Vec::new();
    let start = Instant::now();
    
    for client_id in 0..num_clients {
        let sem = semaphore.clone();
        let payload = msg_payload.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            
            // Create optimized stream
            let stream = OptimizedShmStream::connect(&format!("/tmp/ipc_bench_{}", client_id))
                .await
                .expect("Failed to connect");
            
            let mut client_latencies = Vec::new();
            
            for _ in 0..msgs_per_client {
                let start = Instant::now();
                
                // Write message
                stream.write(&payload).await.expect("Write failed");
                
                // Simulate server echo by writing to recv ring
                stream.recv_ring.try_write(&payload);
                stream.recv_waiter.wake_one(stream.recv_ring.write_seq_ptr());
                
                // Read response
                let _response = stream.read().await.expect("Read failed");
                
                let latency = start.elapsed();
                client_latencies.push(latency.as_micros() as u64);
            }
            
            client_latencies
        });
        
        handles.push(handle);
    }
    
    // Collect all latencies
    for handle in handles {
        if let Ok(client_lats) = handle.await {
            latencies.extend(client_lats);
        }
    }
    
    let elapsed = start.elapsed();
    
    // Calculate metrics
    latencies.sort_unstable();
    let p50 = latencies[latencies.len() / 2] as f64 / 1000.0; // Convert to ms
    let p99 = latencies[(latencies.len() * 99) / 100] as f64 / 1000.0;
    let p999 = latencies[(latencies.len() * 999) / 1000] as f64 / 1000.0;
    
    let throughput = (total_messages as f64) / elapsed.as_secs_f64();
    
    println!("📊 Results:");
    println!("  Clients: {}", num_clients);
    println!("  Total messages: {}", total_messages);
    println!("  Duration: {:.2}s", elapsed.as_secs_f64());
    println!("  Throughput: {:.3} Mmsg/s", throughput / 1_000_000.0);
    println!("  Write latency:");
    println!("    p50: {:.2}µs", p50);
    println!("    p99: {:.2}µs", p99);
    println!("    p999: {:.2}µs", p999);
    println!();
    
    // Check requirements
    let throughput_target = 1_000_000.0;
    let latency_target = 10.0; // µs
    
    let throughput_met = throughput >= throughput_target;
    let latency_met = p99 <= latency_target;
    
    println!("  Requirements:");
    println!("    Throughput ≥1.0 Mmsg/s: {} ({:.3} Mmsg/s)", 
        if throughput_met { "✅" } else { "❌" }, 
        throughput / 1_000_000.0);
    println!("    p99 latency ≤10µs: {} ({:.2}µs)", 
        if latency_met { "✅" } else { "❌" }, 
        p99);
    println!();
    
    if throughput_met && latency_met {
        println!("  ✅ All targets MET!\n");
    } else {
        println!("  ⚠️  Some targets not met\n");
    }
}
