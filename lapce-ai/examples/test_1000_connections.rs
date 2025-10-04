/// Test 1000+ Concurrent Connections - HOUR 7
use lapce_ai_rust::ipc_server::{IpcServer, IpcError};
use lapce_ai_rust::ipc_messages::MessageType;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing 1000+ Concurrent Connections");
    println!("=====================================\n");
    
    // Create server
    let server = Arc::new(IpcServer::new("/tmp/test_1000_conn.sock").await?);
    let connected_count = Arc::new(AtomicUsize::new(0));
    let message_count = Arc::new(AtomicUsize::new(0));
    
    // Register handler
    let msg_counter = message_count.clone();
    server.register_handler(MessageType::Echo, move |data| {
        let counter = msg_counter.clone();
        async move {
            counter.fetch_add(1, Ordering::Relaxed);
            Ok(data)
        }
    });
    
    // Start server
    let server_handle = {
        let server = server.clone();
        tokio::spawn(async move {
            let _ = server.serve().await;
        })
    };
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Test different connection counts
    let test_counts = vec![10, 100, 500, 1000, 1500];
    
    for &count in &test_counts {
        println!("Testing {} concurrent connections...", count);
        let start = Instant::now();
        connected_count.store(0, Ordering::Relaxed);
        message_count.store(0, Ordering::Relaxed);
        
        let mut handles = vec![];
        
        for i in 0..count {
            let connected = connected_count.clone();
            let handle = tokio::spawn(async move {
                match UnixStream::connect("/tmp/test_1000_conn.sock").await {
                    Ok(mut stream) => {
                        connected.fetch_add(1, Ordering::Relaxed);
                        
                        // Send a test message
                        let test_data = format!("client_{}", i).into_bytes();
                        let msg_type = MessageType::Echo.to_bytes();
                        let mut message = Vec::new();
                        message.extend_from_slice(&msg_type);
                        message.extend_from_slice(&test_data);
                        
                        let msg_len = message.len() as u32;
                        
                        // Try to send message
                        if stream.write_all(&msg_len.to_le_bytes()).await.is_ok() {
                            if stream.write_all(&message).await.is_ok() {
                                // Try to read response
                                let mut len_buf = [0u8; 4];
                                if stream.read_exact(&mut len_buf).await.is_ok() {
                                    let response_len = u32::from_le_bytes(len_buf) as usize;
                                    let mut response = vec![0u8; response_len];
                                    let _ = stream.read_exact(&mut response).await;
                                }
                            }
                        }
                        
                        Ok::<(), Box<dyn std::error::Error + Send>>(())
                    }
                    Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send>)
                }
            });
            handles.push(handle);
        }
        
        // Wait for all clients to complete
        let mut successful = 0;
        for handle in handles {
            if handle.await?.is_ok() {
                successful += 1;
            }
        }
        
        let elapsed = start.elapsed();
        let connected = connected_count.load(Ordering::Relaxed);
        let messages = message_count.load(Ordering::Relaxed);
        
        println!("  Connected: {}/{}", connected, count);
        println!("  Successful: {}/{}", successful, count);
        println!("  Messages processed: {}", messages);
        println!("  Time: {:?}", elapsed);
        println!("  Connection rate: {:.0} conn/sec", connected as f64 / elapsed.as_secs_f64());
        
        if connected == count {
            println!("  Status: ✅ PASS");
        } else if connected >= count * 95 / 100 {
            println!("  Status: ⚠️ PARTIAL ({:.0}% connected)", (connected as f64 / count as f64) * 100.0);
        } else {
            println!("  Status: ❌ FAIL");
        }
        
        println!();
        
        // Give server time to clean up
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    // Shutdown server
    server.shutdown();
    server_handle.abort();
    
    println!("\n=== REQUIREMENT #4: 1000+ CONCURRENT CONNECTIONS ===");
    println!("Target: Support 1000+ concurrent connections");
    println!("Result: Tested up to 1500 connections");
    println!("Status: Check results above");
    
    Ok(())
}
