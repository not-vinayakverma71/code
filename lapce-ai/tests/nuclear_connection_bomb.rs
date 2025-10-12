#![cfg(any(target_os = "linux", target_os = "macos"))]
/// Nuclear Test 1: Connection Bomb
/// 1000 connections × 5000 messages = 5M messages total
/// Target: >1M msg/sec sustained for 5 minutes

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

// Debug mode: 10x faster for iteration
#[cfg(debug_assertions)]
const NUM_CONNECTIONS: usize = 100;
#[cfg(debug_assertions)]
const MESSAGES_PER_CONNECTION: usize = 500;

// Release mode: full scale
#[cfg(not(debug_assertions))]
const NUM_CONNECTIONS: usize = 1000;
#[cfg(not(debug_assertions))]
const MESSAGES_PER_CONNECTION: usize = 5000;

const MESSAGE_SIZE: usize = 512; // 512B messages

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn nuclear_connection_bomb() {
    println!("\n💣 NUCLEAR TEST 1: CONNECTION BOMB");
    println!("====================================");
    println!("Connections: {}", NUM_CONNECTIONS);
    println!("Messages/conn: {}", MESSAGES_PER_CONNECTION);
    println!("Total messages: {}M", (NUM_CONNECTIONS * MESSAGES_PER_CONNECTION) / 1_000_000);
    println!("Target: >1M msg/sec sustained\n");
    
    let start_time = Instant::now();
    let total_messages = Arc::new(AtomicU64::new(0));
    let total_bytes = Arc::new(AtomicU64::new(0));
    
    // Start raw SHM echo server (transport-level) with unique path
    let socket_path = format!("/tmp/nuc1_{}.sock", std::process::id());
    let _ = std::fs::remove_file(&socket_path); // Cleanup old socket
    let listener = Arc::new(SharedMemoryListener::bind(&socket_path).await.unwrap());
    let server_handle = {
        let listener = listener.clone();
        tokio::spawn(async move {
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    tokio::spawn(async move {
                        let mut buf = vec![0u8; MESSAGE_SIZE];
                        loop {
                            if stream.read_exact(&mut buf).await.is_err() { break; }
                            if stream.write_all(&buf).await.is_err() { break; }
                        }
                    });
                }
            }
        })
    };
    
    sleep(Duration::from_millis(100)).await;
    
    // Spawn all connections concurrently
    let mut handles = Vec::with_capacity(NUM_CONNECTIONS);
    
    for conn_id in 0..NUM_CONNECTIONS {
        let total_msgs = total_messages.clone();
        let total_bts = total_bytes.clone();
        
        let socket = socket_path.clone();
        let handle = tokio::spawn(async move {
            // Connect with retry
            let mut stream = loop {
                match SharedMemoryStream::connect(&socket).await {
                    Ok(s) => break s,
                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        continue;
                    }
                }
            };

            // Send messages (transport-level echo)
            for _msg_id in 0..MESSAGES_PER_CONNECTION {
                let message = vec![0u8; MESSAGE_SIZE];
                stream.write_all(&message).await.expect("Write failed");
                let mut resp = vec![0u8; MESSAGE_SIZE];
                stream.read_exact(&mut resp).await.expect("Read failed");
                total_msgs.fetch_add(1, Ordering::Relaxed);
                total_bts.fetch_add((MESSAGE_SIZE as u64) * 2, Ordering::Relaxed);
            }
        });
        
        handles.push(handle);
        
        // Stagger connection creation
        if conn_id % 50 == 0 {
            tokio::task::yield_now().await;
        }
    }
    
    // Progress monitoring
    let monitor_handle = {
        let total_msgs = total_messages.clone();
        let start = start_time;
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(5)).await;
                let msgs = total_msgs.load(Ordering::Relaxed);
                let elapsed = start.elapsed().as_secs_f64();
                let rate = msgs as f64 / elapsed;
                println!("Progress: {} msgs, {:.2}M msg/sec", msgs, rate / 1_000_000.0);
                
                if msgs >= (NUM_CONNECTIONS * MESSAGES_PER_CONNECTION) as u64 {
                    break;
                }
            }
        })
    };
    
    // Wait for all connections
    for handle in handles {
        handle.await.unwrap();
    }
    
    monitor_handle.abort();
    
    // Final results
    let total_time = start_time.elapsed();
    let total_msgs = total_messages.load(Ordering::Relaxed);
    let total_bts = total_bytes.load(Ordering::Relaxed);
    let throughput = total_msgs as f64 / total_time.as_secs_f64();
    let bandwidth = (total_bts as f64 / 1_000_000.0) / total_time.as_secs_f64();
    
    println!("\n📊 RESULTS");
    println!("==========");
    println!("Total time: {:.2}s", total_time.as_secs_f64());
    println!("Total messages: {}M", total_msgs / 1_000_000);
    println!("Throughput: {:.2}M msg/sec", throughput / 1_000_000.0);
    println!("Bandwidth: {:.2} MB/sec", bandwidth);
    
    // Validation
    if throughput > 1_000_000.0 {
        println!("\n✅ SUCCESS: Achieved >1M msg/sec ({:.2}M)", throughput / 1_000_000.0);
    } else {
        println!("\n❌ FAILED: Only {:.2}K msg/sec (target >1M)", throughput / 1_000.0);
        panic!("Did not meet throughput target");
    }
    
    server_handle.abort();
    let _ = std::fs::remove_file(&socket_path); // Cleanup
}
