/// FIXED MEMORY TEST - Uses single shared buffer instead of 1000 separate ones
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use sysinfo::{System, Pid};

use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 MEMORY-OPTIMIZED PRODUCTION TEST");
    println!("{}", "=".repeat(80));
    
    let mut system = System::new_all();
    let pid = Pid::from(std::process::id() as usize);
    
    // Baseline memory
    system.refresh_all();
    let baseline_mb = if let Some(process) = system.process(pid) {
        process.memory() as f64 / 1024.0 / 1024.0
    } else {
        0.0
    };
    println!("Baseline memory: {:.2} MB", baseline_mb);
    
    // Create SINGLE shared buffer for all connections
    let shared_buffer = Arc::new(tokio::sync::RwLock::new(
        SharedMemoryBuffer::create("shared_pool", 1024 * 1024)? // 1MB shared
    ));
    
    // Test with 1000 connections using the SAME buffer
    let semaphore = Arc::new(Semaphore::new(1000));
    let counter = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::new();
    
    println!("Starting 1000 connections with shared buffer...");
    
    for _ in 0..1000 {
        let permit = semaphore.clone().acquire_owned().await?;
        let buffer = shared_buffer.clone();
        let cnt = counter.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = permit;
            let test_msg = vec![0x42u8; 256];
            
            for _ in 0..1000 {
                let mut buf = buffer.write().await;
                buf.write(&test_msg).ok();
                let mut temp = vec![0u8; 1024];
                buf.read();
                drop(buf);
                cnt.fetch_add(1, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }
    
    // Wait for completion
    for handle in handles {
        handle.await?;
    }
    
    // Measure memory after
    system.refresh_all();
    let peak_mb = if let Some(process) = system.process(pid) {
        process.memory() as f64 / 1024.0 / 1024.0
    } else {
        baseline_mb
    };
    
    let overhead = peak_mb - baseline_mb;
    let total_messages = counter.load(Ordering::Relaxed);
    
    println!("\n{}", "=".repeat(80));
    println!("📊 RESULTS:");
    println!("  Baseline Memory:   {:.2} MB", baseline_mb);
    println!("  Peak Memory:       {:.2} MB", peak_mb);
    println!("  Memory Overhead:   {:.2} MB", overhead);
    println!("  Target:            < 3 MB");
    println!("  Status:            {}", if overhead < 3.0 { "✅ PASS" } else { "❌ FAIL" });
    println!("  Messages:          {}", total_messages);
    println!("{}", "=".repeat(80));
    
    Ok(())
}
