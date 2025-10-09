/// Scalability Tests
/// Target: ≥1000 concurrent connections, efficient resource utilization

use lapce_ai_rust::ipc::{
    ipc::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream},
    binary_codec::{BinaryCodec, Message, MessageType, MessagePayload, CompletionRequest},
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_1000_concurrent_connections() {
    let path = "/test_1000_connections";
    let target_connections = 1000;
    
    let listener = Arc::new(SharedMemoryListener::bind(path).expect("Failed to bind"));
    let connected_count = Arc::new(AtomicUsize::new(0));
    let processed_count = Arc::new(AtomicUsize::new(0));
    
    // Server task
    let listener_clone = listener.clone();
    let connected_clone = connected_count.clone();
    let processed_clone = processed_count.clone();
    
    let server_handle = tokio::spawn(async move {
        let semaphore = Arc::new(Semaphore::new(target_connections));
        let mut handles = vec![];
        
        for _ in 0..target_connections {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            
            match tokio::time::timeout(
                Duration::from_millis(100),
                listener_clone.accept()
            ).await {
                Ok(Ok((mut stream, _))) => {
                    connected_clone.fetch_add(1, Ordering::Relaxed);
                    let processed_clone = processed_clone.clone();
                    
                    let handle = tokio::spawn(async move {
                        let _permit = permit; // Hold permit
                        let mut buf = vec![0u8; 256];
                        
                        // Process messages
                        loop {
                            match stream.read(&mut buf).await {
                                Ok(n) if n > 0 => {
                                    processed_clone.fetch_add(1, Ordering::Relaxed);
                                    // Echo back
                                    let _ = stream.write_all(&buf[..n]).await;
                                },
                                _ => break,
                            }
                        }
                    });
                    handles.push(handle);
                },
                _ => break,
            }
        }
        
        // Wait for all handlers
        for handle in handles {
            let _ = handle.await;
        }
        
        connected_clone.load(Ordering::Relaxed)
    });
    
    // Client connections
    let mut client_handles = vec![];
    let start = Instant::now();
    
    for i in 0..target_connections {
        let connected_clone = connected_count.clone();
        
        let handle = tokio::spawn(async move {
            // Stagger connections slightly
            tokio::time::sleep(Duration::from_micros(i as u64 * 100)).await;
            
            match tokio::time::timeout(
                Duration::from_secs(10),
                SharedMemoryStream::connect(path)
            ).await {
                Ok(Ok(mut stream)) => {
                    // Send test message
                    let msg = format!("Client {}", i);
                    if stream.write_all(msg.as_bytes()).await.is_ok() {
                        // Read echo
                        let mut buf = vec![0u8; 256];
                        let _ = stream.read(&mut buf).await;
                        return true;
                    }
                },
                _ => {},
            }
            false
        });
        client_handles.push(handle);
        
        // Yield periodically to avoid overwhelming
        if i % 100 == 0 {
            tokio::task::yield_now().await;
        }
    }
    
    // Wait for clients
    let mut successful_clients = 0;
    for handle in client_handles {
        if handle.await.unwrap_or(false) {
            successful_clients += 1;
        }
    }
    
    let elapsed = start.elapsed();
    let server_connections = server_handle.await.expect("Server failed");
    let messages_processed = processed_count.load(Ordering::Relaxed);
    
    println!("Scalability test results:");
    println!("  Target connections: {}", target_connections);
    println!("  Successful clients: {}", successful_clients);
    println!("  Server accepted: {}", server_connections);
    println!("  Messages processed: {}", messages_processed);
    println!("  Time elapsed: {:?}", elapsed);
    println!("  Connections/sec: {:.2}", successful_clients as f64 / elapsed.as_secs_f64());
    
    // Verify we achieved at least 95% of target
    assert!(successful_clients >= (target_connections * 95 / 100),
            "Should achieve at least 95% of target connections");
}

#[tokio::test]
async fn test_connection_pool_scaling() {
    use lapce_ai_rust::ipc::connection_pool::{ConnectionPool, PoolConfig};
    
    let config = PoolConfig {
        min_connections: 10,
        max_connections: 1000,
        connection_timeout: Duration::from_secs(5),
        idle_timeout: Duration::from_secs(30),
        max_lifetime: Duration::from_secs(300),
        retry_attempts: 3,
        retry_delay: Duration::from_millis(100),
    };
    
    let pool = Arc::new(ConnectionPool::new(config));
    let active_connections = Arc::new(AtomicUsize::new(0));
    
    // Simulate load pattern
    let mut handles = vec![];
    
    // Phase 1: Gradual ramp-up
    for i in 0..100 {
        let pool_clone = pool.clone();
        let active_clone = active_connections.clone();
        
        let handle = tokio::spawn(async move {
            match pool_clone.acquire().await {
                Ok(conn) => {
                    active_clone.fetch_add(1, Ordering::Relaxed);
                    // Simulate work
                    tokio::time::sleep(Duration::from_millis(100 + i)).await;
                    active_clone.fetch_sub(1, Ordering::Relaxed);
                    drop(conn);
                    true
                },
                Err(_) => false,
            }
        });
        handles.push(handle);
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Phase 2: Spike load
    for _ in 0..500 {
        let pool_clone = pool.clone();
        let active_clone = active_connections.clone();
        
        let handle = tokio::spawn(async move {
            match pool_clone.acquire().await {
                Ok(conn) => {
                    active_clone.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    active_clone.fetch_sub(1, Ordering::Relaxed);
                    drop(conn);
                    true
                },
                Err(_) => false,
            }
        });
        handles.push(handle);
    }
    
    // Wait for completion
    let mut successful = 0;
    for handle in handles {
        if handle.await.unwrap_or(false) {
            successful += 1;
        }
    }
    
    let peak_active = active_connections.load(Ordering::Relaxed);
    let stats = pool.stats();
    
    println!("Connection pool scaling results:");
    println!("  Successful acquisitions: {}", successful);
    println!("  Peak active connections: {}", peak_active);
    println!("  Pool size: {}", stats.pool_size);
    println!("  Total created: {}", stats.total_created);
    println!("  Total recycled: {}", stats.total_recycled);
    
    assert!(successful >= 500, "Should handle at least 500 connections");
    assert!(stats.total_recycled > 0, "Should recycle connections");
}

#[tokio::test]
async fn test_resource_utilization() {
    use sysinfo::{System, SystemExt, ProcessExt};
    use std::process;
    
    let pid = process::id();
    let mut system = System::new_all();
    system.refresh_all();
    
    let process = system.process(pid as i32).expect("Process not found");
    let baseline_memory = process.memory();
    let baseline_cpu = process.cpu_usage();
    
    println!("Baseline: Memory={} KB, CPU={:.2}%", baseline_memory, baseline_cpu);
    
    // Create many connections
    let path = "/test_resources";
    let listener = Arc::new(SharedMemoryListener::bind(path).expect("Failed to bind"));
    
    let mut handles = vec![];
    for i in 0..100 {
        let listener_clone = listener.clone();
        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener_clone.accept().await.unwrap();
            // Keep connection alive
            for _ in 0..100 {
                let data = vec![i as u8; 1024];
                let _ = stream.write_all(&data).await;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        handles.push(handle);
        
        // Client
        tokio::spawn(async move {
            if let Ok(mut stream) = SharedMemoryStream::connect(path).await {
                let mut buf = vec![0u8; 1024];
                for _ in 0..100 {
                    let _ = stream.read(&mut buf).await;
                }
            }
        });
    }
    
    // Let it run
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Measure under load
    system.refresh_all();
    let process = system.process(pid as i32).expect("Process not found");
    let load_memory = process.memory();
    let load_cpu = process.cpu_usage();
    
    println!("Under load: Memory={} KB, CPU={:.2}%", load_memory, load_cpu);
    
    // Calculate increases
    let memory_increase_mb = (load_memory - baseline_memory) as f64 / 1024.0;
    let cpu_increase = load_cpu - baseline_cpu;
    
    println!("Increases: Memory={:.2} MB, CPU={:.2}%", memory_increase_mb, cpu_increase);
    
    // Verify reasonable resource usage
    assert!(memory_increase_mb < 100.0, "Memory increase should be < 100 MB");
    assert!(cpu_increase < 80.0, "CPU increase should be < 80%");
    
    // Cleanup
    for handle in handles {
        handle.abort();
    }
}

#[tokio::test]
async fn test_message_routing_at_scale() {
    let mut codec = BinaryCodec::new();
    let message_types = vec![
        MessageType::CompletionRequest,
        MessageType::AskRequest,
        MessageType::EditRequest,
        MessageType::ChatMessage,
        MessageType::Heartbeat,
    ];
    
    let start = Instant::now();
    let mut total_routed = 0;
    
    for _ in 0..10000 {
        for msg_type in &message_types {
            let msg = Message {
                id: rand::random(),
                msg_type: *msg_type,
                payload: match msg_type {
                    MessageType::CompletionRequest => {
                        MessagePayload::CompletionRequest(CompletionRequest {
                            prompt: "test".to_string(),
                            model: "model".to_string(),
                            max_tokens: 10,
                            temperature: 0.5,
                            stream: false,
                        })
                    },
                    _ => MessagePayload::Heartbeat,
                },
                timestamp: 1234567890,
            };
            
            let encoded = codec.encode(&msg).unwrap();
            let decoded = codec.decode(&encoded).unwrap();
            
            // Simulate routing
            match decoded.msg_type {
                MessageType::CompletionRequest => total_routed += 1,
                MessageType::AskRequest => total_routed += 1,
                MessageType::EditRequest => total_routed += 1,
                MessageType::ChatMessage => total_routed += 1,
                MessageType::Heartbeat => total_routed += 1,
                _ => {},
            }
        }
    }
    
    let elapsed = start.elapsed();
    let msgs_per_sec = total_routed as f64 / elapsed.as_secs_f64();
    
    println!("Message routing at scale:");
    println!("  Total messages routed: {}", total_routed);
    println!("  Time elapsed: {:?}", elapsed);
    println!("  Messages/sec: {:.0}", msgs_per_sec);
    
    assert!(msgs_per_sec > 100_000.0, "Should route > 100K messages/sec");
}
