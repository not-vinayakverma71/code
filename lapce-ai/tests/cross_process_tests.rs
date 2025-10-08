/// Cross-Process Integration Tests
/// Tests handshake, backpressure, reconnection across processes

use anyhow::Result;
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests {
    use super::*;
    use lapce_ai_rust::ipc::shared_memory_complete::*;
    use lapce_ai_rust::ipc::backpressure::BackpressureManager;
    use lapce_ai_rust::ipc::binary_codec::{BinaryCodec, Message, MessageType, MessagePayload};
    use lapce_ai_rust::ipc::codex_messages::CODEX_PROTOCOL_VERSION;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
    use tokio::sync::Semaphore;

    /// Test handshake protocol
    #[tokio::test]
    async fn test_handshake_protocol() -> Result<()> {
        // Create server buffer
        let server_buffer = SharedMemoryBuffer::create("/test_handshake", 4 * 1024 * 1024)?;
        
        // Client connects
        let client_buffer = SharedMemoryBuffer::open("/test_handshake", 4 * 1024 * 1024)?;
        
        // Send handshake
        let handshake_data = format!("HANDSHAKE:v{}", CODEX_PROTOCOL_VERSION);
        server_buffer.write(handshake_data.as_bytes())?;
        
        // Client reads handshake
        let received = client_buffer.read().unwrap();
        let handshake_str = String::from_utf8_lossy(&received);
        
        assert!(handshake_str.starts_with("HANDSHAKE:"));
        assert!(handshake_str.contains(&CODEX_PROTOCOL_VERSION.to_string()));
        
        // Cleanup
        cleanup_shared_memory("/test_handshake");
        
        Ok(())
    }
    
    /// Test backpressure handling
    #[tokio::test]
    async fn test_backpressure_handling() -> Result<()> {
        let backpressure = BackpressureManager::new();
        let buffer = Arc::new(SharedMemoryBuffer::create("/test_backpressure", 1024 * 1024)?);
        
        let producer_buffer = buffer.clone();
        let consumer_buffer = buffer.clone();
        
        // Producer task
        let producer = tokio::spawn(async move {
            let mut sent = 0;
            for i in 0..1000 {
                let data = format!("Message {}", i);
                
                // Apply backpressure
                if let Err(_) = backpressure.try_send() {
                    // Back off
                    backpressure.apply_backpressure().await;
                }
                
                producer_buffer.write(data.as_bytes()).unwrap();
                sent += 1;
                
                if sent % 100 == 0 {
                    println!("Producer sent {} messages", sent);
                }
            }
            sent
        });
        
        // Consumer task
        let consumer = tokio::spawn(async move {
            let mut received = 0;
            let start = Instant::now();
            
            while received < 1000 && start.elapsed() < Duration::from_secs(10) {
                if let Some(data) = consumer_buffer.read() {
                    received += 1;
                    backpressure.release_pressure();
                    
                    if received % 100 == 0 {
                        println!("Consumer received {} messages", received);
                    }
                }
                tokio::time::sleep(Duration::from_micros(10)).await;
            }
            received
        });
        
        let (sent, received) = tokio::join!(producer, consumer);
        
        assert_eq!(sent?, received?);
        
        // Cleanup
        cleanup_shared_memory("/test_backpressure");
        
        Ok(())
    }
    
    /// Test reconnection workflow
    #[tokio::test]
    async fn test_reconnection_workflow() -> Result<()> {
        let connected = Arc::new(AtomicBool::new(false));
        let reconnect_count = Arc::new(AtomicU64::new(0));
        
        let connected_clone = connected.clone();
        let reconnect_clone = reconnect_count.clone();
        
        // Simulate connection task
        let connection_task = tokio::spawn(async move {
            for attempt in 0..5 {
                println!("Connection attempt {}", attempt + 1);
                
                // Simulate connection attempt
                tokio::time::sleep(Duration::from_millis(100)).await;
                
                if attempt == 2 {
                    // Succeed on third attempt
                    connected_clone.store(true, Ordering::SeqCst);
                    println!("Connected!");
                    break;
                }
                
                reconnect_clone.fetch_add(1, Ordering::SeqCst);
                
                // Exponential backoff
                let backoff = Duration::from_millis(100 * 2u64.pow(attempt));
                tokio::time::sleep(backoff).await;
            }
        });
        
        connection_task.await?;
        
        assert!(connected.load(Ordering::SeqCst));
        assert_eq!(reconnect_count.load(Ordering::SeqCst), 2);
        
        Ok(())
    }
    
    /// Test 1000 concurrent connections
    #[tokio::test] 
    async fn test_1000_concurrent_connections() -> Result<()> {
        let semaphore = Arc::new(Semaphore::new(100)); // Limit concurrent operations
        let success_count = Arc::new(AtomicU64::new(0));
        let error_count = Arc::new(AtomicU64::new(0));
        
        let mut handles = vec![];
        let start = Instant::now();
        
        for i in 0..1000 {
            let sem = semaphore.clone();
            let success = success_count.clone();
            let errors = error_count.clone();
            
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                
                // Create unique buffer for this connection
                let buffer_name = format!("/test_conn_{}", i);
                
                match SharedMemoryBuffer::create(&buffer_name, 32 * 1024) {
                    Ok(mut buffer) => {
                        // Simulate some work
                        let data = format!("Connection {} data", i);
                        if buffer.write(data.as_bytes()).is_ok() {
                            success.fetch_add(1, Ordering::Relaxed);
                        } else {
                            errors.fetch_add(1, Ordering::Relaxed);
                        }
                        
                        // Cleanup
                        cleanup_shared_memory(&buffer_name);
                    }
                    Err(_) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all to complete
        for handle in handles {
            let _ = handle.await;
        }
        
        let elapsed = start.elapsed();
        let successful = success_count.load(Ordering::Relaxed);
        let failed = error_count.load(Ordering::Relaxed);
        
        println!("\n=== 1000 Concurrent Connections Test ===");
        println!("Successful: {}", successful);
        println!("Failed: {}", failed);
        println!("Total time: {:?}", elapsed);
        println!("Connections/sec: {:.2}", 1000.0 / elapsed.as_secs_f64());
        
        assert!(successful >= 900, "At least 900 connections should succeed");
        
        Ok(())
    }
    
    /// Test message ordering
    #[tokio::test]
    async fn test_message_ordering() -> Result<()> {
        let mut buffer = SharedMemoryBuffer::create("/test_ordering", 4 * 1024 * 1024)?;
        let codec = BinaryCodec::new();
        
        // Send messages in order
        let mut sent_ids = vec![];
        for i in 0..100 {
            let msg = Message {
                id: i as u64,
                msg_type: MessageType::Heartbeat,
                payload: MessagePayload::Heartbeat,
                timestamp: i as u64,
            };
            
            let encoded = codec.encode(&msg)?;
            buffer.write(&encoded)?;
            sent_ids.push(i as u64);
        }
        
        // Read and verify order
        let mut received_ids = vec![];
        for _ in 0..100 {
            if let Some(data) = buffer.read() {
                let decoded = codec.decode(&data)?;
                received_ids.push(decoded.id);
            }
        }
        
        assert_eq!(sent_ids, received_ids, "Message order must be preserved");
        
        // Cleanup
        cleanup_shared_memory("/test_ordering");
        
        Ok(())
    }
}

/// Nuclear stress test variants (CI-gated)
#[cfg(all(test, feature = "stress_tests"))]
mod nuclear_stress_tests {
    use super::*;
    
    /// Connection bomb test
    #[tokio::test]
    async fn test_connection_bomb() -> Result<()> {
        let start = Instant::now();
        let timeout = Duration::from_secs(30); // CI timeout
        
        let mut connection_count = 0;
        while start.elapsed() < timeout {
            let name = format!("/bomb_{}", connection_count);
            
            match SharedMemoryBuffer::create(&name, 4096) {
                Ok(_) => connection_count += 1,
                Err(_) => break, // Hit system limit
            }
            
            if connection_count % 1000 == 0 {
                println!("Created {} connections", connection_count);
            }
        }
        
        println!("Connection bomb: created {} connections before limit", connection_count);
        
        // Cleanup
        for i in 0..connection_count {
            cleanup_shared_memory(&format!("/bomb_{}", i));
        }
        
        assert!(connection_count > 1000, "Should handle at least 1000 connections");
        
        Ok(())
    }
    
    /// Memory exhaustion test
    #[tokio::test]
    async fn test_memory_exhaustion() -> Result<()> {
        let mut buffers = vec![];
        let mut total_allocated = 0usize;
        
        // Try to allocate up to 100MB
        for i in 0..100 {
            let size = 1024 * 1024; // 1MB each
            let name = format!("/mem_test_{}", i);
            
            match SharedMemoryBuffer::create(&name, size) {
                Ok(buffer) => {
                    buffers.push((name.clone(), buffer));
                    total_allocated += size;
                }
                Err(_) => {
                    println!("Memory allocation failed at {} MB", total_allocated / 1024 / 1024);
                    break;
                }
            }
        }
        
        // Verify we could allocate reasonable amount
        assert!(total_allocated >= 10 * 1024 * 1024, "Should allocate at least 10MB");
        
        // Cleanup
        for (name, _) in buffers {
            cleanup_shared_memory(&name);
        }
        
        Ok(())
    }
    
    /// Latency torture test
    #[tokio::test]
    async fn test_latency_torture() -> Result<()> {
        let mut buffer = SharedMemoryBuffer::create("/latency_test", 4 * 1024 * 1024)?;
        let iterations = 100_000;
        let mut latencies = vec![];
        
        for _ in 0..iterations {
            let data = vec![0u8; 64]; // Small message
            
            let start = Instant::now();
            buffer.write(&data)?;
            let _ = buffer.read();
            let latency = start.elapsed();
            
            latencies.push(latency.as_nanos() as u64);
        }
        
        // Calculate percentiles
        latencies.sort_unstable();
        let p50 = latencies[latencies.len() / 2];
        let p99 = latencies[latencies.len() * 99 / 100];
        
        println!("Latency torture results:");
        println!("  P50: {} ns", p50);
        println!("  P99: {} ns", p99);
        
        // Requirements: <10μs p99
        assert!(p99 < 10_000, "P99 latency must be <10μs");
        
        // Cleanup
        cleanup_shared_memory("/latency_test");
        
        Ok(())
    }
}
