use std::time::Instant;
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    let base_path = format!("/tmp/ipc_perf_{}", std::process::id());

    // Start server with warm pool
    let listener = SharedMemoryListener::bind(&base_path).await?;
    
    // Single client, single connection test
    println!("🔬 Performance Test: Single client, no accept overhead");
    
    // Pre-connect client
    let mut stream = SharedMemoryStream::connect(&base_path).await?;
    println!("✓ Client connected");
    
    // Test raw write/read performance without accept overhead
    let msg_sizes = vec![64, 256, 1024, 4096, 16384];
    let iterations = 10_000;
    
    for msg_size in msg_sizes {
        let buf = vec![0xABu8; msg_size];
        
        // Write-only test (no round-trip)
        let start = Instant::now();
        for _ in 0..iterations {
            stream.write_all(&buf).await?;
        }
        let elapsed = start.elapsed();
        let msgs_per_sec = iterations as f64 / elapsed.as_secs_f64();
        
        println!("\n📊 Write-only ({} bytes):", msg_size);
        println!("  Throughput: {:.2} Mmsg/s", msgs_per_sec / 1_000_000.0);
        println!("  Latency: {:.2} µs/msg", elapsed.as_micros() as f64 / iterations as f64);
        println!("  Bandwidth: {:.2} MB/s", (msg_size as f64 * msgs_per_sec) / 1_000_000.0);
    }
    
    println!("\n✅ Test complete");
    Ok(())
}
