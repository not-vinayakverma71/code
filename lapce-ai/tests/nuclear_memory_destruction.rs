#![cfg(any(target_os = "linux", target_os = "macos"))]
/// Nuclear Test 2: Memory Destruction
/// Exhaust all buffer pools simultaneously
/// Target: Stay under 3MB always

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

// Debug mode: 10x faster
#[cfg(debug_assertions)]
const CONCURRENT_OPERATIONS: usize = 10;

// Release mode: full scale
#[cfg(not(debug_assertions))]
const CONCURRENT_OPERATIONS: usize = 100;

const BUFFER_SIZES: &[usize] = &[
    64,      // Tiny
    1024,    // 1KB
    4096,    // 4KB
    65536,   // 64KB
    1048576, // 1MB
];

#[tokio::test(flavor = "multi_thread")]
async fn nuclear_memory_destruction() {
    println!("\n💥 NUCLEAR TEST 2: MEMORY DESTRUCTION");
    println!("======================================");
    println!("Concurrent operations: {}", CONCURRENT_OPERATIONS);
    println!("Buffer sizes: {:?}", BUFFER_SIZES);
    println!("Target: <3MB memory always\n");
    
    let start_time = Instant::now();
    let peak_memory = Arc::new(AtomicUsize::new(0));
    
    // Start raw SHM echo server with simple len-prefixed framing
    let socket_path = "/tmp/nuc2.sock";
    let listener = Arc::new(SharedMemoryListener::bind(socket_path).unwrap());
    let server_handle = {
        let listener = listener.clone();
        tokio::spawn(async move {
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    tokio::spawn(async move {
                        loop {
                            // Read 4-byte little-endian length
                            let mut len_buf = [0u8; 4];
                            if stream.read_exact(&mut len_buf).await.is_err() { break; }
                            let len = u32::from_le_bytes(len_buf) as usize;
                            if len == 0 || len > 1_048_576 { break; }
                            // Read payload
                            let mut buf = vec![0u8; len];
                            if stream.read_exact(&mut buf).await.is_err() { break; }
                            // Echo back
                            if stream.write_all(&buf).await.is_err() { break; }
                        }
                    });
                }
            }
        })
    };
    
    sleep(Duration::from_millis(100)).await;
    
    // Memory monitoring task
    let monitor_handle = {
        let peak = peak_memory.clone();
        tokio::spawn(async move {
            let mut max_mem = 0.0;
            for _ in 0..100 {
                let mem = get_process_memory_mb();
                if mem > max_mem {
                    max_mem = mem;
                    peak.store((max_mem * 1024.0) as usize, Ordering::Relaxed); // Store in KB
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
    };
    
    // Spawn memory stress tasks
    let mut handles = Vec::new();
    
    for _ in 0..CONCURRENT_OPERATIONS {
        let handle = tokio::spawn(async move {
            let mut stream = SharedMemoryStream::connect(socket_path)
                .await
                .expect("Failed to connect");
            
            // Cycle through all buffer sizes rapidly
            #[cfg(debug_assertions)]
            let iterations = 10;
            #[cfg(not(debug_assertions))]
            let iterations = 100;
            
            for _ in 0..iterations {
                for (idx, &size) in BUFFER_SIZES.iter().enumerate() {
                    // Create message of varying size
                    let message = vec![idx as u8; size];
                    // Send len-prefixed
                    let mut full_msg = Vec::with_capacity(4 + size);
                    full_msg.extend_from_slice(&(size as u32).to_le_bytes());
                    full_msg.extend_from_slice(&message);
                    stream.write_all(&full_msg).await.expect("Write failed");
                    
                    // Read response
                    let mut response = vec![0u8; size];
                    stream.read_exact(&mut response).await.expect("Read failed");
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }
    
    monitor_handle.abort();
    
    // Results
    let total_time = start_time.elapsed();
    let peak_kb = peak_memory.load(Ordering::Relaxed);
    let peak_mb = peak_kb as f64 / 1024.0;
    
    println!("\n📊 RESULTS");
    println!("==========");
    println!("Test duration: {:.2}s", total_time.as_secs_f64());
    println!("Peak memory: {:.2} MB", peak_mb);
    println!("Operations: {} concurrent", CONCURRENT_OPERATIONS);
    
    // Validation
    if peak_mb < 3.0 {
        println!("\n✅ SUCCESS: Peak memory {:.2} MB < 3 MB", peak_mb);
    } else {
        println!("\n❌ FAILED: Peak memory {:.2} MB >= 3 MB", peak_mb);
        panic!("Exceeded memory limit");
    }
    
    server_handle.abort();
}

fn get_process_memory_mb() -> f64 {
    use sysinfo::System;
    
    let mut sys = System::new();
    sys.refresh_processes();
    
    let pid = sysinfo::Pid::from(std::process::id() as usize);
    if let Some(process) = sys.process(pid) {
        process.memory() as f64 / 1024.0 // KB to MB
    } else {
        0.0
    }
}
