use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Proper throughput test - decouple send and receive to measure actual message rate
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    let base_path = format!("/tmp/ipc_throughput_proper_{}", std::process::id());
    let listener = Arc::new(SharedMemoryListener::bind(&base_path).await?);
    
    let send_count = Arc::new(AtomicU64::new(0));
    let recv_count = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    
    // Server: accept connections and spawn reader tasks
    let server_recv_count = recv_count.clone();
    let server_running = running.clone();
    let _server = {
        let listener = listener.clone();
        tokio::spawn(async move {
            let mut handlers = vec![];
            
            // Accept up to 8 connections
            for _ in 0..8 {
                match listener.accept().await {
                    Ok((mut stream, _)) => {
                        let count = server_recv_count.clone();
                        let running = server_running.clone();
                        handlers.push(tokio::spawn(async move {
                            let mut buf = vec![0u8; 1024];
                            while running.load(Ordering::Relaxed) {
                                if stream.read_exact(&mut buf).await.is_ok() {
                                    count.fetch_add(1, Ordering::Relaxed);
                                } else {
                                    break;
                                }
                            }
                        }));
                    }
                    Err(_) => break,
                }
            }
            
            // Wait for handlers
            for h in handlers {
                let _ = h.await;
            }
        })
    };
    
    tokio::time::sleep(Duration::from_millis(200)).await;
    println!("🚀 Proper Throughput Test - Decoupled Send/Receive");
    println!("Starting 8 clients sending continuously for 5 seconds...\n");
    
    // Clients: connect and spawn writer tasks
    let mut client_handles = vec![];
    let test_duration = Duration::from_secs(5);
    
    for i in 0..8 {
        let path = base_path.clone();
        let count = send_count.clone();
        let running = running.clone();
        
        client_handles.push(tokio::spawn(async move {
            match SharedMemoryStream::connect(&path).await {
                Ok(mut stream) => {
                    let buf = vec![0xABu8; 1024];
                    let start = Instant::now();
                    let mut sent = 0u64;
                    
                    while start.elapsed() < test_duration && running.load(Ordering::Relaxed) {
                        if stream.write_all(&buf).await.is_ok() {
                            sent += 1;
                            count.fetch_add(1, Ordering::Relaxed);
                        } else {
                            break;
                        }
                    }
                    
                    if sent > 0 && i == 0 {
                        println!("[Client 0] Sent {} messages", sent);
                    }
                    sent
                }
                Err(e) => {
                    eprintln!("[Client {}] Connect failed: {}", i, e);
                    0
                }
            }
        }));
    }
    
    // Wait for test duration
    tokio::time::sleep(test_duration).await;
    running.store(false, Ordering::Relaxed);
    
    // Give a moment for final messages to process
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Collect results
    let mut total_client_sent = 0u64;
    for h in client_handles {
        if let Ok(sent) = h.await {
            total_client_sent += sent;
        }
    }
    
    let final_send_count = send_count.load(Ordering::Relaxed);
    let final_recv_count = recv_count.load(Ordering::Relaxed);
    
    let send_throughput = final_send_count as f64 / test_duration.as_secs_f64();
    let recv_throughput = final_recv_count as f64 / test_duration.as_secs_f64();
    
    println!("\n📊 Results:");
    println!("  Duration: {} seconds", test_duration.as_secs());
    println!("  Messages sent: {}", final_send_count);
    println!("  Messages received: {}", final_recv_count);
    println!("  Send throughput: {:.2} Mmsg/s", send_throughput / 1_000_000.0);
    println!("  Recv throughput: {:.2} Mmsg/s", recv_throughput / 1_000_000.0);
    println!("  Per-client avg: {:.0} msg/s", send_throughput / 8.0);
    println!("\n  Target: ≥1.0 Mmsg/s");
    
    if send_throughput >= 1_000_000.0 {
        println!("\n✅ TARGET ACHIEVED!");
    } else {
        let gap = 1_000_000.0 / send_throughput;
        println!("\n⚠️  Current: {:.0}× below target", gap);
        println!("  (This tests sustained write throughput, not latency)");
    }
    
    Ok(())
}
