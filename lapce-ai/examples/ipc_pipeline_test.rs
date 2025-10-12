use std::sync::Arc;
use std::time::{Duration, Instant};
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

/// Pipeline test - send many messages before reading responses (measures throughput, not latency)
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    let base_path = format!("/tmp/ipc_pipeline_{}", std::process::id());

    // Start server with warm pool
    let listener = Arc::new(SharedMemoryListener::bind(&base_path).await?);
    
    // Spawn server accept loop with echo handlers
    let _server = {
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
                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            }
        })
    };
    
    println!("Server ready - testing pipelined throughput");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test with 8 clients, pipelined messages
    let clients = 8usize;
    let msgs_per_client = 5000usize;  // Send this many, then read responses
    let msg_size = 1024usize;

    let total_msgs = (clients * msgs_per_client) as u64;
    let mut handles = Vec::with_capacity(clients);

    let start = Instant::now();

    for _ in 0..clients {
        let path = base_path.clone();
        handles.push(tokio::spawn(async move {
            let mut stream = SharedMemoryStream::connect(&path).await.expect("connect");
            
            let buf = vec![0xABu8; msg_size];
            let mut recv = vec![0u8; msg_size];
            
            // PIPELINE: Send all messages first
            let send_start = Instant::now();
            for _ in 0..msgs_per_client {
                stream.write_all(&buf).await.expect("write_all");
            }
            let send_time = send_start.elapsed();
            
            // Then read all responses
            let recv_start = Instant::now();
            for _ in 0..msgs_per_client {
                stream.read_exact(&mut recv).await.expect("read_exact");
            }
            let recv_time = recv_start.elapsed();
            
            (send_time, recv_time)
        }));
    }

    let mut total_send_ms = 0.0;
    let mut total_recv_ms = 0.0;
    
    for h in handles {
        if let Ok((send_time, recv_time)) = h.await {
            total_send_ms += send_time.as_secs_f64() * 1000.0;
            total_recv_ms += recv_time.as_secs_f64() * 1000.0;
        }
    }

    let elapsed = start.elapsed();
    let msgs_per_sec = total_msgs as f64 / elapsed.as_secs_f64();

    println!("\n📊 Pipelined Performance:");
    println!("  Total messages: {}", total_msgs);
    println!("  Total time: {:.2} s", elapsed.as_secs_f64());
    println!("  Throughput: {:.2} Mmsg/s", msgs_per_sec / 1_000_000.0);
    println!("  Avg send time: {:.2} ms per client", total_send_ms / clients as f64);
    println!("  Avg recv time: {:.2} ms per client", total_recv_ms / clients as f64);
    println!("\n  Target: ≥1.0 Mmsg/s");
    
    if msgs_per_sec >= 1_000_000.0 {
        println!("\n✅ TARGET ACHIEVED!");
    } else {
        println!("\n⚠️  Gap: {:.0}× below target", 1_000_000.0 / msgs_per_sec);
    }
    
    Ok(())
}
