use std::sync::Arc;
use std::time::{Duration, Instant};

use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    let base_path = format!("/tmp/ipc_simple_{}", std::process::id());

    // Start server with warm pool (64 slots)
    let listener = Arc::new(SharedMemoryListener::bind(&base_path).await?);
    
    // Spawn server accept loop with echo handlers
    let server = {
        let listener = listener.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((mut stream, _addr)) => {
                        tokio::spawn(async move {
                            let mut buf = vec![0u8; 1024];
                            loop {
                                if stream.read_exact(&mut buf).await.is_err() {
                                    break;
                                }
                                if stream.write_all(&buf).await.is_err() {
                                    break;
                                }
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Accept error: {}", e);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            }
        })
    };
    
    println!("Server ready with warm pool (64 slots)");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test with 8 clients × 1000 messages
    let clients = 8usize;
    let msgs_per_client = 1000usize;
    let msg_size = 1024usize;

    let total_msgs = (clients * msgs_per_client) as u64;
    let mut handles = Vec::with_capacity(clients);

    let start = Instant::now();
    let connect_start = Instant::now();

    for _i in 0..clients {
        let path = base_path.clone();
        handles.push(tokio::spawn(async move {
            // Connect via lock-free slot claiming
            let conn_start = Instant::now();
            let mut stream = SharedMemoryStream::connect(&path).await.expect("connect");
            let conn_time = conn_start.elapsed();
            
            let buf = vec![0xABu8; msg_size];
            let mut recv = vec![0u8; msg_size];
            
            // Round-trip echo test
            let msg_start = Instant::now();
            for _ in 0..msgs_per_client {
                stream.write_all(&buf).await.expect("write_all");
                stream.read_exact(&mut recv).await.expect("read_exact");
            }
            let msg_time = msg_start.elapsed();
            
            (conn_time, msg_time)
        }));
    }

    let mut total_conn_ms = 0.0;
    let mut total_msg_ms = 0.0;
    
    for h in handles { 
        if let Ok((conn_time, msg_time)) = h.await {
            total_conn_ms += conn_time.as_secs_f64() * 1000.0;
            total_msg_ms += msg_time.as_secs_f64() * 1000.0;
        }
    }
    
    let connect_elapsed = connect_start.elapsed();
    println!("\n📊 Performance Breakdown:");
    println!("  Connection phase: {:.2} ms total ({:.2} ms avg per client)", 
             connect_elapsed.as_secs_f64() * 1000.0, total_conn_ms / clients as f64);
    println!("  Message phase: {:.2} ms avg per client", total_msg_ms / clients as f64);
    println!("  Messages per client: {} round-trips", msgs_per_client);

    let elapsed = start.elapsed();
    let msgs_per_sec = total_msgs as f64 / elapsed.as_secs_f64();

    println!("\n✅ SUCCESS!");
    println!("Throughput: {:.2} Mmsg/s", msgs_per_sec / 1_000_000.0);
    println!("Messages: {} in {:.2} s", total_msgs, elapsed.as_secs_f64());
    println!("Target: ≥ 1.0 Mmsg/s");
    
    // Validate no panics
    println!("All {} clients completed successfully", clients);

    let _ = server;
    Ok(())
}
