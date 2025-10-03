use lapce_ai_rust::ultra_fast_shared_memory::{UltraFastSharedMemory, BatchProcessor};
use std::time::Instant;
use std::sync::Arc;
use tokio::task;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("════════════════════════════════════════════════════════");
    println!("   ULTRA PERFORMANCE TEST - REAL ZERO-COPY");
    println!("════════════════════════════════════════════════════════");
    println!();
    
    // Test 1: Zero-copy SharedMemory
    println!("▶ TEST 1: UltraFastSharedMemory Performance");
    println!("─────────────────────────────────────────");
    test_ultra_fast_shm().await?;
    
    // Test 2: Batch Processing
    println!("\n▶ TEST 2: Batch Processing Performance");
    println!("─────────────────────────────────────────");
    test_batch_processing().await?;
    
    // Test 3: Large Message Throughput
    println!("\n▶ TEST 3: Large Message Throughput");
    println!("─────────────────────────────────────────");
    test_large_messages().await?;
    
    // Test 4: Concurrent Performance
    println!("\n▶ TEST 4: Concurrent Performance");
    println!("─────────────────────────────────────────");
    test_concurrent_performance().await?;
    
    println!("\n════════════════════════════════════════════════════════");
    println!("   PERFORMANCE SUMMARY");
    println!("════════════════════════════════════════════════════════");
    
    Ok(())
}

async fn test_ultra_fast_shm() -> anyhow::Result<()> {
    let mut shm = UltraFastSharedMemory::new("perf_test", 64 * 1024 * 1024)?;
    
    let sizes = vec![(64, 1_000_000), (256, 500_000), (1024, 200_000), (4096, 50_000), (16384, 10_000)];
    
    for (size, iterations) in sizes {
        let data = vec![0xAB; size];
        let mut latencies = Vec::with_capacity(iterations);
        
        let total_start = Instant::now();
        
        for _ in 0..iterations {
            let op_start = Instant::now();
            
            if let Some(buffer) = shm.get_write_buffer(size) {
                buffer.copy_from_slice(&data);
                shm.commit_write(size);
            }
            
            latencies.push(op_start.elapsed());
        }
        
        let total_elapsed = total_start.elapsed();
        let throughput = iterations as f64 / total_elapsed.as_secs_f64();
        let throughput_mb = (size * iterations) as f64 / total_elapsed.as_secs_f64() / (1024.0 * 1024.0);
        
        latencies.sort_unstable();
        let p50 = latencies[latencies.len() / 2].as_nanos() as f64;
        let p99 = latencies[latencies.len() * 99 / 100].as_nanos() as f64;
        
        println!("  Size: {:5}B | Ops: {:7} | P50: {:6.0}ns | P99: {:6.0}ns | Throughput: {:.0} ops/sec | {:.0} MB/s",
                 size, iterations, p50, p99, throughput, throughput_mb);
        
        if throughput > 1_000_000.0 {
            println!("    ✅ Achieved >1M ops/sec!");
        }
        if p50 < 10_000.0 {
            println!("    ✅ Achieved <10μs latency!");
        }
    }
    
    Ok(())
}

async fn test_batch_processing() -> anyhow::Result<()> {
    let shm = UltraFastSharedMemory::new("batch_test", 128 * 1024 * 1024)?;
    let mut processor = BatchProcessor::new(shm, 1000);
    
    let batch_sizes = vec![100, 500, 1000, 5000];
    
    for batch_size in batch_sizes {
        let messages: Vec<Vec<u8>> = (0..batch_size)
            .map(|i| vec![(i % 256) as u8; 256])
            .collect();
        
        let start = Instant::now();
        let sent = processor.send_batch(&messages)?;
        let elapsed = start.elapsed();
        
        let throughput = sent as f64 / elapsed.as_secs_f64();
        let latency_per_msg = elapsed.as_nanos() as f64 / sent as f64;
        
        println!("  Batch size: {:4} | Sent: {:4} | Latency/msg: {:.0}ns | Throughput: {:.0} msg/sec",
                 batch_size, sent, latency_per_msg, throughput);
        
        if throughput > 1_000_000.0 {
            println!("    ✅ Achieved >1M msg/sec in batch mode!");
        }
    }
    
    Ok(())
}

async fn test_large_messages() -> anyhow::Result<()> {
    let mut shm = UltraFastSharedMemory::new("large_test", 256 * 1024 * 1024)?;
    
    let sizes = vec![
        (1024, 10000),        // 1KB
        (16 * 1024, 1000),    // 16KB
        (64 * 1024, 500),     // 64KB
        (256 * 1024, 100),    // 256KB
        (1024 * 1024, 20),    // 1MB
    ];
    
    for (size, iterations) in sizes {
        let data = vec![0xFF; size];
        
        let start = Instant::now();
        let mut successful = 0;
        
        for _ in 0..iterations {
            if let Some(buffer) = shm.get_write_buffer(size) {
                buffer.copy_from_slice(&data);
                shm.commit_write(size);
                successful += 1;
            }
        }
        
        let elapsed = start.elapsed();
        let throughput_mb = (size * successful) as f64 / elapsed.as_secs_f64() / (1024.0 * 1024.0);
        let throughput_ops = successful as f64 / elapsed.as_secs_f64();
        let latency_us = elapsed.as_micros() as f64 / successful as f64;
        
        println!("  Size: {:7} | Count: {:4} | Latency: {:.2}μs | Throughput: {:.0} ops/sec | {:.0} MB/s",
                 format!("{}KB", size / 1024), successful, latency_us, throughput_ops, throughput_mb);
        
        if size <= 1024 && throughput_ops > 1_000_000.0 {
            println!("    ✅ 1KB messages achieve >1M ops/sec!");
        }
    }
    
    Ok(())
}

async fn test_concurrent_performance() -> anyhow::Result<()> {
    let num_tasks = 10;
    let messages_per_task = 100_000;
    
    let start = Instant::now();
    let mut handles = Vec::new();
    
    for task_id in 0..num_tasks {
        let handle = task::spawn(async move {
            let mut shm = UltraFastSharedMemory::new(
                &format!("concurrent_{}", task_id), 
                32 * 1024 * 1024
            ).unwrap();
            
            let data = vec![task_id as u8; 256];
            let mut count = 0;
            
            for _ in 0..messages_per_task {
                if let Some(buffer) = shm.get_write_buffer(256) {
                    buffer.copy_from_slice(&data);
                    shm.commit_write(256);
                    count += 1;
                }
            }
            
            count
        });
        handles.push(handle);
    }
    
    let mut total_processed = 0;
    for handle in handles {
        total_processed += handle.await?;
    }
    
    let elapsed = start.elapsed();
    let throughput = total_processed as f64 / elapsed.as_secs_f64();
    
    println!("  Tasks: {} | Messages/task: {} | Total: {}", 
             num_tasks, messages_per_task, total_processed);
    println!("  Total time: {:.2}s | Throughput: {:.0} msg/sec", 
             elapsed.as_secs_f64(), throughput);
    
    if throughput > 1_000_000.0 {
        println!("  ✅ Achieved >1M msg/sec with {} concurrent tasks!", num_tasks);
    } else {
        println!("  ❌ Only {:.0} msg/sec (target: >1M)", throughput);
    }
    
    Ok(())
}
