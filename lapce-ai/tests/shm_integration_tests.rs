/// Shared Memory Integration Tests
/// Tests handshake, buffer sharing, concurrent connections, and cleanup

use lapce_ai_rust::ipc::shared_memory_complete::{
    SharedMemoryListener, SharedMemoryStream, SharedMemoryBuffer,
    cleanup_shared_memory
};
use std::sync::Arc;
use tokio::sync::Barrier;
use std::time::Duration;

#[tokio::test]
async fn test_basic_handshake() {
    let path = "/test_basic_handshake";
    
    // Start server
    let listener = SharedMemoryListener::bind(path).expect("Failed to bind listener");
    
    // Start client connection in background
    let client_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        SharedMemoryStream::connect(path).await
    });
    
    // Accept connection
    let (mut server_stream, _addr) = listener.accept().await
        .expect("Failed to accept connection");
    
    // Client should connect successfully
    let mut client_stream = client_handle.await
        .expect("Client task failed")
        .expect("Client connection failed");
    
    // Test bidirectional communication
    let test_data = b"Hello from server";
    server_stream.write_all(test_data).await
        .expect("Server write failed");
    
    let mut buf = vec![0u8; test_data.len()];
    client_stream.read_exact(&mut buf).await
        .expect("Client read failed");
    
    assert_eq!(&buf[..], test_data);
    
    // Test reverse direction
    let test_data2 = b"Hello from client";
    client_stream.write_all(test_data2).await
        .expect("Client write failed");
    
    let mut buf2 = vec![0u8; test_data2.len()];
    server_stream.read_exact(&mut buf2).await
        .expect("Server read failed");
    
    assert_eq!(&buf2[..], test_data2);
}

#[tokio::test]
async fn test_multiple_concurrent_connections() {
    let path = "/test_concurrent_connections";
    let num_clients = 10;
    
    // Start server
    let listener = Arc::new(SharedMemoryListener::bind(path)
        .expect("Failed to bind listener"));
    
    // Barrier to synchronize client starts
    let barrier = Arc::new(Barrier::new(num_clients + 1));
    
    // Start server accept loop
    let listener_clone = listener.clone();
    let barrier_clone = barrier.clone();
    let server_handle = tokio::spawn(async move {
        let mut connections = Vec::new();
        
        // Wait for all clients to be ready
        barrier_clone.wait().await;
        
        for i in 0..num_clients {
            let (mut stream, _) = listener_clone.accept().await
                .expect(&format!("Failed to accept connection {}", i));
            
            // Send unique message to each client
            let msg = format!("Hello client {}", i);
            stream.write_all(msg.as_bytes()).await
                .expect(&format!("Failed to write to client {}", i));
            
            connections.push(stream);
        }
        
        connections
    });
    
    // Start multiple clients
    let mut client_handles = Vec::new();
    for i in 0..num_clients {
        let barrier_clone = barrier.clone();
        let handle = tokio::spawn(async move {
            // Wait for all clients to be ready
            barrier_clone.wait().await;
            
            // Small delay to avoid thundering herd
            tokio::time::sleep(Duration::from_millis(i as u64 * 10)).await;
            
            let mut stream = SharedMemoryStream::connect(path).await
                .expect(&format!("Client {} failed to connect", i));
            
            // Read message from server
            let mut buf = vec![0u8; 32];
            let n = stream.read(&mut buf).await
                .expect(&format!("Client {} failed to read", i));
            
            let msg = String::from_utf8_lossy(&buf[..n]);
            assert!(msg.contains(&format!("client {}", i)));
            
            stream
        });
        client_handles.push(handle);
    }
    
    // Wait for all connections
    let _server_connections = server_handle.await.expect("Server task failed");
    for handle in client_handles {
        let _stream = handle.await.expect("Client task failed");
    }
}

#[tokio::test]
async fn test_connection_cleanup() {
    let path = "/test_cleanup";
    
    // Create and drop connections
    {
        let listener = SharedMemoryListener::bind(path)
            .expect("Failed to bind listener");
        
        let client_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            SharedMemoryStream::connect(path).await
        });
        
        let (_server_stream, _) = listener.accept().await
            .expect("Failed to accept");
        let _client_stream = client_handle.await
            .expect("Client task failed")
            .expect("Client connection failed");
        
        // Streams will be dropped here
    }
    
    // Explicitly cleanup shared memory
    cleanup_shared_memory(&format!("{}_control", path));
    
    // Should be able to bind again after cleanup
    let _listener2 = SharedMemoryListener::bind(path)
        .expect("Should be able to bind after cleanup");
}

#[tokio::test]
async fn test_large_message_transfer() {
    let path = "/test_large_message";
    
    let listener = SharedMemoryListener::bind(path)
        .expect("Failed to bind listener");
    
    let client_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        SharedMemoryStream::connect(path).await
    });
    
    let (mut server_stream, _) = listener.accept().await
        .expect("Failed to accept connection");
    let mut client_stream = client_handle.await
        .expect("Client task failed")
        .expect("Client connection failed");
    
    // Send large message (500KB)
    let large_data: Vec<u8> = (0..500_000).map(|i| (i % 256) as u8).collect();
    
    // Send in chunks
    for chunk in large_data.chunks(1024) {
        server_stream.write_all(chunk).await
            .expect("Failed to write chunk");
    }
    
    // Read on client side
    let mut received = Vec::new();
    let mut buf = vec![0u8; 1024];
    let mut total_read = 0;
    
    while total_read < large_data.len() {
        let n = client_stream.read(&mut buf).await
            .expect("Failed to read");
        if n == 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
            continue;
        }
        received.extend_from_slice(&buf[..n]);
        total_read += n;
    }
    
    assert_eq!(received.len(), large_data.len());
    assert_eq!(received, large_data);
}

#[tokio::test]
async fn test_connection_timeout() {
    let path = "/test_timeout";
    
    // Try to connect without a server
    let result = tokio::time::timeout(
        Duration::from_secs(6),
        SharedMemoryStream::connect(path)
    ).await;
    
    assert!(result.is_err() || result.unwrap().is_err(),
            "Should timeout or fail when no server is listening");
}

#[tokio::test]
async fn test_bidirectional_streaming() {
    let path = "/test_bidirectional";
    
    let listener = SharedMemoryListener::bind(path)
        .expect("Failed to bind listener");
    
    let client_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let mut stream = SharedMemoryStream::connect(path).await
            .expect("Failed to connect");
        
        // Client sends and receives simultaneously
        for i in 0..10 {
            let msg = format!("Client message {}", i);
            stream.write_all(msg.as_bytes()).await
                .expect("Client write failed");
            
            let mut buf = vec![0u8; 32];
            let n = stream.read(&mut buf).await
                .expect("Client read failed");
            
            let response = String::from_utf8_lossy(&buf[..n]);
            assert!(response.contains(&format!("Server message {}", i)));
        }
        
        stream
    });
    
    let (mut server_stream, _) = listener.accept().await
        .expect("Failed to accept connection");
    
    // Server sends and receives simultaneously
    for i in 0..10 {
        let mut buf = vec![0u8; 32];
        let n = server_stream.read(&mut buf).await
            .expect("Server read failed");
        
        let request = String::from_utf8_lossy(&buf[..n]);
        assert!(request.contains(&format!("Client message {}", i)));
        
        let msg = format!("Server message {}", i);
        server_stream.write_all(msg.as_bytes()).await
            .expect("Server write failed");
    }
    
    let _client_stream = client_handle.await
        .expect("Client task failed");
}

#[tokio::test]
async fn test_connection_reuse() {
    let path = "/test_reuse";
    
    let listener = SharedMemoryListener::bind(path)
        .expect("Failed to bind listener");
    
    // First connection
    {
        let client_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            SharedMemoryStream::connect(path).await
        });
        
        let (mut server_stream, _) = listener.accept().await
            .expect("Failed to accept first connection");
        let mut client_stream = client_handle.await
            .expect("Client task failed")
            .expect("Client connection failed");
        
        // Exchange data
        server_stream.write_all(b"First").await.unwrap();
        let mut buf = vec![0u8; 5];
        client_stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf[..], b"First");
    }
    
    // Second connection on same listener
    {
        let client_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            SharedMemoryStream::connect(path).await
        });
        
        let (mut server_stream, _) = listener.accept().await
            .expect("Failed to accept second connection");
        let mut client_stream = client_handle.await
            .expect("Client task failed")
            .expect("Client connection failed");
        
        // Exchange data
        server_stream.write_all(b"Second").await.unwrap();
        let mut buf = vec![0u8; 6];
        client_stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf[..], b"Second");
    }
}
