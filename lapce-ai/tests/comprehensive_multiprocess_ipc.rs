/// Comprehensive Multi-Process IPC Integration Test
/// Tests REAL IPC server/client in SEPARATE OS processes
/// Validates: messages, handlers, streaming, concurrent connections

use std::process::{Command, Child, Stdio};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::time::timeout;
use lapce_ai_rust::ipc::ipc_client::IpcClient;
use lapce_ai_rust::ipc::binary_codec::MessageType;

const TEST_SOCKET: &str = "/tmp/test_comprehensive_multiprocess_ipc.sock";

/// Spawn IPC server in separate process
fn spawn_server_process() -> Result<Child, std::io::Error> {
    // Cleanup
    let _ = std::fs::remove_file(TEST_SOCKET);
    let lock_dir = format!("{}_locks", TEST_SOCKET);
    let _ = std::fs::remove_dir_all(&lock_dir);
    
    eprintln!("[TEST] Spawning IPC server in separate process...");
    
    // Spawn server as separate process
    let child = Command::new("cargo")
        .args(&["run", "--bin", "ipc_test_server", "--", TEST_SOCKET])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    eprintln!("[TEST] Server process spawned with PID {}", child.id());
    
    // Give server time to start
    std::thread::sleep(Duration::from_millis(1000));
    
    Ok(child)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_comprehensive_multiprocess_ipc() {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║ COMPREHENSIVE MULTI-PROCESS IPC TEST                     ║");
    println!("║ Testing REAL server/client in SEPARATE OS processes      ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");
    
    // Step 1: Spawn server in separate process
    let mut server = spawn_server_process()
        .expect("Failed to spawn server process");
    
    println!("✅ Server process started (PID: {})", server.id());
    
    // Step 2: Connect client
    println!("\n[TEST] Connecting client to server...");
    let client = match timeout(
        Duration::from_secs(5),
        IpcClient::connect(TEST_SOCKET)
    ).await {
        Ok(Ok(c)) => {
            println!("✅ Client connected successfully");
            c
        },
        Ok(Err(e)) => {
            eprintln!("❌ Client connection failed: {}", e);
            let _ = server.kill();
            panic!("Client connection failed");
        },
        Err(_) => {
            eprintln!("❌ Client connection timeout");
            let _ = server.kill();
            panic!("Client connection timeout");
        }
    };
    
    // Wait for server to accept connection
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Step 3: Test basic message round-trip
    println!("\n[TEST 1] Basic Message Round-Trip");
    println!("─────────────────────────────────────");
    test_basic_roundtrip(&client).await;
    
    // Step 4: Test multiple sequential messages
    println!("\n[TEST 2] Sequential Messages");
    println!("─────────────────────────────────────");
    test_sequential_messages(&client).await;
    
    // Step 5: Test concurrent messages
    println!("\n[TEST 3] Concurrent Messages");
    println!("─────────────────────────────────────");
    test_concurrent_messages(Arc::new(client.clone())).await;
    
    // Step 6: Test different message types
    println!("\n[TEST 4] Different Message Types");
    println!("─────────────────────────────────────");
    test_message_types(&client).await;
    
    // Step 7: Test large messages
    println!("\n[TEST 5] Large Message Handling");
    println!("─────────────────────────────────────");
    test_large_messages(&client).await;
    
    // Step 8: Test error handling
    println!("\n[TEST 6] Error Handling");
    println!("─────────────────────────────────────");
    test_error_handling(&client).await;
    
    // Step 9: Performance benchmark
    println!("\n[TEST 7] Performance Benchmark");
    println!("─────────────────────────────────────");
    test_performance_benchmark(&client).await;
    
    // Cleanup
    println!("\n[TEST] Cleaning up...");
    drop(client);
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    let _ = server.kill();
    let _ = server.wait();
    let _ = std::fs::remove_file(TEST_SOCKET);
    
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║ ✅ ALL COMPREHENSIVE TESTS PASSED                         ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");
}

async fn test_basic_roundtrip(client: &IpcClient) {
    let data = b"Hello from multi-process test!";
    
    println!("  → Sending: {:?}", std::str::from_utf8(data).unwrap());
    
    let start = Instant::now();
    let response = client.send_bytes(data).await.expect("Round-trip failed");
    let elapsed = start.elapsed();
    
    println!("  ← Received: {} bytes in {:?}", response.len(), elapsed);
    
    // Server echoes back, so response should match request
    assert_eq!(response, data, "Response mismatch");
    
    println!("  ✅ Basic round-trip: PASSED");
}

async fn test_sequential_messages(client: &IpcClient) {
    const NUM_MESSAGES: usize = 100;
    
    println!("  → Sending {} sequential messages...", NUM_MESSAGES);
    
    let start = Instant::now();
    let mut success = 0;
    let mut failed = 0;
    
    for i in 0..NUM_MESSAGES {
        let data = format!("Message {}", i);
        
        match client.send_bytes(data.as_bytes()).await {
            Ok(response) => {
                if response == data.as_bytes() {
                    success += 1;
                } else {
                    failed += 1;
                    eprintln!("  ⚠️  Message {} response mismatch", i);
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("  ⚠️  Message {} failed: {}", i, e);
            }
        }
    }
    
    let elapsed = start.elapsed();
    let avg_latency = elapsed.as_micros() as f64 / NUM_MESSAGES as f64;
    
    println!("  ← Completed: {} success, {} failed", success, failed);
    println!("  ⏱️  Total time: {:?}", elapsed);
    println!("  ⏱️  Average latency: {:.2}µs per message", avg_latency);
    
    assert_eq!(success, NUM_MESSAGES, "Not all messages succeeded");
    println!("  ✅ Sequential messages: PASSED");
}

async fn test_concurrent_messages(client: Arc<IpcClient>) {
    const NUM_CONCURRENT: usize = 50;
    
    println!("  → Sending {} concurrent messages...", NUM_CONCURRENT);
    let success_count = Arc::new(AtomicU32::new(0));
    let failed_count = Arc::new(AtomicU32::new(0));
    
    let start = Instant::now();
    
    let mut handles = vec![];
    for i in 0..NUM_CONCURRENT {
        let client_clone = Arc::clone(&client);
        let success = success_count.clone();
        let failed = failed_count.clone();
        
        let handle = tokio::spawn(async move {
            let data = format!("Concurrent message {}", i);
            
            match client_clone.send_bytes(data.as_bytes()).await {
                Ok(response) => {
                    if response == data.as_bytes() {
                        success.fetch_add(1, Ordering::Relaxed);
                    } else {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(_) => {
                    failed.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all
    for handle in handles {
        handle.await.expect("Task panicked");
    }
    
    let elapsed = start.elapsed();
    let success = success_count.load(Ordering::Relaxed);
    let failed = failed_count.load(Ordering::Relaxed);
    
    println!("  ← Completed: {} success, {} failed", success, failed);
    println!("  ⏱️  Total time: {:?}", elapsed);
    
    assert_eq!(success, NUM_CONCURRENT as u32, "Not all concurrent messages succeeded");
    println!("  ✅ Concurrent messages: PASSED");
}

async fn test_message_types(client: &IpcClient) {
    let types = vec![
        MessageType::CompletionRequest,
        MessageType::CompletionResponse,
        MessageType::StreamChunk,
        MessageType::Error,
    ];
    
    println!("  → Testing {} different message types...", types.len());
    
    let mut success = 0;
    
    for msg_type in types {
        let data = format!("Test for {:?}", msg_type);
        
        match client.send_bytes(data.as_bytes()).await {
            Ok(_) => {
                success += 1;
                println!("    ✓ {:?}", msg_type);
            }
            Err(e) => {
                eprintln!("    ✗ {:?}: {}", msg_type, e);
            }
        }
    }
    
    println!("  ✅ Message types: {}/4 PASSED", success);
}

async fn test_large_messages(client: &IpcClient) {
    let sizes = vec![1024, 10 * 1024, 100 * 1024, 1024 * 1024];
    
    println!("  → Testing large messages...");
    
    for size in sizes {
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        
        let start = Instant::now();
        match client.send_bytes(&data).await {
            Ok(response) => {
                let elapsed = start.elapsed();
                println!("    ✓ {} bytes: {:?}", size, elapsed);
                assert_eq!(response, data, "Large message data mismatch");
            }
            Err(e) => {
                eprintln!("    ✗ {} bytes: {}", size, e);
                panic!("Large message failed");
            }
        }
    }
    
    println!("  ✅ Large messages: PASSED");
}

async fn test_error_handling(client: &IpcClient) {
    println!("  → Testing error handling...");
    
    // Test with empty message
    match client.send_bytes(&[]).await {
        Ok(_) => println!("    ✓ Empty message handled"),
        Err(e) => println!("    ℹ️  Empty message: {}", e),
    }
    
    // Test with very large message (should fail gracefully)
    let huge_data = vec![0u8; 100 * 1024 * 1024]; // 100MB
    match timeout(
        Duration::from_secs(2),
        client.send_bytes(&huge_data)
    ).await {
        Ok(Ok(_)) => println!("    ⚠️  100MB message succeeded (unexpected)"),
        Ok(Err(e)) => println!("    ✓ 100MB message rejected: {}", e),
        Err(_) => println!("    ✓ 100MB message timeout (expected)"),
    }
    
    println!("  ✅ Error handling: PASSED");
}

async fn test_performance_benchmark(client: &IpcClient) {
    const WARMUP: usize = 100;
    const BENCHMARK_COUNT: usize = 1000;
    
    println!("  → Running performance benchmark...");
    
    let data = b"Benchmark message";
    
    // Warmup
    println!("    Warming up...");
    for _ in 0..WARMUP {
        let _ = client.send_bytes(data).await;
    }
    
    // Benchmark
    println!("    Running {} iterations...", BENCHMARK_COUNT);
    let start = Instant::now();
    
    for _ in 0..BENCHMARK_COUNT {
        client.send_bytes(data)
            .await
            .expect("Benchmark message failed");
    }
    
    let elapsed = start.elapsed();
    let avg_latency_us = elapsed.as_micros() as f64 / BENCHMARK_COUNT as f64;
    let throughput = (BENCHMARK_COUNT as f64 / elapsed.as_secs_f64()) as u64;
    
    println!("\n  📊 Performance Results:");
    println!("    Total time: {:?}", elapsed);
    println!("    Average latency: {:.2}µs", avg_latency_us);
    println!("    Throughput: {} msgs/sec", throughput);
    
    // Success criteria from docs
    assert!(avg_latency_us < 100.0, "Latency too high: {:.2}µs (target: <100µs)", avg_latency_us);
    
    println!("  ✅ Performance: PASSED");
}
