#![cfg(any(target_os = "linux", target_os = "macos"))]
/// Nuclear Test 2: Memory Destruction
/// Exhaust all buffer pools simultaneously
/// Target: Stay under 3MB always

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use lapce_ai_rust::ipc::ipc_server::IpcServer;
use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryStream;
use lapce_ai_rust::ipc::binary_codec::MessageType;
use bytes::Bytes;

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
    
    // Start IPC server
    let socket_path = "/tmp/lapce_nuclear_2.sock";
    let server = Arc::new(IpcServer::new(socket_path).await.unwrap());
    
    // Register handlers for different sizes
    // Use MessageType::CompletionRequest for all handlers since we're testing memory, not message types
    for (_idx, &size) in BUFFER_SIZES.iter().enumerate() {
        let handler_size = size;
        server.register_handler(MessageType::CompletionRequest, move |data| async move {
            // Allocate temporary buffer to stress memory
            let mut buffer = vec![0u8; handler_size];
            buffer[0] = data[0]; // Touch memory
            Ok(Bytes::from(buffer))
        });
    }
    
    // Start server
    let server_handle = {
        let server = server.clone();
        tokio::spawn(async move {
            server.serve().await.unwrap();
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
            for _ in 0..100 {
                for (idx, &size) in BUFFER_SIZES.iter().enumerate() {
                    // Create message of varying size
                    let message = vec![idx as u8; size.min(1024)];
                    
                    // Send with message type
                    let mut full_msg = vec![];
                    full_msg.extend_from_slice(&(idx as u32).to_le_bytes());
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
    use sysinfo::{System, Pid};
    
    let mut sys = System::new();
    sys.refresh_processes();
    
    let pid = sysinfo::Pid::from(std::process::id() as usize);
    if let Some(process) = sys.process(pid) {
        process.memory() as f64 / 1024.0 // KB to MB
    } else {
        0.0
    }
}
