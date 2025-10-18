/// Benchmark IPC Server Performance
use lapce_ai_rust::ipc_server::{IpcServer, IpcError};
use lapce_ai_rust::ipc_messages::MessageType;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::Bytes;

async fn run_echo_benchmark(iterations: usize) -> Result<Duration, Box<dyn std::error::Error>> {
    // Create server
    let server = Arc::new(IpcServer::new("/tmp/bench_ipc.sock").await?);
    
    // Register echo handler
    server.register_handler(MessageType::Echo, |data| async move {
        Ok(data) // Simple echo
    });
    
    // Start server in background
    let server_handle = {
        let server = server.clone();
        tokio::spawn(async move {
            let _ = server.serve().await;
        })
    };
    
    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Connect client
    let mut stream = UnixStream::connect("/tmp/bench_ipc.sock").await?;
    
    // Prepare test message
    let test_data = vec![0u8; 100]; // 100 byte payload
    let msg_type = MessageType::Echo.to_bytes();
    let mut message = Vec::new();
    message.extend_from_slice(&msg_type);
    message.extend_from_slice(&test_data);
    
    let msg_len = message.len() as u32;
    
    // Benchmark
    let start = Instant::now();
    
    for _ in 0..iterations {
        // Send message
        stream.write_all(&msg_len.to_le_bytes()).await?;
        stream.write_all(&message).await?;
        
        // Read response
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let response_len = u32::from_le_bytes(len_buf) as usize;
        
        let mut response = vec![0u8; response_len];
        stream.read_exact(&mut response).await?;
    }
    
    let elapsed = start.elapsed();
    
    // Shutdown server
    server.shutdown();
    server_handle.abort();
    
    Ok(elapsed)
}

async fn test_concurrent_connections() -> Result<(), Box<dyn std::error::Error>> {
    let server = Arc::new(IpcServer::new("/tmp/bench_ipc_concurrent.sock").await?);
    
    server.register_handler(MessageType::Echo, |data| async move {
        Ok(data)
    });
    
    let server_handle = {
        let server = server.clone();
        tokio::spawn(async move {
            let _ = server.serve().await;
        })
    };
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    println!("Testing 1000 concurrent connections...");
    let start = Instant::now();
    
    let mut handles = vec![];
    for i in 0..1000 {
        let handle = tokio::spawn(async move {
            match UnixStream::connect("/tmp/bench_ipc_concurrent.sock").await {
                Ok(mut stream) => {
                    // Send one message
                    let msg = format!("client_{}", i).into_bytes();
                    let msg_type = MessageType::Echo.to_bytes();
                    let mut message = Vec::new();
                    message.extend_from_slice(&msg_type);
                    message.extend_from_slice(&msg);
                    
                    let msg_len = message.len() as u32;
                    let _ = stream.write_all(&msg_len.to_le_bytes()).await;
                    let _ = stream.write_all(&message).await;
                    
                    Ok::<(), Box<dyn std::error::Error + Send>>(())
                }
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send>)
            }
        });
        handles.push(handle);
    }
    
    let mut successful = 0;
    for handle in handles {
        if handle.await?.is_ok() {
            successful += 1;
        }
    }
    
    let elapsed = start.elapsed();
    println!("Connected {} / 1000 clients in {:?}", successful, elapsed);
    
    server.shutdown();
    server_handle.abort();
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("IPC Server Benchmark");
    println!("====================");
    
    // Test 1: Throughput
    println!("\n1. THROUGHPUT TEST");
    let iterations = 100_000;
    let elapsed = run_echo_benchmark(iterations).await?;
    let throughput = iterations as f64 / elapsed.as_secs_f64();
    println!("   Messages: {}", iterations);
    println!("   Time: {:?}", elapsed);
    println!("   Throughput: {:.0} msg/sec", throughput);
    println!("   Target: >1,000,000 msg/sec");
    println!("   Result: {}", if throughput > 1_000_000.0 { "✅ PASS" } else { "❌ FAIL" });
    
    // Test 2: Latency
    println!("\n2. LATENCY TEST");
    let single_msg_time = elapsed.as_nanos() / iterations as u128;
    let latency_us = single_msg_time as f64 / 1000.0;
    println!("   Average latency: {:.2} μs", latency_us);
    println!("   Target: <10 μs");
    println!("   Result: {}", if latency_us < 10.0 { "✅ PASS" } else { "❌ FAIL" });
    
    // Test 3: Concurrent connections
    println!("\n3. CONCURRENT CONNECTIONS TEST");
    test_concurrent_connections().await?;
    
    // Test 4: Memory usage
    println!("\n4. MEMORY USAGE");
    let mem_info = std::fs::read_to_string("/proc/self/status")?;
    for line in mem_info.lines() {
        if line.starts_with("VmRSS:") {
            println!("   Current RSS: {}", line.split_whitespace().nth(1).unwrap_or("unknown"));
            break;
        }
    }
    println!("   Target: <3MB");
    
    println!("\n5. COMPARISON WITH NODE.JS");
    println!("   Node.js baseline: ~100,000 msg/sec (typical)");
    println!("   Our throughput: {:.0} msg/sec", throughput);
    println!("   Ratio: {:.1}x", throughput / 100_000.0);
    println!("   Target: 10x");
    println!("   Result: {}", if throughput > 1_000_000.0 { "✅ PASS" } else { "❌ FAIL" });
    
    Ok(())
}
