use std::sync::Arc;
use std::time::Instant;
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

/// Bypass accept() - directly create paired streams for testing
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    let base_path = format!("/tmp/ipc_bypass_{}", std::process::id());

    // Create listener to initialize buffers
    let _listener = SharedMemoryListener::bind(&base_path).await?;
    println!("✓ Buffers initialized");
    
    // Create two connected streams using different base paths
    // This simulates a proper client-server connection
    let client_path = format!("{}_client", base_path);
    let server_path = format!("{}_server", base_path);
    
    // Initialize buffers for both paths
    let _listener2 = SharedMemoryListener::bind(&client_path).await?;
    
    let stream1 = SharedMemoryStream::connect(&base_path).await?;
    let stream2 = SharedMemoryStream::connect(&client_path).await?;
    
    println!("✓ Streams connected");
    
    // Use stream1 as server (echo), stream2 as client
    let server_handle = {
        let mut server = stream1;
        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024];
            let mut count = 0;
            while count < 10000 {
                if server.read_exact(&mut buf).await.is_ok() {
                    if server.write_all(&buf).await.is_err() {
                        break;
                    }
                    count += 1;
                } else {
                    break;
                }
            }
            println!("Server echoed {} messages", count);
        })
    };
    
    // Client sends and receives
    let mut client = stream2;
    let msg_count = 10000;
    let msg_size = 1024;
    let send_buf = vec![0xABu8; msg_size];
    let mut recv_buf = vec![0u8; msg_size];
    
    println!("🚀 Starting round-trip test...");
    let start = Instant::now();
    
    for i in 0..msg_count {
        client.write_all(&send_buf).await?;
        client.read_exact(&mut recv_buf).await?;
        
        if i % 1000 == 0 && i > 0 {
            let elapsed = start.elapsed();
            let rate = i as f64 / elapsed.as_secs_f64();
            println!("Progress: {}/{} ({:.0} msg/s)", i, msg_count, rate);
        }
    }
    
    let elapsed = start.elapsed();
    let msgs_per_sec = msg_count as f64 / elapsed.as_secs_f64();
    
    println!("\n📊 Round-trip Performance:");
    println!("  Messages: {}", msg_count);
    println!("  Time: {:.2} s", elapsed.as_secs_f64());
    println!("  Throughput: {:.2} Mmsg/s", msgs_per_sec / 1_000_000.0);
    println!("  Latency: {:.2} µs/msg", elapsed.as_micros() as f64 / msg_count as f64);
    println!("  Target: ≥1.0 Mmsg/s");
    
    let _ = server_handle.await;
    
    Ok(())
}
