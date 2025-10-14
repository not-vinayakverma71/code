/// Memory Load Validation Tests
/// Validates IPC system memory usage under sustained load

use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::Result;

#[cfg(unix)]
use lapce_ai_rust::ipc::SharedMemoryBuffer;

/// Get current process RSS (Resident Set Size) in bytes
#[cfg(target_os = "linux")]
fn get_rss_bytes() -> Result<usize> {
    let contents = std::fs::read_to_string("/proc/self/status")?;
    for line in contents.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb: usize = parts[1].parse()?;
                return Ok(kb * 1024);
            }
        }
    }
    anyhow::bail!("Failed to find VmRSS in /proc/self/status")
}

#[cfg(target_os = "macos")]
fn get_rss_bytes() -> Result<usize> {
    use std::process::Command;
    let output = Command::new("ps")
        .args(&["-o", "rss=", "-p"])
        .arg(std::process::id().to_string())
        .output()?;
    
    let rss_kb = String::from_utf8(output.stdout)?
        .trim()
        .parse::<usize>()?;
    
    Ok(rss_kb * 1024)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn get_rss_bytes() -> Result<usize> {
    anyhow::bail!("RSS measurement not supported on this platform")
}

/// Test: Baseline memory usage
#[tokio::test]
#[cfg(unix)]
async fn test_baseline_memory() -> Result<()> {
    // Measure initial RSS
    let initial_rss = get_rss_bytes()?;
    
    println!("Initial RSS: {} MB", initial_rss / 1024 / 1024);
    
    // Create a single buffer
    let test_id = uuid::Uuid::new_v4();
    let shm_path = format!("/lapce_memory_baseline_{}", test_id);
    let _buffer = SharedMemoryBuffer::create(&shm_path, 1024 * 1024).await?;
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let after_create_rss = get_rss_bytes()?;
    let delta = after_create_rss.saturating_sub(initial_rss);
    
    println!("After 1 buffer: {} MB (delta: {} MB)", 
             after_create_rss / 1024 / 1024,
             delta / 1024 / 1024);
    
    // Baseline should be < 5MB
    assert!(delta < 5 * 1024 * 1024, 
            "Baseline memory too high: {} MB", delta / 1024 / 1024);
    
    Ok(())
}

/// Test: Memory usage with 10 buffers
#[tokio::test]
#[cfg(unix)]
async fn test_multiple_buffers_memory() -> Result<()> {
    let initial_rss = get_rss_bytes()?;
    
    let mut buffers = Vec::new();
    
    // Create 10 buffers
    for i in 0..10 {
        let test_id = uuid::Uuid::new_v4();
        let shm_path = format!("/lapce_multi_{}_{}", i, test_id);
        let buffer = SharedMemoryBuffer::create(&shm_path, 1024 * 1024).await?;
        buffers.push(buffer);
    }
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let after_rss = get_rss_bytes()?;
    let delta = after_rss.saturating_sub(initial_rss);
    
    println!("10 buffers RSS delta: {} MB", delta / 1024 / 1024);
    
    // 10x 1MB buffers should use < 15MB total
    assert!(delta < 15 * 1024 * 1024,
            "Memory usage too high for 10 buffers: {} MB", delta / 1024 / 1024);
    
    Ok(())
}

/// Test: Memory stability under sustained load (1 minute)
#[tokio::test]
#[cfg(unix)]
async fn test_sustained_load_memory() -> Result<()> {
    let initial_rss = get_rss_bytes()?;
    println!("Initial RSS: {} MB", initial_rss / 1024 / 1024);
    
    let test_id = uuid::Uuid::new_v4();
    let shm_path = format!("/lapce_sustained_{}", test_id);
    let buffer = Arc::new(SharedMemoryBuffer::create(&shm_path, 4 * 1024 * 1024).await?);
    
    let test_data = vec![0x42u8; 1024]; // 1KB messages
    let duration = Duration::from_secs(10);  // 10 seconds for CI
    let start = Instant::now();
    
    let mut message_count = 0u64;
    let mut samples = Vec::new();
    
    while start.elapsed() < duration {
        // Send/receive 100 messages
        for _ in 0..100 {
            buffer.write(&test_data).await?;
            let _ = buffer.read().await;
            message_count += 1;
        }
        
        // Sample RSS
        let rss = get_rss_bytes()?;
        samples.push(rss);
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    let final_rss = get_rss_bytes()?;
    let max_rss = samples.iter().max().copied().unwrap_or(final_rss);
    let min_rss = samples.iter().min().copied().unwrap_or(initial_rss);
    let avg_rss = samples.iter().sum::<usize>() / samples.len();
    
    let growth = final_rss.saturating_sub(initial_rss);
    let variance = max_rss.saturating_sub(min_rss);
    
    println!("\nSustained Load Results ({} seconds):", duration.as_secs());
    println!("  Messages: {}", message_count);
    println!("  Initial RSS: {} MB", initial_rss / 1024 / 1024);
    println!("  Final RSS: {} MB", final_rss / 1024 / 1024);
    println!("  Average RSS: {} MB", avg_rss / 1024 / 1024);
    println!("  Max RSS: {} MB", max_rss / 1024 / 1024);
    println!("  Memory growth: {} MB", growth / 1024 / 1024);
    println!("  RSS variance: {} MB", variance / 1024 / 1024);
    
    // Memory growth should be < 10MB
    assert!(growth < 10 * 1024 * 1024,
            "Memory growth too high: {} MB", growth / 1024 / 1024);
    
    // RSS should not vary by more than 20MB
    assert!(variance < 20 * 1024 * 1024,
            "RSS variance too high: {} MB", variance / 1024 / 1024);
    
    Ok(())
}

/// Test: Memory leak detection with repeated create/destroy
#[tokio::test]
#[cfg(unix)]
async fn test_no_memory_leak_cycles() -> Result<()> {
    let initial_rss = get_rss_bytes()?;
    println!("Initial RSS: {} MB", initial_rss / 1024 / 1024);
    
    // Create and destroy 50 buffers
    for i in 0..50 {
        let test_id = uuid::Uuid::new_v4();
        let shm_path = format!("/lapce_leak_{}_{}", i, test_id);
        
        let buffer = SharedMemoryBuffer::create(&shm_path, 1024 * 1024).await?;
        
        // Use it briefly
        buffer.write(b"test").await?;
        let _ = buffer.read().await;
        
        drop(buffer);
        
        // Sample every 10 iterations
        if i % 10 == 0 {
            let current_rss = get_rss_bytes()?;
            println!("After {} cycles: {} MB", i, current_rss / 1024 / 1024);
        }
    }
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    let final_rss = get_rss_bytes()?;
    let growth = final_rss.saturating_sub(initial_rss);
    
    println!("Final RSS: {} MB", final_rss / 1024 / 1024);
    println!("Total growth: {} MB", growth / 1024 / 1024);
    
    // Growth should be < 5MB for 50 cycles
    assert!(growth < 5 * 1024 * 1024,
            "Potential memory leak: {} MB growth", growth / 1024 / 1024);
    
    Ok(())
}

/// Test: Large message memory handling
#[tokio::test]
#[cfg(unix)]
async fn test_large_message_memory() -> Result<()> {
    let initial_rss = get_rss_bytes()?;
    
    let test_id = uuid::Uuid::new_v4();
    let shm_path = format!("/lapce_large_{}", test_id);
    let buffer = SharedMemoryBuffer::create(&shm_path, 8 * 1024 * 1024).await?;
    
    // Send 10 x 500KB messages
    let large_data = vec![0x42u8; 500 * 1024];
    
    for _ in 0..10 {
        buffer.write(&large_data).await?;
        let _ = buffer.read().await;
    }
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let final_rss = get_rss_bytes()?;
    let growth = final_rss.saturating_sub(initial_rss);
    
    println!("Large message test - RSS growth: {} MB", growth / 1024 / 1024);
    
    // Should not grow by more than 15MB
    assert!(growth < 15 * 1024 * 1024,
            "Memory growth too high for large messages: {} MB", growth / 1024 / 1024);
    
    Ok(())
}

/// Test: Concurrent access memory stability
#[tokio::test]
#[cfg(unix)]
async fn test_concurrent_memory() -> Result<()> {
    let initial_rss = get_rss_bytes()?;
    
    let test_id = uuid::Uuid::new_v4();
    let shm_path = format!("/lapce_concurrent_{}", test_id);
    let buffer = Arc::new(SharedMemoryBuffer::create(&shm_path, 8 * 1024 * 1024).await?);
    
    let mut handles = vec![];
    
    // Spawn 20 concurrent tasks
    for i in 0..20 {
        let buf = buffer.clone();
        let handle = tokio::spawn(async move {
            for j in 0..50 {
                let msg = format!("Task {} msg {}", i, j);
                let _ = buf.write(msg.as_bytes()).await;
                let _ = buf.read().await;
            }
        });
        handles.push(handle);
    }
    
    // Wait for all
    for handle in handles {
        handle.await?;
    }
    
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    let final_rss = get_rss_bytes()?;
    let growth = final_rss.saturating_sub(initial_rss);
    
    println!("Concurrent test - RSS growth: {} MB", growth / 1024 / 1024);
    
    // Should not grow by more than 20MB
    assert!(growth < 20 * 1024 * 1024,
            "Memory growth too high under concurrency: {} MB", growth / 1024 / 1024);
    
    Ok(())
}
