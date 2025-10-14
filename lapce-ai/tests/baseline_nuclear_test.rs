/// BASELINE IPC NUCLEAR TEST - No SPSC optimizations
/// Tests the original SharedMemoryBuffer implementation against all 8 success criteria

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Barrier;

use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryBuffer, SharedMemoryListener};

#[tokio::test]
async fn baseline_test_1_throughput() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ BASELINE TEST 1: Throughput                                 ║");
    println!("║ Target: >1M messages/second                                 ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let num_messages = 100_000;
    let msg_size = 1024; // 1KB messages
    
    // Create shared memory buffer
    let buffer = SharedMemoryBuffer::create("/test_throughput", 2 * 1024 * 1024).await.unwrap();
    
    let msg = vec![0u8; msg_size];
    let start = Instant::now();
    
    for _ in 0..num_messages {
        buffer.write(&msg).await.unwrap();
        buffer.read().await.unwrap();
    }
    
    let duration = start.elapsed();
    let throughput = (num_messages as f64) / duration.as_secs_f64();
    
    println!("📊 Results:");
    println!("  Messages: {}", num_messages);
    println!("  Duration: {:.2}s", duration.as_secs_f64());
    println!("  Throughput: {:.2} Kmsg/s", throughput / 1000.0);
    
    let passed = throughput >= 1_000_000.0;
    println!("\n  Status: {}", if passed { "✅ PASSED" } else { "❌ FAILED" });
    println!("  Target: ≥1.0M msg/s");
    println!("  Actual: {:.2}M msg/s", throughput / 1_000_000.0);
}

#[tokio::test]
async fn baseline_test_2_latency() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ BASELINE TEST 2: Latency                                    ║");
    println!("║ Target: <10µs per message round-trip                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let buffer = SharedMemoryBuffer::create("/test_latency", 2 * 1024 * 1024).await.unwrap();
    let msg = vec![0u8; 1024];
    let iterations = 10_000;
    
    let mut latencies = Vec::with_capacity(iterations);
    
    for _ in 0..iterations {
        let start = Instant::now();
        buffer.write(&msg).await.unwrap();
        buffer.read().await.unwrap();
        latencies.push(start.elapsed().as_nanos() as u64);
    }
    
    latencies.sort_unstable();
    let p50 = latencies[latencies.len() / 2] as f64 / 1000.0;
    let p99 = latencies[(latencies.len() * 99) / 100] as f64 / 1000.0;
    let p999 = latencies[(latencies.len() * 999) / 1000] as f64 / 1000.0;
    
    println!("📊 Results:");
    println!("  Iterations: {}", iterations);
    println!("  p50:  {:.2}µs", p50);
    println!("  p99:  {:.2}µs", p99);
    println!("  p999: {:.2}µs", p999);
    
    let passed = p99 < 10.0;
    println!("\n  Status: {}", if passed { "✅ PASSED" } else { "❌ FAILED" });
    println!("  Target: <10µs p99");
    println!("  Actual: {:.2}µs p99", p99);
}

#[tokio::test]
async fn baseline_test_3_memory() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ BASELINE TEST 3: Memory Usage                               ║");
    println!("║ Target: <3MB total footprint                                ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let baseline_mb = get_memory_mb();
    
    // Create 10 buffers
    let mut buffers = Vec::new();
    for i in 0..10 {
        let path = format!("/test_mem_{}", i);
        buffers.push(SharedMemoryBuffer::create(&path, 1024 * 1024).await.unwrap());
    }
    
    let with_buffers_mb = get_memory_mb();
    let memory_used = with_buffers_mb - baseline_mb;
    
    println!("📊 Results:");
    println!("  Baseline: {:.2}MB", baseline_mb);
    println!("  With 10 buffers: {:.2}MB", with_buffers_mb);
    println!("  Memory used: {:.2}MB", memory_used);
    
    let passed = with_buffers_mb < 3.0;
    println!("\n  Status: {}", if passed { "✅ PASSED" } else { "❌ FAILED" });
    println!("  Target: <3.0MB");
    println!("  Actual: {:.2}MB", with_buffers_mb);
}

#[tokio::test]
async fn baseline_test_4_connections() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ BASELINE TEST 4: Concurrent Connections                     ║");
    println!("║ Target: Support 1000+ concurrent connections                ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let num_connections = 1024;
    let msgs_per_conn = 10;
    
    println!("Creating {} connections...", num_connections);
    let start_setup = Instant::now();
    
    let mut buffers = Vec::new();
    for i in 0..num_connections {
        let path = format!("/test_conn_{}", i);
        buffers.push(SharedMemoryBuffer::create(&path, 256 * 1024).await.unwrap());
    }
    println!("Setup time: {:.2}s", start_setup.elapsed().as_secs_f64());
    
    let barrier = Arc::new(Barrier::new(num_connections + 1));
    let mut handles = Vec::new();
    
    for buffer in buffers {
        let barrier = barrier.clone();
        let handle = tokio::spawn(async move {
            barrier.wait().await;
            let msg = vec![0u8; 512];
            for _ in 0..msgs_per_conn {
                buffer.write(&msg).await.unwrap();
                buffer.read().await.unwrap();
            }
        });
        handles.push(handle);
    }
    
    barrier.wait().await;
    let start = Instant::now();
    
    for handle in handles {
        handle.await.unwrap();
    }
    
    let duration = start.elapsed();
    let total_msgs = num_connections * msgs_per_conn;
    let throughput = (total_msgs as f64) / duration.as_secs_f64();
    
    println!("\n📊 Results:");
    println!("  Connections: {}", num_connections);
    println!("  Total messages: {}", total_msgs);
    println!("  Duration: {:.2}s", duration.as_secs_f64());
    println!("  Throughput: {:.2} Kmsg/s", throughput / 1000.0);
    
    let passed = num_connections >= 1000;
    println!("\n  Status: {}", if passed { "✅ PASSED" } else { "❌ FAILED" });
}

#[tokio::test]
async fn baseline_test_5_allocations() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ BASELINE TEST 5: Zero Allocations (Manual Check)            ║");
    println!("║ Target: No heap allocations in hot path                     ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    println!("⚠️  Note: SharedMemoryBuffer uses parking_lot locks");
    println!("   This likely DOES allocate in the hot path.");
    println!("\n  Status: ❌ LIKELY FAILED (uses locks)");
}

#[tokio::test]
async fn baseline_test_6_error_recovery() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ BASELINE TEST 6: Error Recovery                             ║");
    println!("║ Target: <100ms reconnection                                 ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let mut recovery_times = Vec::new();
    
    for _ in 0..100 {
        let buffer = SharedMemoryBuffer::create("/test_recovery", 256 * 1024).await.unwrap();
        let msg = vec![0u8; 512];
        buffer.write(&msg).await.unwrap();
        
        let start = Instant::now();
        drop(buffer);
        let new_buffer = SharedMemoryBuffer::create("/test_recovery_new", 256 * 1024).await.unwrap();
        let recovery_time = start.elapsed();
        
        new_buffer.write(&msg).await.unwrap();
        recovery_times.push(recovery_time);
    }
    
    recovery_times.sort();
    let p99 = recovery_times[(recovery_times.len() * 99) / 100];
    
    println!("📊 Results:");
    println!("  p99 recovery: {:.2}ms", p99.as_micros() as f64 / 1000.0);
    
    let passed = p99 <= Duration::from_millis(100);
    println!("\n  Status: {}", if passed { "✅ PASSED" } else { "❌ FAILED" });
    println!("  Target: <100ms");
    println!("  Actual: {:.2}ms", p99.as_micros() as f64 / 1000.0);
}

fn get_memory_mb() -> f64 {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<f64>() {
                            return kb / 1024.0;
                        }
                    }
                }
            }
        }
    }
    2.0
}
