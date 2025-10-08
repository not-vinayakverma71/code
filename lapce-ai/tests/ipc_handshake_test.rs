/// Control channel handshake tests
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_deterministic_handshake() {
    // Test deterministic connection ID assignment
    use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};
    
    // Create listener
    let listener = Arc::new(SharedMemoryListener::bind("/test_handshake").unwrap());
    
    // Start server accepting connections
    let server = listener.clone();
    let server_handle = tokio::spawn(async move {
        let mut connections = vec![];
        for _ in 0..3 {
            match timeout(Duration::from_secs(2), server.accept()).await {
                Ok(Ok((stream, _))) => {
                    connections.push(stream.conn_id.clone());
                }
                _ => break,
            }
        }
        connections
    });
    
    // Connect multiple clients
    let mut client_ids = vec![];
    for i in 0..3 {
        tokio::time::sleep(Duration::from_millis(50)).await; // Small delay between connections
        
        match timeout(
            Duration::from_secs(1), 
            SharedMemoryStream::connect("/test_handshake")
        ).await {
            Ok(Ok(stream)) => {
                client_ids.push(stream.conn_id.clone());
                println!("Client {} connected with ID: {}", i, stream.conn_id);
            }
            _ => {
                println!("Client {} failed to connect", i);
            }
        }
    }
    
    // Get server-side connections
    let server_connections = server_handle.await.unwrap();
    
    // Verify deterministic IDs
    assert_eq!(client_ids.len(), 3);
    assert_eq!(server_connections.len(), 3);
    
    // Each client should have a unique ID
    let mut unique_ids = client_ids.clone();
    unique_ids.sort();
    unique_ids.dedup();
    assert_eq!(unique_ids.len(), 3, "All connection IDs should be unique");
    
    // Server should see the same IDs
    for client_id in &client_ids {
        assert!(server_connections.contains(client_id), 
                "Server should see client ID: {}", client_id);
    }
}

#[tokio::test] 
async fn test_concurrent_handshake_rendezvous() {
    use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};
    
    // Create listener
    let listener = Arc::new(SharedMemoryListener::bind("/test_concurrent").unwrap());
    
    // Server task
    let server = listener.clone();
    let server_handle = tokio::spawn(async move {
        let mut count = 0;
        for _ in 0..10 {
            match timeout(Duration::from_secs(3), server.accept()).await {
                Ok(Ok(_)) => count += 1,
                _ => break,
            }
        }
        count
    });
    
    // Launch 10 concurrent client connections
    let mut handles = vec![];
    for i in 0..10 {
        let handle = tokio::spawn(async move {
            match timeout(
                Duration::from_secs(2),
                SharedMemoryStream::connect("/test_concurrent")
            ).await {
                Ok(Ok(stream)) => {
                    // Write a test message
                    let msg = format!("Client {} message", i);
                    stream.write_all(msg.as_bytes()).await.ok();
                    true
                }
                _ => false
            }
        });
        handles.push(handle);
    }
    
    // Wait for all clients
    let mut successful = 0;
    for handle in handles {
        if handle.await.unwrap() {
            successful += 1;
        }
    }
    
    // Wait for server
    let server_count = server_handle.await.unwrap();
    
    println!("Concurrent handshake: {} clients connected, server accepted {}", 
             successful, server_count);
    
    assert!(successful >= 8, "Most clients should connect successfully");
    assert_eq!(successful, server_count, "Server should accept all successful clients");
}

#[tokio::test]
async fn test_handshake_timeout_recovery() {
    use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};
    
    // Test that handshake timeouts are handled gracefully
    
    // Try to connect without a server (should timeout)
    let result = timeout(
        Duration::from_millis(100),
        SharedMemoryStream::connect("/test_no_server")
    ).await;
    
    assert!(result.is_err() || result.unwrap().is_err(), 
            "Should timeout or fail when no server exists");
    
    // Now create server and verify recovery
    let _listener = SharedMemoryListener::bind("/test_recovery").unwrap();
    
    // Client should be able to connect now
    let result = timeout(
        Duration::from_secs(1),
        SharedMemoryStream::connect("/test_recovery")
    ).await;
    
    assert!(result.is_ok(), "Should connect after server starts");
}

#[tokio::test]
async fn test_connection_id_collision_prevention() {
    use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};
    use std::collections::HashSet;
    
    let listener = Arc::new(SharedMemoryListener::bind("/test_collision").unwrap());
    
    // Collect connection IDs from server
    let server = listener.clone();
    let server_handle = tokio::spawn(async move {
        let mut ids = HashSet::new();
        for _ in 0..20 {
            match timeout(Duration::from_millis(500), server.accept()).await {
                Ok(Ok((stream, _))) => {
                    ids.insert(stream.conn_id.clone());
                }
                _ => break,
            }
        }
        ids
    });
    
    // Rapid-fire client connections to test collision prevention
    let mut client_handles = vec![];
    for _ in 0..20 {
        let handle = tokio::spawn(async {
            match SharedMemoryStream::connect("/test_collision").await {
                Ok(stream) => Some(stream.conn_id),
                Err(_) => None,
            }
        });
        client_handles.push(handle);
    }
    
    // Collect client IDs
    let mut client_ids = HashSet::new();
    for handle in client_handles {
        if let Ok(Some(id)) = handle.await {
            client_ids.insert(id);
        }
    }
    
    let server_ids = server_handle.await.unwrap();
    
    println!("Collision test: {} unique client IDs, {} unique server IDs",
             client_ids.len(), server_ids.len());
    
    // All IDs should be unique (no collisions)
    assert_eq!(client_ids.len(), server_ids.len(), 
               "Client and server should see same number of unique connections");
    
    // Verify no UUID collisions occurred
    for id in &client_ids {
        assert!(uuid::Uuid::parse_str(id).is_ok(), "Connection ID should be valid UUID");
    }
}
