#![cfg(any(target_os = "linux", target_os = "macos"))]
/// Nuclear Test 3: Latency Torture
/// 999 background connections + 1 test connection
/// Target: <10μs latency in 99%+ of 10,000 messages under max load

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use lapce_ai_rust::{IpcServer, IpcConfig};
use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryStream;
use bytes::Bytes;

const BACKGROUND_CONNECTIONS: usize = 999;
const TEST_MESSAGES: usize = 10000;
const MESSAGE_SIZE: usize = 256;

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn nuclear_latency_torture() {
    println!("\n⚡ NUCLEAR TEST 3: LATENCY TORTURE");
    println!("===================================");
    println!("Background connections: {}", BACKGROUND_CONNECTIONS);
    println!("Test messages: {}", TEST_MESSAGES);
    println!("Target: <10μs P99 latency\n");
    
    let start_time = Instant::now();
    let mut latencies = Vec::with_capacity(TEST_MESSAGES);
    
    // Start IPC server
    let socket_path = "/tmp/lapce_nuclear_3.sock";
    let server = Arc::new(IpcServer::new(socket_path).await.unwrap());
    
    // Register fast echo handler
    server.register_handler(MessageType::Echo, |data| async move {
        Ok(data) // Immediate echo
    });
    
    // Start server
    let server_handle = {
        let server = server.clone();
        tokio::spawn(async move {
            server.serve().await.unwrap();
        })
    };
    
    sleep(Duration::from_millis(100)).await;
    
    // Start background load generators
    let stop_signal = Arc::new(AtomicU64::new(0));
    let mut background_handles = Vec::new();
    
    for _ in 0..BACKGROUND_CONNECTIONS {
        let stop = stop_signal.clone();
        let handle = tokio::spawn(async move {
            let mut stream = SharedMemoryStream::connect(socket_path)
                .await
                .expect("Failed to connect");
            
            let message = vec![0u8; MESSAGE_SIZE];
            
            while stop.load(Ordering::Relaxed) == 0 {
                // Continuous load
                stream.write_all(&message).await.expect("Write failed");
                let mut response = vec![0u8; MESSAGE_SIZE];
                stream.read_exact(&mut response).await.expect("Read failed");
            }
        });
        background_handles.push(handle);
    }
    
    // Wait for background load to stabilize
    sleep(Duration::from_secs(2)).await;
    println!("Background load established, starting latency test...");
    
    // Test connection for latency measurement
    let mut test_stream = SharedMemoryStream::connect(socket_path)
        .await
        .expect("Failed to connect test stream");
    
    // Measure latencies under load
    for i in 0..TEST_MESSAGES {
        let message = vec![(i % 256) as u8; MESSAGE_SIZE];
        
        let start = Instant::now();
        
        // Send
        test_stream.write_all(&message).await.expect("Write failed");
        
        // Receive
        let mut response = vec![0u8; MESSAGE_SIZE];
        test_stream.read_exact(&mut response).await.expect("Read failed");
        
        let latency = start.elapsed();
        latencies.push(latency.as_micros() as u64);
        
        if i % 1000 == 0 {
            println!("Tested {} messages...", i);
        }
    }
    
    // Stop background load
    stop_signal.store(1, Ordering::Relaxed);
    for handle in background_handles {
        handle.abort();
    }
    
    // Analyze latencies
    latencies.sort_unstable();
    let p50 = latencies[latencies.len() * 50 / 100];
    let p95 = latencies[latencies.len() * 95 / 100];
    let p99 = latencies[latencies.len() * 99 / 100];
    let p999 = latencies[latencies.len() * 999 / 1000];
    let min = *latencies.first().unwrap();
    let max = *latencies.last().unwrap();
    
    let total_time = start_time.elapsed();
    
    println!("\n📊 RESULTS");
    println!("==========");
    println!("Test duration: {:.2}s", total_time.as_secs_f64());
    println!("Background load: {} connections", BACKGROUND_CONNECTIONS);
    println!("\nLatency Distribution:");
    println!("  Min: {:.2} μs", min);
    println!("  P50: {:.2} μs", p50);
    println!("  P95: {:.2} μs", p95);
    println!("  P99: {:.2} μs", p99);
    println!("  P99.9: {:.2} μs", p999);
    println!("  Max: {:.2} μs", max);
    
    // Count how many are under 10μs
    let under_10us = latencies.iter().filter(|&&l| l < 10).count();
    let percentage = (under_10us as f64 / latencies.len() as f64) * 100.0;
    
    println!("\n{:.2}% of messages under 10μs", percentage);
    
    // Validation
    if p99 < 10 {
        println!("\n✅ SUCCESS: P99 latency {:.2}μs < 10μs", p99);
    } else {
        println!("\n❌ FAILED: P99 latency {:.2}μs >= 10μs", p99);
        panic!("Did not meet latency target");
    }
    
    server_handle.abort();
}
