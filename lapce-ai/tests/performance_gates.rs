/// Performance Gates for IPC Implementation
/// Enforces: >=1M msg/s, p50<10μs, p99<50μs, memory<3MB at 100 conns

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

#[derive(Debug)]
struct PerformanceMetrics {
    throughput: f64,
    p50_us: u128,
    p99_us: u128,
    memory_mb: f64,
}

/// Main performance gate test
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn performance_gate_test() {
    println!("\n🎯 PERFORMANCE GATES TEST");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let metrics = run_performance_test().await;
    
    // Verify performance gates
    println!("\n📊 Performance Results:");
    println!("  Throughput: {:.2}M msg/s (required: >=1M)", metrics.throughput / 1_000_000.0);
    println!("  p50 Latency: {}μs (required: <10μs)", metrics.p50_us);
    println!("  p99 Latency: {}μs (required: <50μs)", metrics.p99_us);
    println!("  Memory Usage: {:.2}MB (required: <3MB)", metrics.memory_mb);
    
    // Assert gates
    assert!(metrics.throughput >= 1_000_000.0, 
            "Throughput {:.2}M msg/s failed gate (>=1M msg/s)", metrics.throughput / 1_000_000.0);
    assert!(metrics.p50_us < 10, 
            "p50 latency {}μs failed gate (<10μs)", metrics.p50_us);
    assert!(metrics.p99_us < 50, 
            "p99 latency {}μs failed gate (<50μs)", metrics.p99_us);
    assert!(metrics.memory_mb < 3.0, 
            "Memory usage {:.2}MB failed gate (<3MB)", metrics.memory_mb);
    
    println!("\n✅ ALL PERFORMANCE GATES PASSED!");
}

async fn run_performance_test() -> PerformanceMetrics {
    // Setup
    let listener = Arc::new(SharedMemoryListener::bind("/perf_gate").unwrap());
    let message_count = 1_000_000;
    let connection_count = 100;
    let mut latencies = Vec::with_capacity(message_count);
    
    // Start echo server
    let server_listener = listener.clone();
    let server = tokio::spawn(async move {
        let mut connections = vec![];
        
        // Accept connections
        for _ in 0..connection_count {
            if let Ok((stream, _)) = server_listener.accept().await {
                connections.push(stream);
            }
        }
        
        // Echo messages
        for mut stream in connections {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 256];
                loop {
                    if let Ok(n) = stream.read(&mut buf).await {
                        if n == 0 { break; }
                        stream.write(&buf[..n]).await.ok();
                    } else {
                        break;
                    }
                }
            });
        }
    });
    
    // Create client connections
    let mut clients = vec![];
    for _ in 0..connection_count {
        let client = SharedMemoryStream::connect("/perf_gate").await.unwrap();
        clients.push(client);
    }
    
    // Warmup
    for client in &mut clients {
        for _ in 0..100 {
            client.write(b"warmup").await.ok();
            let mut buf = vec![0u8; 32];
            client.read(&mut buf).await.ok();
        }
    }
    
    // Measure memory before
    let mem_before = get_memory_usage_mb();
    
    // Performance measurement
    let start = Instant::now();
    let messages_per_client = message_count / connection_count;
    
    let mut handles = vec![];
    for mut client in clients {
        let handle = tokio::spawn(async move {
            let mut client_latencies = Vec::with_capacity(messages_per_client);
            let test_message = b"performance_test_message_12345";
            let mut response_buf = vec![0u8; 64];
            
            for _ in 0..messages_per_client {
                let msg_start = Instant::now();
                
                client.write(test_message).await.unwrap();
                client.read(&mut response_buf).await.unwrap();
                
                let latency = msg_start.elapsed();
                client_latencies.push(latency);
            }
            
            client_latencies
        });
        handles.push(handle);
    }
    
    // Collect all latencies
    for handle in handles {
        if let Ok(client_lats) = handle.await {
            latencies.extend(client_lats);
        }
    }
    
    let elapsed = start.elapsed();
    let mem_after = get_memory_usage_mb();
    
    // Calculate metrics
    latencies.sort();
    let p50_us = latencies[latencies.len() / 2].as_micros();
    let p99_us = latencies[latencies.len() * 99 / 100].as_micros();
    let throughput = message_count as f64 / elapsed.as_secs_f64();
    let memory_mb = mem_after - mem_before;
    
    drop(server);
    
    PerformanceMetrics {
        throughput,
        p50_us,
        p99_us,
        memory_mb: memory_mb.max(0.1), // Ensure positive value
    }
}

fn get_memory_usage_mb() -> f64 {
    // Simple memory estimation
    // In production, use proper memory profiling
    use std::fs;
    
    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
    }
    
    // Fallback estimate
    1.0
}

/// CI-friendly performance benchmark
#[test]
fn ci_performance_benchmark() {
    // This test can be run in CI to track performance over time
    println!("\n📈 CI Performance Benchmark");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let result = rt.block_on(async {
        let listener = SharedMemoryListener::bind("/ci_bench").unwrap();
        
        // Simple throughput test
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 1024];
            let mut count = 0;
            
            while count < 100_000 {
                if stream.read(&mut buf).await.is_ok() {
                    stream.write(&buf[..32]).await.ok();
                    count += 1;
                }
            }
            count
        });
        
        let mut client = SharedMemoryStream::connect("/ci_bench").await.unwrap();
        let msg = vec![0xAA; 32];
        let mut response = vec![0u8; 32];
        
        let start = Instant::now();
        for _ in 0..100_000 {
            client.write(&msg).await.unwrap();
            client.read(&mut response).await.unwrap();
        }
        let elapsed = start.elapsed();
        
        let count = server.await.unwrap();
        let throughput = count as f64 / elapsed.as_secs_f64();
        
        (throughput, elapsed)
    });
    
    println!("  Throughput: {:.2}M msg/s", result.0 / 1_000_000.0);
    println!("  Duration: {:.2}s", result.1.as_secs_f64());
    
    // CI assertion
    assert!(result.0 > 1_000_000.0, "CI throughput gate failed");
}

/// Memory stress test with 100 connections
#[tokio::test]
async fn memory_stress_100_connections() {
    println!("\n💾 Memory Stress Test (100 connections)");
    
    let mem_start = get_memory_usage_mb();
    
    // Create 100 connections
    let listener = SharedMemoryListener::bind("/mem_stress").unwrap();
    
    let server = tokio::spawn(async move {
        let mut connections = vec![];
        for _ in 0..100 {
            if let Ok((stream, _)) = listener.accept().await {
                connections.push(stream);
            }
        }
        connections
    });
    
    let mut clients = vec![];
    for _ in 0..100 {
        let client = SharedMemoryStream::connect("/mem_stress").await.unwrap();
        clients.push(client);
    }
    
    // Exchange some data
    for client in &mut clients {
        client.write(b"test").await.ok();
    }
    
    let mem_peak = get_memory_usage_mb();
    let mem_used = mem_peak - mem_start;
    
    println!("  Memory used for 100 connections: {:.2}MB", mem_used);
    assert!(mem_used < 3.0, "Memory usage exceeds 3MB limit");
    
    // Cleanup
    drop(clients);
    drop(server);
}
