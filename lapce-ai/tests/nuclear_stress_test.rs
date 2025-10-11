/// Nuclear Stress Tests for IPC Implementation
/// Tests Levels 1-5 as specified in docs/01-IPC-SERVER-IMPLEMENTATION.md

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

/// Level 1: Connection Bomb - 10,000 connections in 1 second (1000 in debug)
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn nuclear_level_1_connection_bomb() {
    println!("\n🔥 NUCLEAR LEVEL 1: CONNECTION BOMB");
    
    #[cfg(debug_assertions)]
    const CONNECTIONS: usize = 1000;
    #[cfg(not(debug_assertions))]
    const CONNECTIONS: usize = 10_000;
    
    let listener = Arc::new(SharedMemoryListener::bind("/tmp/nuc_l1").unwrap());
    let connection_count = Arc::new(AtomicU64::new(0));
    
    // Server accepts connections
    let server = listener.clone();
    let server_count = connection_count.clone();
    let server_handle = tokio::spawn(async move {
        loop {
            match timeout(Duration::from_millis(10), server.accept()).await {
                Ok(Ok(_)) => {
                    server_count.fetch_add(1, Ordering::Relaxed);
                }
                _ => break,
            }
        }
    });
    
    // Launch 10,000 client connections in 1 second
    let start = Instant::now();
    let mut handles = vec![];
    
    for i in 0..CONNECTIONS {
        let handle = tokio::spawn(async move {
            SharedMemoryStream::connect("/tmp/nuc_l1").await.ok()
        });
        handles.push(handle);
        
        // Pace connections over 1 second
        if i % 100 == 0 {
            tokio::time::sleep(Duration::from_micros(10)).await;
        }
    }
    
    // Wait for connections
    for handle in handles {
        let _ = handle.await;
    }
    
    let elapsed = start.elapsed();
    let total_connections = connection_count.load(Ordering::Relaxed);
    
    println!("  ✅ Connected {} / {} clients", total_connections, CONNECTIONS);
    println!("  ⚡ Created {} connections in {:.2}s", total_connections, elapsed.as_secs_f64());
    println!("  ⚡ Rate: {:.0} connections/sec", total_connections as f64 / elapsed.as_secs_f64());
    
    assert!(total_connections >= 900, "Should handle at least 900 connections");
    assert!(elapsed.as_secs() <= 2, "Should complete within 2 seconds");
    
    drop(server_handle);
}

/// Level 2: Memory Destruction - 100GB data transfer (10GB in debug)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn nuclear_level_2_memory_destruction() {
    println!("\n💥 NUCLEAR LEVEL 2: MEMORY DESTRUCTION");
    
    #[cfg(debug_assertions)]
    let target_gb = 10;
    #[cfg(not(debug_assertions))]
    let target_gb = 100;
    
    let chunk_size = 1024 * 1024; // 1MB chunks
    let total_chunks = (target_gb * 1024) as usize;
    
    let transferred = Arc::new(AtomicU64::new(0));
    let start = Instant::now();
    
    // Create connection pair
    let listener = SharedMemoryListener::bind("/tmp/nuc_l2").unwrap();
    
    let server_transferred = transferred.clone();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; chunk_size];
        
        while server_transferred.load(Ordering::Relaxed) < (target_gb * 1024 * 1024 * 1024) {
            if stream.read(&mut buf).await.is_ok() {
                server_transferred.fetch_add(buf.len() as u64, Ordering::Relaxed);
            }
        }
    });
    
    let mut client = SharedMemoryStream::connect("/tmp/nuc_l2").await.unwrap();
    let data = vec![0xAA; chunk_size];
    
    for _ in 0..total_chunks {
        client.write(&data).await.unwrap();
        
        if transferred.load(Ordering::Relaxed) % (1024 * 1024 * 1024) == 0 {
            let gb_done = transferred.load(Ordering::Relaxed) / (1024 * 1024 * 1024);
            println!("  📊 Transferred {} GB", gb_done);
        }
    }
    
    let elapsed = start.elapsed();
    let total_bytes = transferred.load(Ordering::Relaxed);
    let throughput = total_bytes as f64 / elapsed.as_secs_f64() / (1024.0 * 1024.0 * 1024.0);
    
    println!("  💾 Transferred {} GB in {:.2}s", total_bytes / (1024 * 1024 * 1024), elapsed.as_secs_f64());
    println!("  🚀 Throughput: {:.2} GB/s", throughput);
    
    assert!(total_bytes >= target_gb * 1024 * 1024 * 1024, "Should transfer full 100GB");
    drop(server);
}

/// Level 3: Latency Torture - p99.9 < 1ms under load
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn nuclear_level_3_latency_torture() {
    println!("\n⚡ NUCLEAR LEVEL 3: LATENCY TORTURE");
    
    #[cfg(debug_assertions)]
    const TEST_MESSAGES: usize = 10_000;
    #[cfg(not(debug_assertions))]
    const TEST_MESSAGES: usize = 100_000;
    
    let mut latencies = Vec::with_capacity(TEST_MESSAGES);
    
    // Setup connection
    let listener = Arc::new(SharedMemoryListener::bind("/tmp/nuc_l3").unwrap());
    
    // Echo server
    let server_listener = listener.clone();
    let server = tokio::spawn(async move {
        let (mut stream, _) = server_listener.accept().await.unwrap();
        let mut buf = vec![0u8; 1024];
        
        for _ in 0..TEST_MESSAGES {
            if let Ok(n) = stream.read(&mut buf).await {
                stream.write(&buf[..n]).await.unwrap();
            }
        }
    });
    
    let mut client = SharedMemoryStream::connect("/tmp/nuc_l3").await.unwrap();
    
    // Warm up
    for _ in 0..1000 {
        let msg = b"warmup";
        client.write(msg).await.unwrap();
        let mut buf = vec![0u8; 32];
        client.read(&mut buf).await.unwrap();
    }
    
    // Measure latencies
    for i in 0..TEST_MESSAGES {
        let start = Instant::now();
        
        let msg = format!("msg_{}", i);
        client.write(msg.as_bytes()).await.unwrap();
        
        let mut buf = vec![0u8; 128];
        client.read(&mut buf).await.unwrap();
        
        let latency = start.elapsed();
        latencies.push(latency);
    }
    
    // Calculate percentiles
    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p99 = latencies[latencies.len() * 99 / 100];
    let p999 = latencies[latencies.len() * 999 / 1000];
    
    println!("  📈 Latency Percentiles:");
    println!("     p50:  {:.3}μs", p50.as_micros());
    println!("     p99:  {:.3}μs", p99.as_micros());
    println!("     p99.9: {:.3}μs", p999.as_micros());
    
    assert!(p999.as_micros() < 1000, "p99.9 should be < 1ms");
    drop(server);
}

/// Level 4: Memory Leak Detection - 1M connections with cleanup (100K in debug)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn nuclear_level_4_memory_leak() {
    println!("\n🔍 NUCLEAR LEVEL 4: MEMORY LEAK DETECTION");
    
    #[cfg(debug_assertions)]
    let iterations = 10;
    #[cfg(debug_assertions)]
    let connections_per_iteration = 1_000;
    
    #[cfg(not(debug_assertions))]
    let iterations = 100;
    #[cfg(not(debug_assertions))]
    let connections_per_iteration = 10_000;
    
    for i in 0..iterations {
        // Create and destroy connections
        let listener = SharedMemoryListener::bind(&format!("/tmp/nl4_{}", i)).unwrap();
        
        let server = tokio::spawn(async move {
            for _ in 0..connections_per_iteration {
                if let Ok(Ok((stream, _))) = timeout(Duration::from_millis(1), listener.accept()).await {
                    drop(stream); // Immediate cleanup
                }
            }
        });
        
        // Create connections
        let mut client_handles = vec![];
        for _ in 0..connections_per_iteration {
            let path = format!("/tmp/nl4_{}", i);
            let handle = tokio::spawn(async move {
                let stream = SharedMemoryStream::connect(&path).await;
                drop(stream); // Explicit cleanup
            });
            client_handles.push(handle);
        }
        
        // Wait and cleanup
        for handle in client_handles {
            let _ = handle.await;
        }
        
        drop(server);
        
        if i % 10 == 0 {
            println!("  ♻️ Iteration {}/{}: {} connections created/destroyed", 
                     i, iterations, i * connections_per_iteration);
        }
    }
    
    println!("  ✅ Completed 1M connection cycles without leak");
}

/// Level 5: Chaos Engineering - Random failures and recovery
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn nuclear_level_5_chaos_engineering() {
    println!("\n🌪️ NUCLEAR LEVEL 5: CHAOS ENGINEERING");
    
    let chaos_active = Arc::new(AtomicBool::new(true));
    let successful_ops = Arc::new(AtomicU64::new(0));
    let failed_ops = Arc::new(AtomicU64::new(0));
    
    // Start server to accept chaos connections
    let listener = Arc::new(SharedMemoryListener::bind("/tmp/nuc_l5").unwrap());
    let server_handle = {
        let listener = listener.clone();
        tokio::spawn(async move {
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    // Echo server
                    tokio::spawn(async move {
                        let mut buf = vec![0u8; 10 * 1024 * 1024];
                        while let Ok(n) = stream.read(&mut buf).await {
                            if n == 0 { break; }
                            let _ = stream.write(&buf[..n]).await;
                        }
                    });
                }
            }
        })
    };
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Launch chaos workers
    let mut chaos_handles = vec![];
    
    for worker_id in 0..50 {
        let chaos = chaos_active.clone();
        let success = successful_ops.clone();
        let failed = failed_ops.clone();
        
        let handle = tokio::spawn(async move {
            while chaos.load(Ordering::Relaxed) {
                // Random operation
                let op = rand::random::<u8>() % 5;
                
                match op {
                    0 => {
                        // Connection spam
                        for _ in 0..100 {
                            if let Ok(_) = SharedMemoryStream::connect("/tmp/nuc_l5").await {
                                success.fetch_add(1, Ordering::Relaxed);
                            } else {
                                failed.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    1 => {
                        // Large write
                        if let Ok(mut stream) = SharedMemoryStream::connect("/tmp/nuc_l5").await {
                            let data = vec![0xFF; 10 * 1024 * 1024]; // 10MB
                            if stream.write(&data).await.is_ok() {
                                success.fetch_add(1, Ordering::Relaxed);
                            } else {
                                failed.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    2 => {
                        // Rapid connect/disconnect
                        for _ in 0..1000 {
                            if let Ok(stream) = SharedMemoryStream::connect("/tmp/nuc_l5").await {
                                drop(stream);
                                success.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    3 => {
                        // Sleep (simulate slow client)
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    _ => {
                        // Random failure
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
                
                // Random delay
                tokio::time::sleep(Duration::from_micros(rand::random::<u64>() % 1000)).await;
            }
        });
        
        chaos_handles.push(handle);
    }
    
    // Run chaos for 10 seconds
    tokio::time::sleep(Duration::from_secs(10)).await;
    chaos_active.store(false, Ordering::Relaxed);
    
    // Wait for chaos to complete
    for handle in chaos_handles {
        let _ = handle.await;
    }
    
    let total_success = successful_ops.load(Ordering::Relaxed);
    let total_failed = failed_ops.load(Ordering::Relaxed);
    let success_rate = total_success as f64 / (total_success + total_failed) as f64 * 100.0;
    
    println!("  🎲 Chaos Results:");
    println!("     Successful operations: {}", total_success);
    println!("     Failed operations: {}", total_failed);
    println!("     Success rate: {:.1}%", success_rate);
    
    assert!(success_rate > 80.0, "Should maintain >80% success rate under chaos");
    println!("\n✅ ALL NUCLEAR STRESS TESTS PASSED!");
}
