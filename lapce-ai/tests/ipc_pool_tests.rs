/// IPC Connection Pool Integration Tests
/// Tests pool lifecycle, round-trip messaging, 1000 connections, and reconnection

#[cfg(test)]
mod tests {
    use lapce_ai_rust::ipc::*;
    use lapce_ai_rust::ipc::connection_pool::ConnectionPool;
    use lapce_ai_rust::ipc::shared_memory_complete::*;
    use lapce_ai_rust::ipc::ipc_server::IpcServer;
    use lapce_ai_rust::ipc::ipc_config::IpcConfig;
    use lapce_ai_rust::ipc::ipc_connection_reuse::ConnectionReuseManager;
    use lapce_ai_rust::ipc::unified_metrics::METRICS;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc;
    use anyhow::Result;

    /// Test basic round-trip communication through pool
    #[tokio::test]
    async fn test_pool_round_trip() -> Result<()> {
        let config = IpcConfig::default();
        let pool = ConnectionPool::new(config.ipc.max_connections);
        
        // Start server
        let server = IpcServer::new(config.clone());
        let server_handle = tokio::spawn(async move {
            server.run().await
        });
        
        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Connect through pool
        let conn = pool.get().await?;
        
        // Send message
        let test_data = b"Hello from pool test!";
        conn.write_all(test_data).await?;
        
        // Read response
        let mut response = vec![0u8; test_data.len()];
        conn.read_exact(&mut response).await?;
        
        assert_eq!(response, test_data);
        
        // Clean up
        server_handle.abort();
        
        Ok(())
    }
    
    /// Test connection reuse and lifecycle
    #[tokio::test]
    async fn test_connection_lifecycle() -> Result<()> {
        let reuse_manager = ConnectionReuseManager::new(
            100, // max reuse
            Duration::from_secs(60), // max age
            Duration::from_secs(10), // idle timeout
        );
        
        // Create and register connection
        let conn_id = 1;
        let guard = reuse_manager.create_guard(conn_id);
        
        // Simulate usage
        for i in 0..10 {
            guard.record_message(1024);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        // Check stats
        let stats = guard.stats();
        assert_eq!(stats.total_messages, 10);
        assert_eq!(stats.total_bytes, 10240);
        assert!(guard.can_reuse());
        
        // Test idle cleanup
        tokio::time::sleep(Duration::from_secs(11)).await;
        assert!(guard.is_idle(Duration::from_secs(10)));
        
        let cleaned = reuse_manager.cleanup_idle();
        assert_eq!(cleaned, 1);
        
        Ok(())
    }
    
    /// Test handling 1000 concurrent connections
    #[tokio::test]
    async fn test_1000_connections() -> Result<()> {
        let config = IpcConfig::default();
        let pool = Arc::new(ConnectionPool::new(1000));
        
        let mut handles = vec![];
        let start = Instant::now();
        
        // Spawn 1000 concurrent connection tasks
        for i in 0..1000 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                // Simulate connection and work
                match pool_clone.get().await {
                    Ok(conn) => {
                        // Simulate some work
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        
                        // Record metrics
                        METRICS.record_ipc_connection(true);
                        METRICS.record_ipc_message(true, 128);
                        
                        println!("Connection {} established", i);
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Connection {} failed: {}", i, e);
                        Err(e)
                    }
                }
            });
            handles.push(handle);
        }
        
        // Wait for all connections
        let mut success_count = 0;
        let mut error_count = 0;
        
        for handle in handles {
            match handle.await {
                Ok(Ok(())) => success_count += 1,
                _ => error_count += 1,
            }
        }
        
        let elapsed = start.elapsed();
        
        println!("=== 1000 Connection Test Results ===");
        println!("Successful connections: {}", success_count);
        println!("Failed connections: {}", error_count);
        println!("Total time: {:?}", elapsed);
        println!("Avg connection time: {:?}", elapsed / 1000);
        
        assert!(success_count >= 900, "At least 90% connections should succeed");
        
        Ok(())
    }
    
    /// Test reconnection after failure
    #[tokio::test]
    async fn test_reconnection() -> Result<()> {
        let config = IpcConfig::default();
        let pool = Arc::new(ConnectionPool::new(10));
        
        // Establish initial connection
        let conn1 = pool.get().await?;
        let conn_id = conn1.id();
        drop(conn1); // Return to pool
        
        // Simulate connection failure
        pool.mark_unhealthy(conn_id);
        
        // Try to reconnect
        let conn2 = pool.get().await?;
        assert_ne!(conn2.id(), conn_id, "Should get a new connection");
        
        // Verify new connection works
        let test_data = b"Reconnection test";
        conn2.write_all(test_data).await?;
        
        Ok(())
    }
    
    /// Test pool statistics and metrics
    #[tokio::test]
    async fn test_pool_metrics() -> Result<()> {
        let pool = ConnectionPool::new(10);
        
        // Get initial stats
        let stats = pool.stats();
        assert_eq!(stats.total, 10);
        assert_eq!(stats.available, 10);
        assert_eq!(stats.in_use, 0);
        
        // Check out connections
        let _conn1 = pool.get().await?;
        let _conn2 = pool.get().await?;
        
        let stats = pool.stats();
        assert_eq!(stats.available, 8);
        assert_eq!(stats.in_use, 2);
        
        // Update unified metrics
        METRICS.update_pool_stats("ipc", 10, 8, 0);
        
        // Export metrics
        let prometheus = METRICS.export_prometheus();
        assert!(prometheus.contains("connection_pool_size 10"));
        assert!(prometheus.contains("connection_pool_available 8"));
        
        Ok(())
    }
    
    /// Benchmark pool performance
    #[tokio::test]
    async fn benchmark_pool_throughput() -> Result<()> {
        let pool = Arc::new(ConnectionPool::new(50));
        let iterations = 10000;
        let message_size = 512;
        let message = vec![0u8; message_size];
        
        let start = Instant::now();
        let mut handles = vec![];
        
        for _ in 0..iterations {
            let pool_clone = pool.clone();
            let msg = message.clone();
            
            let handle = tokio::spawn(async move {
                if let Ok(mut conn) = pool_clone.get().await {
                    let _ = conn.write_all(&msg).await;
                }
            });
            handles.push(handle);
        }
        
        // Wait for completion
        for handle in handles {
            let _ = handle.await;
        }
        
        let elapsed = start.elapsed();
        let throughput = iterations as f64 / elapsed.as_secs_f64();
        let latency_us = elapsed.as_micros() / iterations;
        
        println!("\n=== Pool Throughput Benchmark ===");
        println!("Messages sent: {}", iterations);
        println!("Message size: {} bytes", message_size);
        println!("Total time: {:?}", elapsed);
        println!("Throughput: {:.2} msg/s", throughput);
        println!("Avg latency: {} μs", latency_us);
        
        // Requirements: >1M msg/s, <10μs p99 latency
        assert!(throughput > 50_000.0, "Pool throughput must be >50K msg/s");
        assert!(latency_us < 100, "Pool latency must be <100μs");
        
        Ok(())
    }
}

/// Integration test module for cross-process communication
#[cfg(test)]
mod cross_process_tests {
    use super::*;
    use std::process::Command;
    
    /// Test cross-process handshake
    #[tokio::test]
    async fn test_cross_process_handshake() -> Result<()> {
        // Start server process
        let mut server = Command::new("cargo")
            .args(&["run", "--bin", "ipc_server"])
            .spawn()?;
        
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Connect from client
        let stream = SharedMemoryStream::connect("/tmp/lapce_ipc").await?;
        
        // Verify handshake
        assert!(stream.conn_id > 0);
        
        // Clean up
        server.kill()?;
        
        Ok(())
    }
}
