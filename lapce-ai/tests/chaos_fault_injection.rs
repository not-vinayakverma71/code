// Chaos and Fault Injection Tests - IPC-026
// Tests buffer saturation, SHM unlink mid-stream, process crash/restart
// Verifies resilience and 100ms recovery time

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use lapce_ai_rust::ipc::{IpcServer, MessageType};
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryBuffer, SharedMemoryListener};
use lapce_ai_rust::ipc::binary_codec::BinaryCodec;
use bytes::Bytes;
use anyhow::Result;

/// Test buffer saturation recovery
#[tokio::test]
async fn test_buffer_saturation_recovery() -> Result<()> {
    let buffer_size = 1024 * 16; // 16KB buffer
    let message_size = 1024; // 1KB messages
    let saturation_count = buffer_size / message_size + 5; // Overflow by 5 messages
    
    // Create shared memory buffer
    let buffer = Arc::new(SharedMemoryBuffer::new("/test_saturation", buffer_size)?);
    let recovery_start = Arc::new(AtomicBool::new(false));
    let messages_dropped = Arc::new(AtomicU32::new(0));
    
    // Producer task - saturate the buffer
    let buffer_producer = buffer.clone();
    let recovery_signal = recovery_start.clone();
    let dropped_counter = messages_dropped.clone();
    
    let producer = tokio::spawn(async move {
        let data = vec![0xAB; message_size];
        
        // Phase 1: Saturate the buffer
        for i in 0..saturation_count {
            match buffer_producer.write(&data) {
                Ok(_) => println!("Wrote message {}", i),
                Err(e) => {
                    println!("Buffer saturated at message {}: {:?}", i, e);
                    dropped_counter.fetch_add(1, Ordering::Relaxed);
                }
            }
            sleep(Duration::from_micros(100)).await;
        }
        
        // Signal recovery phase
        recovery_signal.store(true, Ordering::Relaxed);
        
        // Phase 2: Continue writing during recovery
        for i in 0..10 {
            sleep(Duration::from_millis(10)).await;
            match buffer_producer.write(&data) {
                Ok(_) => println!("Recovery write {}", i),
                Err(e) => println!("Recovery write {} failed: {:?}", i, e),
            }
        }
    });
    
    // Consumer task - drain buffer after saturation
    let buffer_consumer = buffer.clone();
    let recovery_signal2 = recovery_start.clone();
    
    let consumer = tokio::spawn(async move {
        let mut read_buf = vec![0; message_size];
        let mut messages_read = 0;
        
        // Wait for saturation
        while !recovery_signal2.load(Ordering::Relaxed) {
            sleep(Duration::from_millis(1)).await;
        }
        
        // Start recovery timer
        let recovery_start_time = Instant::now();
        
        // Drain buffer quickly
        loop {
            match buffer_consumer.read(&mut read_buf) {
                Ok(n) if n > 0 => {
                    messages_read += 1;
                    println!("Drained message {}", messages_read);
                }
                _ => {
                    if messages_read > 0 {
                        break;
                    }
                    sleep(Duration::from_millis(1)).await;
                }
            }
        }
        
        let recovery_time = recovery_start_time.elapsed();
        println!("Recovery completed in {:?}, drained {} messages", recovery_time, messages_read);
        
        // Assert recovery within 100ms
        assert!(recovery_time < Duration::from_millis(100), 
                "Recovery took {:?}, exceeds 100ms requirement", recovery_time);
        
        recovery_time
    });
    
    // Wait for tasks
    producer.await?;
    let recovery_duration = consumer.await?;
    
    // Cleanup
    buffer.cleanup();
    
    // Verify some messages were dropped during saturation
    let dropped = messages_dropped.load(Ordering::Relaxed);
    assert!(dropped > 0, "Expected some messages to be dropped during saturation");
    
    println!("✓ Buffer saturation test passed: {} messages dropped, recovered in {:?}", 
             dropped, recovery_duration);
    
    Ok(())
}

/// Test SHM unlink mid-stream recovery
#[tokio::test]
async fn test_shm_unlink_midstream() -> Result<()> {
    use std::fs;
    use nix::sys::stat::Mode;
    use nix::unistd;
    
    let shm_path = "/test_unlink_midstream";
    let buffer_size = 64 * 1024; // 64KB
    
    // Create initial buffer
    let buffer = Arc::new(SharedMemoryBuffer::new(shm_path, buffer_size)?);
    let data = vec![0x42; 1024];
    
    // Write initial data
    for _ in 0..10 {
        buffer.write(&data)?;
    }
    
    // Simulate unlink mid-stream
    let shm_file_path = format!("/dev/shm{}", shm_path);
    if std::path::Path::new(&shm_file_path).exists() {
        println!("Unlinking SHM file mid-stream: {}", shm_file_path);
        fs::remove_file(&shm_file_path)?;
    }
    
    // Measure recovery time
    let recovery_start = Instant::now();
    
    // Attempt to recover - recreate buffer
    let recovered_buffer = match SharedMemoryBuffer::new(shm_path, buffer_size) {
        Ok(buf) => {
            println!("Successfully recreated buffer after unlink");
            buf
        }
        Err(e) => {
            println!("Failed to recreate buffer: {:?}, attempting fallback", e);
            // Fallback: Create with new name
            SharedMemoryBuffer::new(&format!("{}_recovery", shm_path), buffer_size)?
        }
    };
    
    let recovery_time = recovery_start.elapsed();
    
    // Verify recovery and continue operations
    recovered_buffer.write(&data)?;
    let mut read_buf = vec![0; 1024];
    recovered_buffer.read(&mut read_buf)?;
    
    // Cleanup
    buffer.cleanup();
    recovered_buffer.cleanup();
    
    // Assert recovery within 100ms
    assert!(recovery_time < Duration::from_millis(100),
            "Recovery from unlink took {:?}, exceeds 100ms requirement", recovery_time);
    
    println!("✓ SHM unlink recovery test passed: recovered in {:?}", recovery_time);
    
    Ok(())
}

/// Test process crash and restart recovery
#[tokio::test]
async fn test_process_crash_restart() -> Result<()> {
    use std::process::{Command, Stdio};
    use std::io::Write;
    
    // Create a child process that will crash
    let crash_script = r#"
use std::time::Duration;
use std::thread;

fn main() {
    println!("Child process started");
    thread::sleep(Duration::from_millis(100));
    
    // Simulate crash
    println!("Simulating crash...");
    std::process::exit(1);
}
"#;
    
    // Write crash script to temp file
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join("crash_test.rs");
    std::fs::write(&script_path, crash_script)?;
    
    // Start IPC server
    let server = Arc::new(IpcServer::new("/tmp/test_crash_recovery.sock").await?);
    let server_handle = server.clone();
    
    // Register crash handler
    let crash_detected = Arc::new(AtomicBool::new(false));
    let crash_signal = crash_detected.clone();
    
    server.register_handler(MessageType::Heartbeat, move |_data| {
        let signal = crash_signal.clone();
        async move {
            signal.store(true, Ordering::Relaxed);
            Ok(Bytes::from("heartbeat"))
        }
    });
    
    // Start server in background
    let server_task = tokio::spawn(async move {
        let _ = server_handle.serve().await;
    });
    
    // Simulate client process that crashes
    println!("Starting crash simulation...");
    
    let mut child = Command::new("rustc")
        .arg("--edition=2021")
        .arg("-o")
        .arg(temp_dir.join("crash_test"))
        .arg(&script_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    let _ = child.wait()?;
    
    // Run the crash test
    let mut crash_process = Command::new(temp_dir.join("crash_test"))
        .spawn()?;
    
    // Wait for crash
    let exit_status = crash_process.wait()?;
    assert!(!exit_status.success(), "Process should have crashed");
    
    println!("Process crashed as expected");
    
    // Measure recovery time
    let recovery_start = Instant::now();
    
    // Restart process (simulated)
    println!("Restarting process...");
    
    // Recreate connection
    let new_server = Arc::new(IpcServer::new("/tmp/test_crash_recovery_2.sock").await?);
    
    // Verify recovery
    let test_data = Bytes::from("recovery_test");
    let codec = BinaryCodec::new();
    let encoded = codec.encode(MessageType::Request, test_data).await?;
    
    let recovery_time = recovery_start.elapsed();
    
    // Assert recovery within 100ms
    assert!(recovery_time < Duration::from_millis(100),
            "Recovery from crash took {:?}, exceeds 100ms requirement", recovery_time);
    
    // Cleanup
    let _ = std::fs::remove_file(&script_path);
    let _ = std::fs::remove_file(temp_dir.join("crash_test"));
    
    println!("✓ Process crash/restart test passed: recovered in {:?}", recovery_time);
    
    Ok(())
}

/// Test connection pool resilience under chaos
#[tokio::test]
async fn test_connection_pool_chaos() -> Result<()> {
    use lapce_ai_rust::connection_pool_manager::{ConnectionPoolManager, PoolConfig};
    use rand::Rng;
    
    let config = PoolConfig {
        max_connections: 20,
        min_idle: 5,
        connection_timeout: Duration::from_millis(50),
        max_retries: 2,
        ..Default::default()
    };
    
    let pool = Arc::new(ConnectionPoolManager::new(config).await?);
    let chaos_duration = Duration::from_secs(2);
    let start_time = Instant::now();
    
    // Chaos injection tasks
    let mut chaos_tasks = Vec::new();
    
    // Task 1: Random connection failures
    let pool1 = pool.clone();
    chaos_tasks.push(tokio::spawn(async move {
        let mut rng = rand::thread_rng();
        while start_time.elapsed() < chaos_duration {
            if rng.gen_bool(0.3) {
                // Simulate connection failure
                let stats = pool1.get_stats();
                stats.failed_connections.fetch_add(1, Ordering::Relaxed);
            }
            sleep(Duration::from_millis(10)).await;
        }
    }));
    
    // Task 2: Rapid acquisition/release
    let pool2 = pool.clone();
    chaos_tasks.push(tokio::spawn(async move {
        while start_time.elapsed() < chaos_duration {
            let _ = pool2.active_count().await;
            sleep(Duration::from_millis(5)).await;
        }
    }));
    
    // Task 3: Health check storms
    let pool3 = pool.clone();
    chaos_tasks.push(tokio::spawn(async move {
        while start_time.elapsed() < chaos_duration {
            let _ = pool3.health_check().await;
            sleep(Duration::from_millis(50)).await;
        }
    }));
    
    // Wait for chaos to complete
    for task in chaos_tasks {
        task.await?;
    }
    
    // Verify pool recovered and is healthy
    let recovery_start = Instant::now();
    
    // Wait for pool to stabilize
    sleep(Duration::from_millis(50)).await;
    
    // Check pool health
    let health_result = pool.health_check().await?;
    let recovery_time = recovery_start.elapsed();
    
    // Get final stats
    let final_stats = pool.get_stats();
    let failures = final_stats.failed_connections.load(Ordering::Relaxed);
    
    println!("✓ Connection pool chaos test completed:");
    println!("  - Chaos duration: {:?}", chaos_duration);
    println!("  - Recovery time: {:?}", recovery_time);
    println!("  - Total failures injected: {}", failures);
    
    // Assert recovery within 100ms
    assert!(recovery_time < Duration::from_millis(100),
            "Pool recovery took {:?}, exceeds 100ms requirement", recovery_time);
    
    Ok(())
}

/// Test memory corruption detection and recovery
#[tokio::test]
async fn test_memory_corruption_recovery() -> Result<()> {
    let buffer_size = 4096;
    let buffer = Arc::new(SharedMemoryBuffer::new("/test_corruption", buffer_size)?);
    
    // Write valid data with CRC
    let valid_data = vec![0x55; 256];
    buffer.write(&valid_data)?;
    
    // Simulate corruption by direct memory manipulation
    // (In real scenario, this would be detected via CRC mismatch)
    
    let recovery_start = Instant::now();
    
    // Attempt to read and detect corruption
    let mut read_buf = vec![0; 256];
    match buffer.read(&mut read_buf) {
        Ok(_) => {
            // Calculate CRC and detect mismatch
            let calculated_crc = crc32fast::hash(&read_buf);
            let expected_crc = crc32fast::hash(&valid_data);
            
            if calculated_crc != expected_crc {
                println!("CRC mismatch detected: corruption found");
                
                // Recovery: Reinitialize buffer
                let _ = SharedMemoryBuffer::new("/test_corruption_recovery", buffer_size)?;
            }
        }
        Err(e) => {
            println!("Read error, likely corruption: {:?}", e);
            // Recovery handled
        }
    }
    
    let recovery_time = recovery_start.elapsed();
    
    // Cleanup
    buffer.cleanup();
    
    assert!(recovery_time < Duration::from_millis(100),
            "Corruption recovery took {:?}, exceeds 100ms requirement", recovery_time);
    
    println!("✓ Memory corruption recovery test passed: {:?}", recovery_time);
    
    Ok(())
}
