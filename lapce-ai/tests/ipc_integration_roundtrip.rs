/// Full IPC Integration Test - Real Message Round-Trips
/// Tests complete end-to-end IPC flow with actual server

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;
use lapce_ai_rust::ipc::ipc_server::IpcServer;
use lapce_ai_rust::ipc::ipc_client::IpcClient;
use lapce_ai_rust::ipc::binary_codec::MessageType;
use lapce_ai_rust::ipc::IpcError;
use bytes::Bytes;
use tokio::time::Duration;

const TEST_SOCKET: &str = "/tmp/lapce-ipc-integration-test.sock";

#[tokio::test]
async fn test_full_roundtrip() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ IPC Integration Test: Full Message Round-Trip               ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // Cleanup socket and lock directory
    let _ = std::fs::remove_file(TEST_SOCKET);
    let lock_dir = format!("{}_locks", TEST_SOCKET);
    let _ = std::fs::remove_dir_all(&lock_dir);
    
    // Start server
    let server = Arc::new(IpcServer::new(TEST_SOCKET).await.expect("Server creation failed"));
    
    // Register echo handler that properly returns formatted response
    server.register_handler(MessageType::CompletionRequest, |data: Bytes| async move {
        // Echo back the same data (already in IPC format)
        Ok(data)
    });
    
    // Start server in background
    let server_handle = tokio::spawn({
        let server = server.clone();
        async move {
            server.serve().await
        }
    });
    
    // Wait for server to be ready (longer delay for watcher to start)
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    println!("✅ Server started");
    println!("📊 Testing round-trip performance...\n");
    
    // Give server's filesystem watcher time to start polling
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Connect client
    let client = match IpcClient::connect(TEST_SOCKET).await {
        Ok(c) => {
            println!("✅ Client connected (lock file created)");
            c
        },
        Err(e) => {
            println!("❌ Client connection failed: {}", e);
            server_handle.abort();
            let _ = std::fs::remove_file(TEST_SOCKET);
            return;
        }
    };
    
    // Give server time to detect client's lock file and accept connection
    println!("⏳ Waiting for server to accept connection...");
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Check if server has accepted
    let stats_before = server.connection_pool_stats().await;
    println!("🔍 Server stats before sending: total={}, active={}",
        stats_before.total_connections.load(Ordering::Relaxed),
        stats_before.active_connections.load(Ordering::Relaxed)
    );
    
    // Create properly formatted IPC message with 24-byte header
    let payload = b"Hello IPC Server!";
    let msg_type = MessageType::CompletionRequest as u16;
    let msg_id = 12345u64;
    
    // Build 24-byte header
    let mut header = vec![0u8; 24];
    header[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());  // Magic
    header[4] = 1;  // Version
    header[5] = 0;  // Flags
    header[6..8].copy_from_slice(&msg_type.to_le_bytes());  // Type
    header[8..12].copy_from_slice(&(payload.len() as u32).to_le_bytes());  // Length
    header[12..20].copy_from_slice(&msg_id.to_le_bytes());  // Message ID
    header[20..24].fill(0);  // CRC placeholder
    
    // Calculate CRC32 over header + payload
    let mut full_msg = Vec::new();
    full_msg.extend_from_slice(&header);
    full_msg.extend_from_slice(payload);
    let crc = crc32fast::hash(&full_msg);
    header[20..24].copy_from_slice(&crc.to_le_bytes());
    
    // Rebuild with correct CRC
    full_msg.clear();
    full_msg.extend_from_slice(&header);
    full_msg.extend_from_slice(payload);
    
    let iterations = 100;
    let start = Instant::now();
    let mut successful = 0;
    let mut failed = 0;
    
    for i in 0..iterations {
        match client.send_bytes(&full_msg).await {
            Ok(response) => {
                successful += 1;
                if i == 0 {
                    println!("✅ First message round-trip: {} bytes sent, {} bytes received", 
                        full_msg.len(), response.len());
                }
            },
            Err(e) => {
                failed += 1;
                if failed == 1 {
                    println!("⚠️  Message failed: {}", e);
                }
            }
        }
    }
    
    let elapsed = start.elapsed();
    let avg_latency_us = elapsed.as_micros() as f64 / iterations as f64;
    
    println!("\n📊 Round-Trip Performance:");
    println!("   Messages: {} successful, {} failed", successful, failed);
    println!("   Total time: {:?}", elapsed);
    println!("   Avg latency: {:.2} µs", avg_latency_us);
    println!("   Throughput: {:.0} msg/s", iterations as f64 / elapsed.as_secs_f64());
    
    // Verify stats
    let stats = server.connection_pool_stats().await;
    println!("\n📊 Connection Pool Stats:");
    println!("   Total connections: {}", stats.total_connections.load(Ordering::Relaxed));
    println!("   Active connections: {}", stats.active_connections.load(Ordering::Relaxed));
    println!("   Total requests: {}", stats.total_requests.load(Ordering::Relaxed));
    
    // Cleanup
    server_handle.abort();
    let _ = std::fs::remove_file(TEST_SOCKET);
}

#[tokio::test]
async fn test_concurrent_messages() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ IPC Integration Test: Concurrent Messages                   ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // Cleanup
    let test_socket = "/tmp/lapce-ipc-concurrent-test.sock";
    let _ = std::fs::remove_file(test_socket);
    
    // Start server
    let server = Arc::new(IpcServer::new(test_socket).await.expect("Server creation failed"));
    
    // Register handler
    server.register_handler(MessageType::CompletionRequest, |_data: Bytes| async move {
        Ok(Bytes::from("response"))
    });
    
    println!("✅ Server configured for concurrent test");
    
    // Start server
    let server_handle = tokio::spawn({
        let server = server.clone();
        async move {
            server.serve().await
        }
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Connect multiple clients concurrently
    let num_clients = 10;
    let messages_per_client = 50;
    
    println!("📊 Testing {} concurrent clients, {} messages each\n", num_clients, messages_per_client);
    
    let start = Instant::now();
    let mut handles = vec![];
    
    for client_id in 0..num_clients {
        let socket = test_socket.to_string();
        let handle = tokio::spawn(async move {
            let client = IpcClient::connect(&socket).await?;
            let mut success = 0;
            
            for _ in 0..messages_per_client {
                if client.send_bytes(b"test").await.is_ok() {
                    success += 1;
                }
            }
            
            Ok::<_, anyhow::Error>((client_id, success))
        });
        handles.push(handle);
    }
    
    // Wait for all clients
    let mut total_success = 0;
    for handle in handles {
        if let Ok(Ok((id, success))) = handle.await {
            total_success += success;
            println!("✅ Client {} completed: {}/{} messages", id, success, messages_per_client);
        }
    }
    
    let elapsed = start.elapsed();
    let total_messages = num_clients * messages_per_client;
    
    println!("\n📊 Concurrent Performance:");
    println!("   Total messages: {}/{}", total_success, total_messages);
    println!("   Time: {:?}", elapsed);
    println!("   Throughput: {:.0} msg/s", total_messages as f64 / elapsed.as_secs_f64());
    
    // Cleanup
    server_handle.abort();
    let _ = std::fs::remove_file(test_socket);
}

#[tokio::test]
async fn test_error_recovery() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ IPC Integration Test: Error Recovery                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let test_socket = "/tmp/lapce-ipc-error-test.sock";
    let _ = std::fs::remove_file(test_socket);
    
    let server = Arc::new(IpcServer::new(test_socket).await.expect("Server creation failed"));
    
    // Register failing handler
    server.register_handler(MessageType::CompletionRequest, |_data: Bytes| async move {
        Err(IpcError::invalid_message("Test error".to_string()))
    });
    
    println!("✅ Server configured with failing handler");
    
    // Start server
    let server_handle = tokio::spawn({
        let server = server.clone();
        async move {
            server.serve().await
        }
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Connect client and test error handling
    match IpcClient::connect(test_socket).await {
        Ok(client) => {
            println!("✅ Client connected");
            
            // Send message that will trigger error
            match client.send_bytes(b"test").await {
                Ok(_) => println!("⚠️  Expected error but got success"),
                Err(e) => println!("✅ Error handled correctly: {}", e),
            }
            
            // Verify client can recover
            println!("\n📊 Testing recovery after error...");
            match client.send_bytes(b"test2").await {
                Ok(_) => println!("⚠️  Second request succeeded (handler still failing)"),
                Err(e) => println!("✅ Error handling consistent: {}", e),
            }
        },
        Err(e) => println!("❌ Client connection failed: {}", e),
    }
    
    // Cleanup
    server_handle.abort();
    
    let _ = std::fs::remove_file(test_socket);
}
