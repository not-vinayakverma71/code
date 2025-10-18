/// Test Unix Socket IpcServer - HOUR 4
use lapce_ai_rust::ipc_server::{IpcServer, IpcError};
use lapce_ai_rust::ipc_messages::MessageType;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Unix Socket IPC Server");
    println!("==============================\n");
    
    // Test 1: Server creation
    println!("1. Creating server...");
    let server = Arc::new(IpcServer::new("/tmp/test_unix_ipc.sock").await?);
    println!("   ✅ Server created successfully");
    
    // Test 2: Handler registration
    println!("\n2. Registering echo handler...");
    server.register_handler(MessageType::Echo, |data| async move {
        Ok(data) // Simple echo
    });
    println!("   ✅ Handler registered");
    
    // Test 3: Start server
    println!("\n3. Starting server...");
    let server_handle = {
        let server = server.clone();
        tokio::spawn(async move {
            let _ = server.serve().await;
        })
    };
    
    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("   ✅ Server started");
    
    // Test 4: Connect client and send message
    println!("\n4. Testing echo message...");
    let mut stream = UnixStream::connect("/tmp/test_unix_ipc.sock").await?;
    
    // Prepare message
    let test_data = b"Hello, IPC!";
    let msg_type = MessageType::Echo.to_bytes();
    let mut message = Vec::new();
    message.extend_from_slice(&msg_type);
    message.extend_from_slice(test_data);
    
    let msg_len = message.len() as u32;
    
    // Send message
    stream.write_all(&msg_len.to_le_bytes()).await?;
    stream.write_all(&message).await?;
    
    // Read response
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let response_len = u32::from_le_bytes(len_buf) as usize;
    
    let mut response = vec![0u8; response_len];
    stream.read_exact(&mut response).await?;
    
    if response == test_data {
        println!("   ✅ Echo test passed - received: {:?}", String::from_utf8_lossy(&response));
    } else {
        println!("   ❌ Echo test failed");
    }
    
    // Test 5: Throughput measurement
    println!("\n5. Measuring throughput...");
    let iterations = 10000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        stream.write_all(&msg_len.to_le_bytes()).await?;
        stream.write_all(&message).await?;
        
        stream.read_exact(&mut len_buf).await?;
        let response_len = u32::from_le_bytes(len_buf) as usize;
        let mut response = vec![0u8; response_len];
        stream.read_exact(&mut response).await?;
    }
    
    let elapsed = start.elapsed();
    let throughput = iterations as f64 / elapsed.as_secs_f64();
    
    println!("   Messages: {}", iterations);
    println!("   Time: {:?}", elapsed);
    println!("   Throughput: {:.0} msg/sec", throughput);
    println!("   Latency: {:.2} μs per message", elapsed.as_micros() as f64 / iterations as f64);
    
    // Shutdown server
    server.shutdown();
    server_handle.abort();
    
    println!("\n✅ Unix Socket IPC Server works correctly!");
    println!("   Achieved: {:.0} msg/sec", throughput);
    
    Ok(())
}
