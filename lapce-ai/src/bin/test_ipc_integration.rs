/// Cross-process IPC Integration Tests
/// Tests handshake, round-trip, backpressure, and concurrent connections

use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use lapce_ai_rust::ipc::{IpcServer, IpcError, MessageType};
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};
use bytes::Bytes;
use tokio::time::timeout;

const TEST_SOCKET_PATH: &str = "test_ipc_integration";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IPC Cross-Process Integration Test ===\n");
    
    // Test 1: Handshake
    test_handshake().await?;
    
    // Test 2: Round-trip messaging
    test_round_trip().await?;
    
    // Test 3: Backpressure
    test_backpressure().await?;
    
    // Test 4: Concurrent connections
    test_concurrent_connections().await?;
    
    println!("\n✅ All integration tests passed!");
    Ok(())
}

async fn test_handshake() -> Result<(), Box<dyn std::error::Error>> {
    println!("Test 1: Client-Server Handshake");
    
    // Start server in background
    let server_handle = tokio::spawn(async move {
        let mut listener = SharedMemoryListener::bind(TEST_SOCKET_PATH)?;
        let (stream, _) = listener.accept().await?;
        Ok::<_, Box<dyn std::error::Error>>(stream)
    });
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Client connects
    let start = Instant::now();
    let client = SharedMemoryStream::connect(TEST_SOCKET_PATH).await?;
    let handshake_time = start.elapsed();
    
    // Wait for server to accept
    let _server_stream = timeout(Duration::from_secs(2), server_handle).await??;
    
    println!("  ✓ Handshake completed in {:?}", handshake_time);
    assert!(handshake_time < Duration::from_secs(1), "Handshake too slow");
    
    Ok(())
}

async fn test_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nTest 2: Round-Trip Messaging");
    
    // Start echo server
    let server_handle = tokio::spawn(async move {
        let mut listener = SharedMemoryListener::bind("test_roundtrip")?;
        let (mut stream, _) = listener.accept().await?;
        
        // Echo back messages
        for _ in 0..10 {
            let mut buf = vec![0u8; 1024];
            if let Ok(_) = stream.read_exact(&mut buf).await {
                // Echo back
                stream.write_all(&buf).await?;
            }
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Client sends and receives
    let mut client = SharedMemoryStream::connect("test_roundtrip").await?;
    
    let mut latencies = Vec::new();
    for i in 0..10 {
        let msg = format!("Message {}", i);
        let msg_bytes = msg.as_bytes();
        let mut padded = vec![0u8; 1024];
        padded[..msg_bytes.len()].copy_from_slice(msg_bytes);
        
        let start = Instant::now();
        client.write_all(&padded).await?;
        
        let mut response = vec![0u8; 1024];
        client.read_exact(&mut response).await?;
        let latency = start.elapsed();
        
        latencies.push(latency);
        assert_eq!(&padded[..], &response[..], "Message mismatch");
    }
    
    let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    println!("  ✓ 10 round-trips completed");
    println!("  ✓ Average latency: {:?}", avg_latency);
    assert!(avg_latency < Duration::from_micros(100), "Latency too high");
    
    server_handle.await??;
    Ok(())
}

async fn test_backpressure() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nTest 3: Backpressure Mechanism");
    
    // Create a listener but don't read (to trigger backpressure)
    let server_handle = tokio::spawn(async move {
        let mut listener = SharedMemoryListener::bind("test_backpressure")?;
        let (_stream, _) = listener.accept().await?;
        
        // Don't read, causing backpressure
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok::<_, Box<dyn std::error::Error>>(())
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let mut client = SharedMemoryStream::connect("test_backpressure").await?;
    
    // Try to send many messages quickly
    let start = Instant::now();
    let mut blocked = false;
    
    for i in 0..1000 {
        let msg = vec![0u8; 1024]; // 1KB messages
        
        match timeout(Duration::from_millis(10), client.write_all(&msg)).await {
            Ok(Ok(_)) => continue,
            Ok(Err(e)) if e.to_string().contains("would block") => {
                blocked = true;
                println!("  ✓ Backpressure triggered after {} messages", i);
                break;
            }
            _ => {
                // Timeout means we're blocked
                blocked = true;
                println!("  ✓ Backpressure triggered after {} messages", i);
                break;
            }
        }
    }
    
    assert!(blocked, "Backpressure should have triggered");
    let elapsed = start.elapsed();
    println!("  ✓ Backpressure handled correctly in {:?}", elapsed);
    
    server_handle.await??;
    Ok(())
}

async fn test_concurrent_connections() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nTest 4: Concurrent Connections");
    
    const NUM_CLIENTS: usize = 10;
    const MESSAGES_PER_CLIENT: usize = 100;
    
    // Start server that handles multiple connections
    let server_handle = tokio::spawn(async move {
        let mut listener = SharedMemoryListener::bind("test_concurrent")?;
        let mut handles = Vec::new();
        
        for _ in 0..NUM_CLIENTS {
            let (mut stream, _) = listener.accept().await?;
            
            let handle = tokio::spawn(async move {
                for _ in 0..MESSAGES_PER_CLIENT {
                    let mut buf = vec![0u8; 256];
                    if stream.read_exact(&mut buf).await.is_ok() {
                        stream.write_all(&buf).await?;
                    }
                }
                Ok::<_, Box<dyn std::error::Error>>(())
            });
            handles.push(handle);
        }
        
        // Wait for all handlers
        for handle in handles {
            handle.await??;
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    });
    
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Start multiple clients
    let success_count = Arc::new(AtomicU64::new(0));
    let mut client_handles = Vec::new();
    
    let start = Instant::now();
    
    for client_id in 0..NUM_CLIENTS {
        let success_count = success_count.clone();
        
        let handle = tokio::spawn(async move {
            let mut stream = SharedMemoryStream::connect("test_concurrent").await?;
            
            for msg_id in 0..MESSAGES_PER_CLIENT {
                let msg = format!("Client {} Message {}", client_id, msg_id);
                let mut padded = vec![0u8; 256];
                padded[..msg.len()].copy_from_slice(msg.as_bytes());
                
                stream.write_all(&padded).await?;
                
                let mut response = vec![0u8; 256];
                stream.read_exact(&mut response).await?;
                
                assert_eq!(&padded[..], &response[..]);
                success_count.fetch_add(1, Ordering::Relaxed);
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        });
        client_handles.push(handle);
    }
    
    // Wait for all clients
    for handle in client_handles {
        handle.await??;
    }
    
    let elapsed = start.elapsed();
    let total_messages = success_count.load(Ordering::Relaxed);
    let throughput = total_messages as f64 / elapsed.as_secs_f64();
    
    println!("  ✓ {} concurrent clients completed", NUM_CLIENTS);
    println!("  ✓ {} messages exchanged in {:?}", total_messages, elapsed);
    println!("  ✓ Throughput: {:.0} msg/sec", throughput);
    
    assert_eq!(total_messages, (NUM_CLIENTS * MESSAGES_PER_CLIENT) as u64);
    assert!(throughput > 10000.0, "Throughput too low");
    
    server_handle.await??;
    Ok(())
}
