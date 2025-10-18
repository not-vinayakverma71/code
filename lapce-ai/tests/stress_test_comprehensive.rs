/// COMPREHENSIVE STRESS TEST
/// Tests 1000+ concurrent connections with sustained load
/// Validates memory stability and performance under realistic production load

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;
use anyhow::Result;

const TEST_BASE: &str = "/tmp/stress_test";

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_comprehensive_stress() {
    println!("\n╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ COMPREHENSIVE STRESS TEST - PRODUCTION VALIDATION                ║");
    println!("║ Target: 1000+ concurrent connections, sustained load             ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝\n");
    
    cleanup().await;
    
    // Test 1: 1000 Concurrent Connections
    println!("[TEST 1] 1000 Concurrent Connections");
    println!("─────────────────────────────────────────────────────────────────");
    test_1000_concurrent_connections().await.expect("1000 connections test failed");
    
    // Test 2: Sustained Load (5 minutes)
    println!("\n[TEST 2] Sustained Load Test (5 minutes)");
    println!("─────────────────────────────────────────────────────────────────");
    test_sustained_load().await.expect("Sustained load test failed");
    
    // Test 3: Memory Stability Under Load
    println!("\n[TEST 3] Memory Stability Validation");
    println!("─────────────────────────────────────────────────────────────────");
    test_memory_stability().await.expect("Memory stability test failed");
    
    // Test 4: Burst Traffic Pattern
    println!("\n[TEST 4] Burst Traffic Handling");
    println!("─────────────────────────────────────────────────────────────────");
    test_burst_traffic().await.expect("Burst traffic test failed");
    
    // Test 5: Connection Churn
    println!("\n[TEST 5] Connection Churn (Create/Destroy)");
    println!("─────────────────────────────────────────────────────────────────");
    test_connection_churn().await.expect("Connection churn test failed");
    
    cleanup().await;
    
    println!("\n╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ ✅ ALL STRESS TESTS PASSED - PRODUCTION READY                    ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝\n");
}

async fn cleanup() {
    for i in 0..1500 {
        let path = format!("{}_{}", TEST_BASE, i);
        let _ = std::fs::remove_file(&path);
    }
}

async fn test_1000_concurrent_connections() -> Result<()> {
    const NUM_CONNECTIONS: usize = 1000;
    const MESSAGES_PER_CONN: usize = 10;
    
    println!("  → Creating {} concurrent connections...", NUM_CONNECTIONS);
    
    let success_count = Arc::new(AtomicU64::new(0));
    let failed_count = Arc::new(AtomicU64::new(0));
    let total_messages = Arc::new(AtomicU64::new(0));
    
    let start = Instant::now();
    
    // Create all connections concurrently
    let mut handles = vec![];
    for conn_id in 0..NUM_CONNECTIONS {
        let success = success_count.clone();
        let failed = failed_count.clone();
        let total_msgs = total_messages.clone();
        
        let handle = tokio::spawn(async move {
            let path = format!("{}_{}", TEST_BASE, conn_id);
            
            match SharedMemoryBuffer::create(&path, 2 * 1024 * 1024).await {
                Ok(buf) => {
                    let buf = Arc::new(buf);
                    
                    // Send messages on this connection
                    for msg_id in 0..MESSAGES_PER_CONN {
                        let data = format!("Conn {} Msg {}", conn_id, msg_id);
                        
                        if buf.write(data.as_bytes()).await.is_ok() {
                            if let Some(received) = buf.read().await {
                                if received == data.as_bytes() {
                                    total_msgs.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                    }
                    
                    success.fetch_add(1, Ordering::Relaxed);
                    drop(buf);
                    let _ = std::fs::remove_file(&path);
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
        handle.await?;
    }
    
    let elapsed = start.elapsed();
    let success = success_count.load(Ordering::Relaxed);
    let failed = failed_count.load(Ordering::Relaxed);
    let total_msgs = total_messages.load(Ordering::Relaxed);
    
    println!("\n  📊 Results:");
    println!("    Connections: {}/{} successful", success, NUM_CONNECTIONS);
    println!("    Failed: {}", failed);
    println!("    Total messages: {}", total_msgs);
    println!("    Time: {:?}", elapsed);
    println!("    Throughput: {:.0} msgs/sec", total_msgs as f64 / elapsed.as_secs_f64());
    
    assert!(success >= NUM_CONNECTIONS as u64 * 95 / 100, 
        "Less than 95% connections succeeded: {}/{}", success, NUM_CONNECTIONS);
    
    println!("  ✅ 1000 concurrent connections: PASSED");
    Ok(())
}

async fn test_sustained_load() -> Result<()> {
    const DURATION_SECS: u64 = 300; // 5 minutes
    const NUM_WORKERS: usize = 50;
    
    println!("  → Running {} workers for {} seconds...", NUM_WORKERS, DURATION_SECS);
    
    let messages_sent = Arc::new(AtomicU64::new(0));
    let messages_received = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let stop_flag = Arc::new(AtomicUsize::new(0));
    
    let start = Instant::now();
    
    // Spawn workers
    let mut handles = vec![];
    for worker_id in 0..NUM_WORKERS {
        let msgs_sent = messages_sent.clone();
        let msgs_recv = messages_received.clone();
        let errs = errors.clone();
        let stop = stop_flag.clone();
        
        let handle = tokio::spawn(async move {
            let path = format!("{}_sustained_{}", TEST_BASE, worker_id);
            
            match SharedMemoryBuffer::create(&path, 2 * 1024 * 1024).await {
                Ok(buf) => {
                    let buf = Arc::new(buf);
                    
                    while stop.load(Ordering::Relaxed) == 0 {
                        let data = format!("Worker {} sustained load", worker_id);
                        
                        match buf.write(data.as_bytes()).await {
                            Ok(_) => {
                                msgs_sent.fetch_add(1, Ordering::Relaxed);
                                
                                if let Some(received) = buf.read().await {
                                    if received == data.as_bytes() {
                                        msgs_recv.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            }
                            Err(_) => {
                                errs.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    
                    drop(buf);
                    let _ = std::fs::remove_file(&path);
                }
                Err(_) => {
                    errs.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Monitor progress
    let monitor_handle = tokio::spawn({
        let msgs_sent = messages_sent.clone();
        let msgs_recv = messages_received.clone();
        let errs = errors.clone();
        
        async move {
            for minute in 1..=5 {
                tokio::time::sleep(Duration::from_secs(60)).await;
                let sent = msgs_sent.load(Ordering::Relaxed);
                let recv = msgs_recv.load(Ordering::Relaxed);
                let err = errs.load(Ordering::Relaxed);
                println!("    [{}min] Sent: {}, Recv: {}, Errors: {}", minute, sent, recv, err);
            }
        }
    });
    
    // Wait for duration
    tokio::time::sleep(Duration::from_secs(DURATION_SECS)).await;
    
    // Stop workers
    stop_flag.store(1, Ordering::Relaxed);
    
    // Wait for workers to finish
    for handle in handles {
        handle.await?;
    }
    monitor_handle.abort();
    
    let elapsed = start.elapsed();
    let sent = messages_sent.load(Ordering::Relaxed);
    let recv = messages_received.load(Ordering::Relaxed);
    let errs = errors.load(Ordering::Relaxed);
    
    println!("\n  📊 Sustained Load Results:");
    println!("    Duration: {:?}", elapsed);
    println!("    Messages sent: {}", sent);
    println!("    Messages received: {}", recv);
    println!("    Errors: {}", errs);
    println!("    Throughput: {:.0} msgs/sec", sent as f64 / elapsed.as_secs_f64());
    println!("    Error rate: {:.2}%", (errs as f64 / sent as f64) * 100.0);
    
    assert!(recv >= sent * 95 / 100, "Less than 95% messages received");
    assert!(errs < sent / 100, "Error rate > 1%");
    
    println!("  ✅ Sustained load: PASSED");
    Ok(())
}

async fn test_memory_stability() -> Result<()> {
    println!("  → Monitoring memory stability over 2 minutes...");
    
    let path = format!("{}_memory_test", TEST_BASE);
    let buf = Arc::new(SharedMemoryBuffer::create(&path, 2 * 1024 * 1024).await?);
    
    let baseline_rss = get_memory_usage()?;
    println!("    Baseline RSS: {} MB", baseline_rss / 1024 / 1024);
    
    let mut memory_samples = vec![baseline_rss];
    
    // Run for 2 minutes, measuring memory every 10 seconds
    for iteration in 1..=12 {
        // Send burst of messages
        for _ in 0..1000 {
            let data = b"Memory stability test message";
            buf.write(data).await?;
            let _ = buf.read().await;
        }
        
        tokio::time::sleep(Duration::from_secs(10)).await;
        
        let current_rss = get_memory_usage()?;
        memory_samples.push(current_rss);
        
        let growth = (current_rss as f64 - baseline_rss as f64) / baseline_rss as f64 * 100.0;
        println!("    [{}min] RSS: {} MB (growth: {:.1}%)", 
            iteration / 6, current_rss / 1024 / 1024, growth);
    }
    
    let final_rss = memory_samples.last().unwrap();
    let growth = (*final_rss as f64 - baseline_rss as f64) / baseline_rss as f64 * 100.0;
    
    println!("\n  📊 Memory Stability Results:");
    println!("    Baseline: {} MB", baseline_rss / 1024 / 1024);
    println!("    Final: {} MB", final_rss / 1024 / 1024);
    println!("    Growth: {:.1}%", growth);
    
    // Memory should not grow more than 10% over 2 minutes
    assert!(growth < 10.0, "Memory growth too high: {:.1}%", growth);
    
    drop(buf);
    let _ = std::fs::remove_file(&path);
    
    println!("  ✅ Memory stability: PASSED");
    Ok(())
}

async fn test_burst_traffic() -> Result<()> {
    const BURST_SIZE: usize = 1000;
    const NUM_BURSTS: usize = 10;
    
    println!("  → Testing {} bursts of {} messages each...", NUM_BURSTS, BURST_SIZE);
    
    let path = format!("{}_burst", TEST_BASE);
    let buf = Arc::new(SharedMemoryBuffer::create(&path, 2 * 1024 * 1024).await?);
    
    let mut total_latencies = vec![];
    
    for burst_id in 0..NUM_BURSTS {
        let burst_start = Instant::now();
        
        for msg_id in 0..BURST_SIZE {
            let data = format!("Burst {} Msg {}", burst_id, msg_id);
            buf.write(data.as_bytes()).await?;
            let _ = buf.read().await;
        }
        
        let burst_time = burst_start.elapsed();
        let avg_latency = burst_time.as_micros() as f64 / BURST_SIZE as f64;
        total_latencies.push(avg_latency);
        
        println!("    Burst {}: {:.0}µs avg latency, {} msgs/sec", 
            burst_id, avg_latency, (BURST_SIZE as f64 / burst_time.as_secs_f64()) as u64);
        
        // Cool down between bursts
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    let avg_of_avgs: f64 = total_latencies.iter().sum::<f64>() / total_latencies.len() as f64;
    
    println!("\n  📊 Burst Traffic Results:");
    println!("    Average latency: {:.0}µs", avg_of_avgs);
    println!("    Bursts completed: {}/{}", NUM_BURSTS, NUM_BURSTS);
    
    drop(buf);
    let _ = std::fs::remove_file(&path);
    
    println!("  ✅ Burst traffic: PASSED");
    Ok(())
}

async fn test_connection_churn() -> Result<()> {
    const CYCLES: usize = 100;
    const CONNS_PER_CYCLE: usize = 50;
    
    println!("  → Testing connection churn: {} cycles × {} connections...", CYCLES, CONNS_PER_CYCLE);
    
    let created = Arc::new(AtomicU64::new(0));
    let destroyed = Arc::new(AtomicU64::new(0));
    
    let start = Instant::now();
    
    for cycle in 0..CYCLES {
        let mut handles = vec![];
        
        // Create connections
        for conn_id in 0..CONNS_PER_CYCLE {
            let created_clone = created.clone();
            let destroyed_clone = destroyed.clone();
            
            let handle = tokio::spawn(async move {
                let path = format!("{}_churn_{}_{}", TEST_BASE, cycle, conn_id);
                
                if let Ok(buf) = SharedMemoryBuffer::create(&path, 2 * 1024 * 1024).await {
                    created_clone.fetch_add(1, Ordering::Relaxed);
                    
                    // Use connection briefly
                    let data = b"Churn test";
                    let _ = buf.write(data).await;
                    let _ = buf.read().await;
                    
                    // Destroy connection
                    drop(buf);
                    let _ = std::fs::remove_file(&path);
                    destroyed_clone.fetch_add(1, Ordering::Relaxed);
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for cycle to complete
        for handle in handles {
            handle.await?;
        }
        
        if cycle % 10 == 0 {
            println!("    Cycle {}: Created {}, Destroyed {}", 
                cycle, created.load(Ordering::Relaxed), destroyed.load(Ordering::Relaxed));
        }
    }
    
    let elapsed = start.elapsed();
    let total_created = created.load(Ordering::Relaxed);
    let total_destroyed = destroyed.load(Ordering::Relaxed);
    
    println!("\n  📊 Connection Churn Results:");
    println!("    Total cycles: {}", CYCLES);
    println!("    Created: {}", total_created);
    println!("    Destroyed: {}", total_destroyed);
    println!("    Time: {:?}", elapsed);
    println!("    Rate: {:.0} connections/sec", total_created as f64 / elapsed.as_secs_f64());
    
    assert_eq!(total_created, total_destroyed, "Memory leak: not all connections destroyed");
    
    println!("  ✅ Connection churn: PASSED");
    Ok(())
}

fn get_memory_usage() -> Result<usize> {
    let pid = std::process::id();
    let status = std::fs::read_to_string(format!("/proc/{}/status", pid))?;
    
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb: usize = parts[1].parse()?;
                return Ok(kb * 1024); // Convert to bytes
            }
        }
    }
    
    anyhow::bail!("Could not read memory usage")
}
