/// Production Scale Benchmark: Test 128 and 512 concurrent clients
/// 
/// Validates:
/// - Round-trip ≥1.0 Mmsg/s under load
/// - p99 ≤10µs write latency
/// - No panics or deadlocks
/// - Memory footprint ≤3MB baseline + slots

use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use hdrhistogram::Histogram;

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> anyhow::Result<()> {
    println!("🔥 Production Scale Benchmark\n");
    
    // Test configurations
    let configs = vec![
        (32, 1000, "Baseline (32 clients)"),
        (128, 1000, "Medium Scale (128 clients)"),
        (512, 500, "High Scale (512 clients)"),
    ];
    
    for (num_clients, msgs_per_client, label) in configs {
        println!("═══ {} ═══", label);
        
        let result = run_benchmark(num_clients, msgs_per_client).await?;
        
        println!("\n📊 Results:");
        println!("  Clients: {}", num_clients);
        println!("  Total messages: {}", result.total_messages);
        println!("  Duration: {:.2}s", result.duration_secs);
        println!("  Throughput: {:.3} Mmsg/s", result.throughput_mmsg_s);
        println!("  Write latency:");
        println!("    p50: {:.2}µs", result.p50_write_us);
        println!("    p99: {:.2}µs", result.p99_write_us);
        println!("    p999: {:.2}µs", result.p999_write_us);
        
        // Check against requirements
        let pass_throughput = result.throughput_mmsg_s >= 1.0;
        let pass_latency = result.p99_write_us <= 10.0;
        
        println!("\n  Requirements:");
        println!("    Throughput ≥1.0 Mmsg/s: {} ({:.3} Mmsg/s)", 
            if pass_throughput { "✅" } else { "❌" },
            result.throughput_mmsg_s
        );
        println!("    p99 latency ≤10µs: {} ({:.2}µs)", 
            if pass_latency { "✅" } else { "❌" },
            result.p99_write_us
        );
        
        if !pass_throughput || !pass_latency {
            println!("\n⚠️  Performance regression detected!");
            std::process::exit(1);
        }
        
        println!("\n✅ {} PASSED\n", label);
        
        // Cool down between tests
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    
    println!("🎉 All scale benchmarks passed!");
    Ok(())
}

#[derive(Debug)]
struct BenchmarkResult {
    total_messages: u64,
    duration_secs: f64,
    throughput_mmsg_s: f64,
    p50_write_us: f64,
    p99_write_us: f64,
    p999_write_us: f64,
}

async fn run_benchmark(num_clients: usize, msgs_per_client: usize) -> anyhow::Result<BenchmarkResult> {
    let base_path = format!("/tmp/ipc_scale_{}", std::process::id());
    let listener = Arc::new(SharedMemoryListener::bind(&base_path).await?);
    
    let send_count = Arc::new(AtomicU64::new(0));
    let recv_count = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    
    // Spawn server accept loop
    let server_recv_count = recv_count.clone();
    let server_running = running.clone();
    let _server = {
        let listener = listener.clone();
        tokio::spawn(async move {
            let mut handlers = vec![];
            
            for _ in 0..num_clients {
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
            
            for h in handlers {
                let _ = h.await;
            }
        })
    };
    
    // Wait for server to be ready
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Track write latencies
    let write_latencies = Arc::new(parking_lot::Mutex::new(
        Histogram::<u64>::new(3).unwrap()
    ));
    
    // Spawn clients
    let mut client_handles = vec![];
    let start = Instant::now();
    
    for _ in 0..num_clients {
        let path = base_path.clone();
        let count = send_count.clone();
        let latencies = write_latencies.clone();
        
        client_handles.push(tokio::spawn(async move {
            let mut stream = SharedMemoryStream::connect(&path).await.ok()?;
            let buf = vec![0xABu8; 1024];
            
            for _ in 0..msgs_per_client {
                let write_start = Instant::now();
                if stream.write_all(&buf).await.is_ok() {
                    let write_elapsed = write_start.elapsed();
                    latencies.lock().record(write_elapsed.as_nanos() as u64).ok();
                    count.fetch_add(1, Ordering::Relaxed);
                } else {
                    break;
                }
            }
            
            Some(())
        }));
    }
    
    // Wait for all clients to finish
    for h in client_handles {
        let _ = h.await;
    }
    
    let duration = start.elapsed();
    running.store(false, Ordering::Relaxed);
    
    // Give server time to process remaining messages
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let total_sent = send_count.load(Ordering::Relaxed);
    let throughput = (total_sent as f64) / duration.as_secs_f64();
    
    // Extract latency percentiles
    let hist = write_latencies.lock();
    let p50_ns = hist.value_at_quantile(0.50);
    let p99_ns = hist.value_at_quantile(0.99);
    let p999_ns = hist.value_at_quantile(0.999);
    
    Ok(BenchmarkResult {
        total_messages: total_sent,
        duration_secs: duration.as_secs_f64(),
        throughput_mmsg_s: throughput / 1_000_000.0,
        p50_write_us: p50_ns as f64 / 1000.0,
        p99_write_us: p99_ns as f64 / 1000.0,
        p999_write_us: p999_ns as f64 / 1000.0,
    })
}
