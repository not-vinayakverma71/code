use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    let base_path = format!("/ipc_smoke_{}", std::process::id());

    // Start server
    let listener = Arc::new(SharedMemoryListener::bind(&base_path).await?);

    let server = {
        let listener = listener.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((mut stream, _addr)) => {
                        tokio::spawn(async move {
                            let mut buf = vec![0u8; 1024];
                            loop {
                                if stream.read_exact(&mut buf[..]).await.is_err() {
                                    break;
                                }
                                if stream.write_all(&buf[..]).await.is_err() {
                                    break;
                                }
                            }
                        });
                    }
                    Err(_) => {
                        // Short delay to avoid tight loop on transient errors
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                }
            }
        })
    };

    // Client workload
    let clients = 32usize;
    let msgs_per_client = 200usize;
    let msg_size = 1024usize; // 1KiB

    let total_msgs = (clients * msgs_per_client) as u64;
    let mut handles = Vec::with_capacity(clients);

    let start = Instant::now();

    let latencies = Arc::new(RwLock::new(Vec::<u128>::with_capacity(total_msgs as usize)));

    for _ in 0..clients {
        let path = base_path.clone();
        let latencies = latencies.clone();
        handles.push(tokio::spawn(async move {
            let mut stream = SharedMemoryStream::connect(&path).await.expect("connect");
            let buf = vec![0xABu8; msg_size];
            let mut recv = vec![0u8; msg_size];
            for _ in 0..msgs_per_client {
                let t0 = Instant::now();
                stream.write_all(&buf).await.expect("write_all");
                stream.read_exact(&mut recv).await.expect("read_exact");
                let dt = t0.elapsed().as_micros();
                latencies.write().await.push(dt as u128);
            }
        }));
    }

    for h in handles { let _ = h.await; }

    let elapsed = start.elapsed();
    let msgs_per_sec = total_msgs as f64 / elapsed.as_secs_f64();

    // p99 latency (microseconds)
    let mut lats = latencies.write().await;
    lats.sort_unstable();
    let p99_idx = ((lats.len() as f64) * 0.99).floor() as usize; 
    let p99 = lats.get(p99_idx.min(lats.len().saturating_sub(1))).cloned().unwrap_or(0);

    println!("Throughput: {:.2} Mmsg/s", msgs_per_sec / 1_000_000.0);
    println!("p99 latency: {} µs", p99);
    println!("Messages: {} in {:.2} s", total_msgs, elapsed.as_secs_f64());

    // Keep server running briefly to drain any pending tasks
    let _ = server;
    Ok(())
}
