use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

/// Sustained throughput test - measure actual message rate over time
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    let base_path = format!("/tmp/ipc_throughput_{}", std::process::id());
    let listener = Arc::new(SharedMemoryListener::bind(&base_path).await?);
    
    let msg_count = Arc::new(AtomicU64::new(0));
    
    // Spawn server accept loop with echo handlers that count messages
    let server_counter = msg_count.clone();
    let _server = {
        let listener = listener.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((mut stream, _addr)) => {
                        let counter = server_counter.clone();
                        tokio::spawn(async move {
                            let mut buf = vec![0u8; 1024];
                            loop {
                                if stream.read_exact(&mut buf).await.is_err() {
                                    break;
                                }
                                counter.fetch_add(1, Ordering::Relaxed);
                                if stream.write_all(&buf).await.is_err() {
                                    break;
                                }
                            }
                        });
                    }
                    Err(_) => tokio::time::sleep(Duration::from_millis(10)).await,
                }
            }
        })
    };
    
    println!("🚀 Sustained Throughput Test");
    println!("Starting 8 clients sending continuously for 5 seconds...\n");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Start 8 clients sending continuously
    let mut handles = vec![];
    let test_duration = Duration::from_secs(5);
    
    for _ in 0..8 {
        let path = base_path.clone();
        handles.push(tokio::spawn(async move {
            let mut stream = SharedMemoryStream::connect(&path).await.expect("connect");
            let buf = vec![0xABu8; 1024];
            let mut recv = vec![0u8; 1024];
            let mut count = 0u64;
            
            let start = Instant::now();
            while start.elapsed() < test_duration {
                if stream.write_all(&buf).await.is_ok() {
                    if stream.read_exact(&mut recv).await.is_ok() {
                        count += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            count
        }));
    }
    
    // Wait for test duration + grace period
    tokio::time::sleep(test_duration + Duration::from_secs(1)).await;
    
    // Collect results
    let mut total_client_msgs = 0u64;
    for h in handles {
        if let Ok(count) = h.await {
            total_client_msgs += count;
        }
    }
    
    let server_msgs = msg_count.load(Ordering::Relaxed);
    let throughput = total_client_msgs as f64 / test_duration.as_secs_f64();
    
    println!("📊 Results:");
    println!("  Duration: {} seconds", test_duration.as_secs());
    println!("  Total messages: {}", total_client_msgs);
    println!("  Server processed: {}", server_msgs);
    println!("  Throughput: {:.2} Mmsg/s", throughput / 1_000_000.0);
    println!("  Per-client rate: {:.0} msg/s", throughput / 8.0);
    println!("\n  Target: ≥1.0 Mmsg/s");
    
    if throughput >= 1_000_000.0 {
        println!("\n✅ TARGET ACHIEVED!");
    } else {
        println!("\n⚠️  Current: {:.0}× below target", 1_000_000.0 / throughput);
    }
    
    Ok(())
}
