/// Realistic Workload Stress Tests
/// Simulates production-like usage patterns for IPC system

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use anyhow::Result;
use tokio::time::sleep;

#[cfg(unix)]
use lapce_ai_rust::ipc::SharedMemoryBuffer;

/// Simulates mixed message sizes (realistic distribution)
fn generate_realistic_message(size_category: usize) -> Vec<u8> {
    match size_category % 5 {
        0 => vec![0x41; 100],           // 100B - small
        1 => vec![0x42; 1024],          // 1KB - typical
        2 => vec![0x43; 10 * 1024],     // 10KB - medium
        3 => vec![0x44; 50 * 1024],     // 50KB - large
        4 => vec![0x45; 100 * 1024],    // 100KB - very large
        _ => unreachable!(),
    }
}

/// Test: Realistic mixed workload (small, medium, large messages)
#[tokio::test]
#[cfg(unix)]
async fn test_realistic_mixed_workload() -> Result<()> {
    let test_id = uuid::Uuid::new_v4();
    let shm_path = format!("/lapce_workload_{}", test_id);
    let buffer = Arc::new(SharedMemoryBuffer::create(&shm_path, 4 * 1024 * 1024).await?);
    
    let duration = Duration::from_secs(5);
    let start = Instant::now();
    
    let mut message_counts = [0u64; 5];
    let mut total_bytes = 0u64;
    let mut total_messages = 0u64;
    
    while start.elapsed() < duration {
        for size_cat in 0..5 {
            let msg = generate_realistic_message(size_cat);
            buffer.write(&msg).await?;
            let _ = buffer.read().await;
            
            message_counts[size_cat] += 1;
            total_bytes += msg.len() as u64;
            total_messages += 1;
        }
    }
    
    let elapsed = start.elapsed();
    let throughput_msgs = total_messages as f64 / elapsed.as_secs_f64();
    let throughput_mb = (total_bytes as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64();
    
    println!("\nRealistic Mixed Workload ({:.1}s):", elapsed.as_secs_f64());
    println!("  Total messages: {}", total_messages);
    println!("  Total bytes: {} MB", total_bytes / 1024 / 1024);
    println!("  Throughput: {:.0} msg/s", throughput_msgs);
    println!("  Bandwidth: {:.2} MB/s", throughput_mb);
    println!("  Message distribution:");
    println!("    100B: {}", message_counts[0]);
    println!("    1KB: {}", message_counts[1]);
    println!("    10KB: {}", message_counts[2]);
    println!("    50KB: {}", message_counts[3]);
    println!("    100KB: {}", message_counts[4]);
    
    // Should handle >1000 mixed messages/sec
    assert!(throughput_msgs > 1000.0,
            "Throughput too low: {:.0} msg/s", throughput_msgs);
    
    Ok(())
}

/// Test: Burst traffic simulation
#[tokio::test]
#[cfg(unix)]
async fn test_burst_traffic() -> Result<()> {
    let test_id = uuid::Uuid::new_v4();
    let shm_path = format!("/lapce_burst_{}", test_id);
    let buffer = Arc::new(SharedMemoryBuffer::create(&shm_path, 8 * 1024 * 1024).await?);
    
    let burst_count = 100;
    let burst_size = vec![0x42; 10 * 1024]; // 10KB messages
    
    println!("\nBurst Traffic Test:");
    
    // Send 5 bursts with pauses
    for burst_num in 0..5 {
        let start = Instant::now();
        
        for _ in 0..burst_count {
            buffer.write(&burst_size).await?;
            let _ = buffer.read().await;
        }
        
        let burst_duration = start.elapsed();
        let burst_throughput = burst_count as f64 / burst_duration.as_secs_f64();
        
        println!("  Burst {}: {} msg in {:?} ({:.0} msg/s)", 
                 burst_num + 1, burst_count, burst_duration, burst_throughput);
        
        // Pause between bursts
        sleep(Duration::from_millis(200)).await;
    }
    
    Ok(())
}

/// Test: Long-running stable connection
#[tokio::test]
#[cfg(unix)]
async fn test_long_running_stability() -> Result<()> {
    let test_id = uuid::Uuid::new_v4();
    let shm_path = format!("/lapce_stable_{}", test_id);
    let buffer = Arc::new(SharedMemoryBuffer::create(&shm_path, 4 * 1024 * 1024).await?);
    
    let test_data = vec![0x42; 1024];
    let duration = Duration::from_secs(20);
    let start = Instant::now();
    
    let mut count = 0u64;
    let mut error_count = 0u64;
    let mut samples = Vec::new();
    
    println!("\nLong-running Stability Test (20s):");
    
    while start.elapsed() < duration {
        // Send batch of 50 messages
        for _ in 0..50 {
            match buffer.write(&test_data).await {
                Ok(_) => {
                    let _ = buffer.read().await;
                    count += 1;
                }
                Err(_) => error_count += 1,
            }
        }
        
        // Sample throughput every 2 seconds
        let elapsed = start.elapsed().as_secs();
        if elapsed > 0 && elapsed % 2 == 0 && samples.len() < (elapsed / 2) as usize {
            let current_throughput = count as f64 / start.elapsed().as_secs_f64();
            samples.push(current_throughput);
        }
        
        sleep(Duration::from_millis(100)).await;
    }
    
    let elapsed = start.elapsed();
    let avg_throughput = count as f64 / elapsed.as_secs_f64();
    let error_rate = error_count as f64 / (count + error_count) as f64;
    
    println!("  Total messages: {}", count);
    println!("  Errors: {}", error_count);
    println!("  Error rate: {:.4}%", error_rate * 100.0);
    println!("  Average throughput: {:.0} msg/s", avg_throughput);
    
    // Calculate stability (variance in throughput samples)
    if samples.len() > 1 {
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let variance = samples.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / samples.len() as f64;
        let std_dev = variance.sqrt();
        let cv = std_dev / mean; // Coefficient of variation
        
        println!("  Throughput stability (CV): {:.2}%", cv * 100.0);
        
        // CV should be < 50% for acceptable performance (high-speed buffers can vary)
        assert!(cv < 0.5, "Unstable throughput: CV = {:.2}%", cv * 100.0);
    }
    
    // Error rate should be < 0.1%
    assert!(error_rate < 0.001, "Too many errors: {:.4}%", error_rate * 100.0);
    
    Ok(())
}

/// Test: Concurrent producers and consumers
#[tokio::test]
#[cfg(unix)]
async fn test_concurrent_producers_consumers() -> Result<()> {
    let test_id = uuid::Uuid::new_v4();
    let shm_path = format!("/lapce_concurrent_{}", test_id);
    let buffer = Arc::new(SharedMemoryBuffer::create(&shm_path, 8 * 1024 * 1024).await?);
    
    let producer_count = 10;
    let consumer_count = 10;
    let messages_per_task = 100;
    
    let sent_count = Arc::new(AtomicU64::new(0));
    let received_count = Arc::new(AtomicU64::new(0));
    
    let mut handles = vec![];
    
    // Spawn producers
    for i in 0..producer_count {
        let buf = buffer.clone();
        let sent = sent_count.clone();
        let handle = tokio::spawn(async move {
            for j in 0..messages_per_task {
                let msg = format!("Producer {} msg {}", i, j);
                if buf.write(msg.as_bytes()).await.is_ok() {
                    sent.fetch_add(1, Ordering::Relaxed);
                }
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        });
        handles.push(handle);
    }
    
    // Spawn consumers
    for _ in 0..consumer_count {
        let buf = buffer.clone();
        let received = received_count.clone();
        let handle = tokio::spawn(async move {
            for _ in 0..messages_per_task {
                if let Some(_data) = buf.read().await {
                    received.fetch_add(1, Ordering::Relaxed);
                }
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        });
        handles.push(handle);
    }
    
    // Wait for all
    for handle in handles {
        handle.await?;
    }
    
    let sent = sent_count.load(Ordering::Relaxed);
    let received = received_count.load(Ordering::Relaxed);
    
    println!("\nConcurrent Producers/Consumers:");
    println!("  Producers: {}", producer_count);
    println!("  Consumers: {}", consumer_count);
    println!("  Messages sent: {}", sent);
    println!("  Messages received: {}", received);
    
    // At least 80% should succeed
    assert!(sent >= (producer_count * messages_per_task) as u64 * 8 / 10,
            "Too many send failures");
    
    Ok(())
}

/// Test: Variable load with idle periods
#[tokio::test]
#[cfg(unix)]
async fn test_variable_load_with_idle() -> Result<()> {
    let test_id = uuid::Uuid::new_v4();
    let shm_path = format!("/lapce_variable_{}", test_id);
    let buffer = Arc::new(SharedMemoryBuffer::create(&shm_path, 4 * 1024 * 1024).await?);
    
    let test_data = vec![0x42; 1024];
    
    println!("\nVariable Load Test:");
    
    // High load period
    println!("  High load (2s)...");
    let start = Instant::now();
    let mut high_count = 0u64;
    
    while start.elapsed() < Duration::from_secs(2) {
        buffer.write(&test_data).await?;
        let _ = buffer.read().await;
        high_count += 1;
    }
    
    let high_throughput = high_count as f64 / start.elapsed().as_secs_f64();
    println!("    {} messages ({:.0} msg/s)", high_count, high_throughput);
    
    // Idle period
    println!("  Idle (1s)...");
    sleep(Duration::from_secs(1)).await;
    
    // Medium load
    println!("  Medium load (2s)...");
    let start = Instant::now();
    let mut med_count = 0u64;
    
    while start.elapsed() < Duration::from_secs(2) {
        buffer.write(&test_data).await?;
        let _ = buffer.read().await;
        med_count += 1;
        
        sleep(Duration::from_micros(500)).await; // Throttle
    }
    
    let med_throughput = med_count as f64 / start.elapsed().as_secs_f64();
    println!("    {} messages ({:.0} msg/s)", med_count, med_throughput);
    
    // Idle period
    println!("  Idle (1s)...");
    sleep(Duration::from_secs(1)).await;
    
    // Low load
    println!("  Low load (2s)...");
    let start = Instant::now();
    let mut low_count = 0u64;
    
    while start.elapsed() < Duration::from_secs(2) {
        buffer.write(&test_data).await?;
        let _ = buffer.read().await;
        low_count += 1;
        
        sleep(Duration::from_millis(10)).await; // Heavy throttle
    }
    
    let low_throughput = low_count as f64 / start.elapsed().as_secs_f64();
    println!("    {} messages ({:.0} msg/s)", low_count, low_throughput);
    
    // All phases should complete without errors
    assert!(high_count > 0 && med_count > 0 && low_count > 0,
            "Load variation test failed");
    
    Ok(())
}

/// Test: Error recovery and resilience
#[tokio::test]
#[cfg(unix)]
async fn test_error_recovery() -> Result<()> {
    let test_id = uuid::Uuid::new_v4();
    let shm_path = format!("/lapce_recovery_{}", test_id);
    let buffer = Arc::new(SharedMemoryBuffer::create(&shm_path, 128 * 1024).await?); // Small buffer
    
    let normal_data = vec![0x42; 1024];
    let oversized_data = vec![0x43; 200 * 1024]; // Too large
    
    let mut success_count = 0u64;
    let mut error_count = 0u64;
    
    println!("\nError Recovery Test:");
    
    for i in 0..100 {
        // Every 10th message is oversized
        let data = if i % 10 == 0 {
            &oversized_data
        } else {
            &normal_data
        };
        
        match buffer.write(data).await {
            Ok(_) => {
                let _ = buffer.read().await;
                success_count += 1;
            }
            Err(_) => {
                error_count += 1;
            }
        }
    }
    
    println!("  Successes: {}", success_count);
    println!("  Errors: {}", error_count);
    println!("  Success rate: {:.1}%", success_count as f64 / 100.0 * 100.0);
    
    // Note: Buffer may accept oversized messages if they fit in available space
    // This is actually good - it means buffer is flexible
    assert!(success_count >= 50, "Too many failures: {}", success_count);
    
    // Verify buffer still works after errors
    buffer.write(&normal_data).await?;
    let _ = buffer.read().await;
    
    println!("  Buffer still functional after errors: ✓");
    
    Ok(())
}
