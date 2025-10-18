use std::sync::Arc;
use std::time::{Duration, Instant};
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

/// Direct test without filesystem watcher overhead - pre-connected streams
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    let base_path = format!("/tmp/ipc_direct_{}", std::process::id());

    // Start server with warm pool
    let listener = Arc::new(SharedMemoryListener::bind(&base_path).await?);
    
    println!("🚀 Direct Performance Test - Pre-connected streams");
    
    // Pre-connect a single stream pair for testing
    let mut client_stream = SharedMemoryStream::connect(&base_path).await?;
    
    // Server accepts and creates echo task
    let server_handle = {
        let listener = listener.clone();
        tokio::spawn(async move {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    let mut buf = vec![0u8; 1024];
                    let mut count = 0;
                    while count < 10000 {
                        if stream.read_exact(&mut buf).await.is_ok() {
                            if stream.write_all(&buf).await.is_err() {
                                break;
                            }
                            count += 1;
                        }
                    }
                    println!("Server echoed {} messages", count);
                }
                Err(e) => eprintln!("Accept error: {}", e),
            }
        })
    };
    
    // Give server time to accept
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    // Test round-trip performance
    let msg_count = 10000;
    let msg_size = 1024;
    let buf = vec![0xABu8; msg_size];
    let mut recv = vec![0u8; msg_size];
    
    let start = Instant::now();
    
    for i in 0..msg_count {
        client_stream.write_all(&buf).await?;
        client_stream.read_exact(&mut recv).await?;
        
        if i % 1000 == 0 {
            println!("Progress: {}/{}", i, msg_count);
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
    
    // Wait for server to finish
    let _ = tokio::time::timeout(Duration::from_secs(1), server_handle).await;
    
    Ok(())
}
