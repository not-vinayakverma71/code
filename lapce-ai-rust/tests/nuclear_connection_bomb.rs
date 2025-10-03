/// Nuclear Test 1: Connection Bomb
/// 1000 connections × 5000 messages = 5M messages total
/// Target: >1M msg/sec sustained for 5 minutes

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use lapce_ai_rust::ipc::ipc_server::{IpcServer;
use lapce_ai_rust::shared_memory_complete::SharedMemoryStream;
use bytes::Bytes;

const NUM_CONNECTIONS: usize = 1000;
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
    
    // Start IPC server
    let socket_path = "/tmp/lapce_nuclear_1.sock";
    let server = Arc::new(IpcServer::new(socket_path).await.unwrap());
    
    // Register echo handler
    server.register_handler(0, |data| async move {
        Ok(data) // Echo back
    });
    
    // Start server
    let server_handle = {
        let server = server.clone();
        tokio::spawn(async move {
            server.serve().await.unwrap();
        })
    };
    
    sleep(Duration::from_millis(100)).await;
    
    // Spawn all connections concurrently
    let mut handles = Vec::with_capacity(NUM_CONNECTIONS);
    
    for conn_id in 0..NUM_CONNECTIONS {
        let total_msgs = total_messages.clone();
        let total_bts = total_bytes.clone();
        
        let handle = tokio::spawn(async move {
            // Connect
            let mut stream = SharedMemoryStream::connect(socket_path)
                .await
                .expect("Failed to connect");
            
            // Send messages
            for _msg_id in 0..MESSAGES_PER_CONNECTION {
                let message = vec![0u8; MESSAGE_SIZE];
                
                // Send
                stream.write_all(&message).await.expect("Write failed");
                
                // Receive
                let mut response = vec![0u8; MESSAGE_SIZE];
                stream.read_exact(&mut response).await.expect("Read failed");
                
                // Stats
                total_msgs.fetch_add(1, Ordering::Relaxed);
                total_bts.fetch_add(MESSAGE_SIZE as u64 * 2, Ordering::Relaxed);
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
}
