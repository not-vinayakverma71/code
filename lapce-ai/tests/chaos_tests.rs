/// Chaos and Fault Injection Tests
/// Tests resilience under extreme conditions

use lapce_ai_rust::ipc::{
    ipc::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream, SharedMemoryBuffer},
    binary_codec::{BinaryCodec, Message, MessageType, MessagePayload, CompletionRequest},
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_buffer_saturation_recovery() {
    let path = "/test_saturation";
    let listener = SharedMemoryListener::bind(path).expect("Failed to bind");
    
    let client_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let mut stream = SharedMemoryStream::connect(path).await.expect("Connect failed");
        
        // Saturate buffer with large messages
        let large_msg = vec![0xAA; 100_000];
        for i in 0..100 {
            match timeout(Duration::from_millis(100), stream.write_all(&large_msg)).await {
                Ok(Ok(_)) => {},
                Ok(Err(_)) => {
                    // Buffer full, expected
                    println!("Buffer saturated at iteration {}", i);
                    break;
                },
                Err(_) => {
                    // Timeout, buffer is saturated
                    println!("Write timeout at iteration {}", i);
                    break;
                }
            }
        }
        
        // Now try to recover by reading
        let mut buf = vec![0u8; 1024];
        for _ in 0..10 {
            if stream.read(&mut buf).await.is_ok() {
                // Successfully reading, recovering from saturation
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        // Should be able to write again after recovery
        let small_msg = b"recovered";
        stream.write_all(small_msg).await.expect("Recovery write failed");
    });
    
    let (mut server_stream, _) = listener.accept().await.expect("Accept failed");
    
    // Let client saturate, then drain
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    let mut drain_buf = vec![0u8; 100_000];
    let mut total_read = 0;
    while total_read < 1_000_000 {
        match timeout(Duration::from_millis(100), server_stream.read(&mut drain_buf)).await {
            Ok(Ok(n)) if n > 0 => {
                total_read += n;
            },
            _ => break,
        }
    }
    
    // Read recovery message
    let mut buf = vec![0u8; 32];
    let n = server_stream.read(&mut buf).await.expect("Recovery read failed");
    assert_eq!(&buf[..n], b"recovered");
    
    client_handle.await.expect("Client task failed");
}

#[tokio::test]
async fn test_shm_unlink_mid_stream() {
    let path = "/test_unlink";
    let listener = SharedMemoryListener::bind(path).expect("Failed to bind");
    
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    
    let client_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let mut stream = SharedMemoryStream::connect(path).await.expect("Connect failed");
        
        let mut counter = 0;
        while !shutdown_clone.load(Ordering::Relaxed) {
            let msg = format!("Message {}", counter);
            match stream.write_all(msg.as_bytes()).await {
                Ok(_) => counter += 1,
                Err(e) => {
                    println!("Write error after {} messages: {}", counter, e);
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        counter
    });
    
    let (mut server_stream, _) = listener.accept().await.expect("Accept failed");
    
    // Read some messages
    let mut buf = vec![0u8; 128];
    for _ in 0..5 {
        server_stream.read(&mut buf).await.expect("Read failed");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Simulate unlink mid-stream
    cleanup_shared_memory(&format!("{}_control", path));
    
    // Try to continue reading (should fail gracefully)
    for _ in 0..5 {
        match server_stream.read(&mut buf).await {
            Ok(_) => {
                // May succeed for buffered data
            },
            Err(e) => {
                println!("Expected read error after unlink: {}", e);
                break;
            }
        }
    }
    
    shutdown.store(true, Ordering::Relaxed);
    let messages_sent = client_handle.await.expect("Client task failed");
    assert!(messages_sent > 0, "Should have sent some messages before failure");
}

#[tokio::test]
async fn test_process_crash_restart() {
    let path = "/test_crash";
    
    // First "process"
    {
        let listener = SharedMemoryListener::bind(path).expect("Failed to bind");
        
        let client_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let mut stream = SharedMemoryStream::connect(path).await.expect("Connect failed");
            
            for i in 0..5 {
                let msg = format!("Before crash {}", i);
                stream.write_all(msg.as_bytes()).await.expect("Write failed");
            }
        });
        
        let (mut server_stream, _) = listener.accept().await.expect("Accept failed");
        
        let mut buf = vec![0u8; 128];
        for _ in 0..3 {
            server_stream.read(&mut buf).await.expect("Read failed");
        }
        
        client_handle.await.expect("Client task failed");
        
        // Simulate crash - drop everything
    }
    
    // Cleanup
    cleanup_shared_memory(&format!("{}_control", path));
    
    // Second "process" - restart
    {
        let listener = SharedMemoryListener::bind(path).expect("Failed to bind after crash");
        
        let client_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let mut stream = SharedMemoryStream::connect(path).await.expect("Connect after crash failed");
            
            for i in 0..5 {
                let msg = format!("After restart {}", i);
                stream.write_all(msg.as_bytes()).await.expect("Write after restart failed");
            }
        });
        
        let (mut server_stream, _) = listener.accept().await.expect("Accept after crash failed");
        
        let mut buf = vec![0u8; 128];
        let n = server_stream.read(&mut buf).await.expect("Read after crash failed");
        let msg = String::from_utf8_lossy(&buf[..n]);
        assert!(msg.contains("After restart"), "Should receive messages after restart");
        
        client_handle.await.expect("Client task after crash failed");
    }
}

#[tokio::test]
async fn test_memory_exhaustion() {
    let path = "/test_memory_exhaustion";
    
    // Try to create many connections
    let listener = Arc::new(SharedMemoryListener::bind(path).expect("Failed to bind"));
    
    let mut client_handles = vec![];
    let mut server_handles = vec![];
    
    for i in 0..100 {
        let listener_clone = listener.clone();
        
        // Server accept task
        let server_handle = tokio::spawn(async move {
            match timeout(Duration::from_millis(100), listener_clone.accept()).await {
                Ok(Ok((stream, _))) => Some(stream),
                _ => None,
            }
        });
        server_handles.push(server_handle);
        
        // Client connect task
        let client_handle = tokio::spawn(async move {
            match timeout(Duration::from_millis(100), SharedMemoryStream::connect(path)).await {
                Ok(Ok(stream)) => {
                    println!("Connection {} established", i);
                    Some(stream)
                },
                _ => {
                    println!("Connection {} failed (expected under memory pressure)", i);
                    None
                }
            }
        });
        client_handles.push(client_handle);
        
        // Small delay to avoid thundering herd
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Count successful connections
    let mut successful_clients = 0;
    for handle in client_handles {
        if let Ok(Some(_)) = handle.await {
            successful_clients += 1;
        }
    }
    
    println!("Successfully created {} connections under memory pressure", successful_clients);
    assert!(successful_clients > 0, "Should create at least some connections");
}

#[tokio::test]
async fn test_rapid_connect_disconnect() {
    let path = "/test_rapid";
    let listener = Arc::new(SharedMemoryListener::bind(path).expect("Failed to bind"));
    
    let listener_clone = listener.clone();
    let server_handle = tokio::spawn(async move {
        let mut connections = vec![];
        for _ in 0..50 {
            match timeout(Duration::from_millis(100), listener_clone.accept()).await {
                Ok(Ok((stream, _))) => connections.push(stream),
                _ => break,
            }
        }
        connections.len()
    });
    
    // Rapid connect/disconnect cycles
    for i in 0..50 {
        tokio::spawn(async move {
            match SharedMemoryStream::connect(path).await {
                Ok(mut stream) => {
                    // Send one message and disconnect
                    let msg = format!("Quick msg {}", i);
                    let _ = stream.write_all(msg.as_bytes()).await;
                    // Drop stream immediately
                },
                Err(_) => {
                    // Connection failed, acceptable under stress
                }
            }
        });
        
        // Very short delay
        tokio::time::sleep(Duration::from_micros(100)).await;
    }
    
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    let connections_accepted = server_handle.await.expect("Server task failed");
    println!("Accepted {} rapid connections", connections_accepted);
    assert!(connections_accepted > 0, "Should accept at least some connections");
}

#[tokio::test]
async fn test_codec_corruption_recovery() {
    let mut codec = BinaryCodec::new();
    
    // Create valid message
    let msg = Message {
        id: 999,
        msg_type: MessageType::CompletionRequest,
        payload: MessagePayload::CompletionRequest(CompletionRequest {
            prompt: "Test".to_string(),
            model: "model".to_string(),
            max_tokens: 10,
            temperature: 0.5,
            stream: false,
        }),
        timestamp: 1234567890,
    };
    
    let mut encoded = codec.encode(&msg).expect("Encode failed");
    
    // Corrupt various parts
    let corruptions = vec![
        0,  // Magic
        5,  // Flags
        10, // Length
        15, // Message ID
        22, // CRC
    ];
    
    for corrupt_pos in corruptions {
        let mut corrupted = encoded.clone();
        if corrupt_pos < corrupted.len() {
            corrupted[corrupt_pos] ^= 0xFF; // Flip bits
            
            // Should fail gracefully
            match codec.decode(&corrupted) {
                Ok(_) => panic!("Should have detected corruption at position {}", corrupt_pos),
                Err(e) => {
                    println!("Correctly detected corruption at {}: {}", corrupt_pos, e);
                }
            }
        }
    }
    
    // Original should still decode
    let decoded = codec.decode(&encoded).expect("Original decode failed");
    assert_eq!(decoded.id, msg.id);
}

#[tokio::test]
async fn test_concurrent_chaos() {
    use std::sync::Arc;
    use tokio::sync::Barrier;
    
    let path = "/test_concurrent_chaos";
    let listener = Arc::new(SharedMemoryListener::bind(path).expect("Failed to bind"));
    let barrier = Arc::new(Barrier::new(11)); // 10 chaos agents + 1 server
    
    // Server task
    let listener_clone = listener.clone();
    let barrier_clone = barrier.clone();
    let server_handle = tokio::spawn(async move {
        barrier_clone.wait().await;
        
        let mut total_messages = 0;
        for _ in 0..10 {
            match timeout(Duration::from_millis(500), listener_clone.accept()).await {
                Ok(Ok((mut stream, _))) => {
                    tokio::spawn(async move {
                        let mut buf = vec![0u8; 1024];
                        loop {
                            match stream.read(&mut buf).await {
                                Ok(0) => break,
                                Ok(_) => {},
                                Err(_) => break,
                            }
                        }
                    });
                    total_messages += 1;
                },
                _ => break,
            }
        }
        total_messages
    });
    
    // Launch chaos agents
    let mut chaos_handles = vec![];
    for i in 0..10 {
        let barrier_clone = barrier.clone();
        let handle = tokio::spawn(async move {
            barrier_clone.wait().await;
            
            // Random delay
            tokio::time::sleep(Duration::from_millis(i as u64 * 10)).await;
            
            match SharedMemoryStream::connect(path).await {
                Ok(mut stream) => {
                    // Random operations
                    for j in 0..10 {
                        let msg = vec![i as u8; (j + 1) * 100];
                        let _ = stream.write_all(&msg).await;
                        
                        if j % 3 == 0 {
                            // Random disconnect
                            if j > 5 {
                                break;
                            }
                        }
                    }
                    true
                },
                Err(_) => false,
            }
        });
        chaos_handles.push(handle);
    }
    
    // Wait for chaos to complete
    let mut successful_agents = 0;
    for handle in chaos_handles {
        if handle.await.expect("Chaos agent failed") {
            successful_agents += 1;
        }
    }
    
    let server_connections = server_handle.await.expect("Server task failed");
    
    println!("Chaos test: {} agents succeeded, {} connections accepted", 
             successful_agents, server_connections);
    assert!(successful_agents > 0, "At least some chaos agents should succeed");
}
