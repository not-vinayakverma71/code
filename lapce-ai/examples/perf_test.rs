use lapce_ai_rust::shared_memory_transport::SharedMemoryTransport;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 SharedMemory Performance Test");
    println!("================================");
    
    // Test 1: Latency measurement
    println!("\n📊 Test 1: Latency Measurement");
    println!("------------------------------");
    
    let transport = SharedMemoryTransport::new(1024 * 1024)?;
    let message = b"Hello, SharedMemory!";
    
    // Warmup
    for _ in 0..100 {
        transport.send(message).await?;
        let _ = transport.recv().await?;
    }
    
    // Measure single message latency
    let mut latencies = Vec::new();
    for _ in 0..1000 {
        let start = Instant::now();
        transport.send(message).await?;
        let _ = transport.recv().await?;
        let elapsed = start.elapsed();
        latencies.push(elapsed);
    }
    
    latencies.sort();
    let p50 = latencies[500];
    let p95 = latencies[950];
    let p99 = latencies[990];
    let avg: Duration = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    
    println!("Latency results (1000 samples):");
    println!("  Average: {:?}", avg);
    println!("  P50: {:?}", p50);
    println!("  P95: {:?}", p95);
    println!("  P99: {:?}", p99);
    
    if avg.as_micros() < 10 {
        println!("✅ Latency target achieved! (<10μs)");
    } else {
        println!("❌ Latency target not met (target: <10μs)");
    }
    
    // Test 2: Throughput measurement
    println!("\n📊 Test 2: Throughput Measurement");
    println!("---------------------------------");
    
    let transport = SharedMemoryTransport::new(1024 * 1024)?;
    let message = vec![0u8; 1024]; // 1KB message
    
    let start = Instant::now();
    let mut count = 0;
    
    // Run for 1 second
    while start.elapsed() < Duration::from_secs(1) {
        transport.send(&message).await?;
        let _ = transport.recv().await?;
        count += 1;
    }
    
    let elapsed = start.elapsed();
    let throughput = count as f64 / elapsed.as_secs_f64();
    
    println!("Throughput results:");
    println!("  Messages: {}", count);
    println!("  Duration: {:?}", elapsed);
    println!("  Rate: {:.0} msg/sec", throughput);
    
    if throughput > 1_000_000.0 {
        println!("✅ Throughput target achieved! (>1M msg/sec)");
    } else {
        println!("❌ Throughput target not met (target: >1M msg/sec)");
    }
    
    // Test 3: Memory usage
    println!("\n📊 Test 3: Memory Usage");
    println!("----------------------");
    
    let pid = std::process::id();
    println!("Process PID: {}", pid);
    
    // Read memory stats from /proc
    if let Ok(status) = std::fs::read_to_string(format!("/proc/{}/status", pid)) {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let rss_kb: u64 = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let rss_mb = rss_kb as f64 / 1024.0;
                
                println!("Resident Set Size: {:.2} MB", rss_mb);
                
                if rss_mb < 3.0 {
                    println!("✅ Memory target achieved! (<3MB)");
                } else {
                    println!("❌ Memory target not met (target: <3MB)");
                }
                break;
            }
        }
    }
    
    // Test 4: Concurrent connections
    println!("\n📊 Test 4: Concurrent Connections");
    println!("--------------------------------");
    
    let mut handles = Vec::new();
    let start = Instant::now();
    
    for i in 0..100 {
        let handle = tokio::spawn(async move {
            let transport = SharedMemoryTransport::new(256 * 1024).unwrap();
            let msg = format!("Connection {}", i).into_bytes();
            for _ in 0..10 {
                transport.send(&msg).await.unwrap();
                let _ = transport.recv().await.unwrap();
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await?;
    }
    
    let elapsed = start.elapsed();
    println!("100 concurrent connections:");
    println!("  Total time: {:?}", elapsed);
    println!("  Avg per connection: {:?}", elapsed / 100);
    
    // Summary
    println!("\n================================");
    println!("📈 Performance Test Complete!");
    println!("================================");
    
    Ok(())
}
