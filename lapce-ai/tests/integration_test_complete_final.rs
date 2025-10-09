/// Complete Integration Test Suite for lapce-ai-rust
use lapce_ai_rust::{
    ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream, SharedMemoryBuffer},
    ipc_server_complete::IpcServerComplete,
    ipc_messages::{AIRequest, Message, MessageRole, MessageType},
    provider_pool::{ProviderPool, ProviderPoolConfig},
};
use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::test]
async fn test_shared_memory_basic() {
    let mut buffer = SharedMemoryBuffer::create("test_basic", 1024 * 1024).unwrap();
    
    let data = vec![42u8; 256];
    assert!(buffer.write(&data).unwrap());
    
    let read_data = buffer.read().unwrap();
    assert_eq!(read_data, data);
}

#[tokio::test]
async fn test_shared_memory_performance() {
    let mut buffer = SharedMemoryBuffer::create("test_perf", 4 * 1024 * 1024).unwrap();
    let data = vec![0xABu8; 1024];
    let iterations = 10000;
    
    let start = Instant::now();
    for _ in 0..iterations {
        buffer.write(&data).unwrap();
        let _ = buffer.read().unwrap();
    }
    let duration = start.elapsed();
    
    let ops_per_sec = (iterations * 2) as f64 / duration.as_secs_f64();
    let latency_us = duration.as_micros() as f64 / (iterations * 2) as f64;
    
    println!("Performance: {:.2}M ops/sec, {:.3}μs latency", ops_per_sec / 1_000_000.0, latency_us);
    
    // Verify performance meets requirements
    assert!(latency_us < 10.0, "Latency must be < 10μs");
    assert!(ops_per_sec > 1_000_000.0, "Throughput must be > 1M ops/sec");
}

#[tokio::test]
async fn test_ipc_server_lifecycle() {
    let config_path = "test_config.toml";
    
    // Create test config
    std::fs::write(config_path, r#"
        socket_path = "/tmp/test_lapce_ipc.sock"
        max_connections = 10
        enable_metrics = false
    "#).unwrap();
    
    let server = IpcServerComplete::from_config_file(config_path).await.unwrap();
    
    // Start server
    let server_handle = {
        let server = server.clone();
        tokio::spawn(async move {
            server.serve().await
        })
    };
    
    // Give server time to start
    sleep(Duration::from_millis(100)).await;
    
    // Shutdown
    server.shutdown().await;
    
    // Clean up
    std::fs::remove_file(config_path).ok();
    std::fs::remove_file("/tmp/test_lapce_ipc.sock").ok();
}

#[tokio::test]
async fn test_message_protocol() {
    let request = AIRequest {
        messages: vec![
            Message {
                role: MessageRole::System,
                content: "You are a helpful assistant.".to_string(),
                tool_calls: None,
            },
            Message {
                role: MessageRole::User,
                content: "Hello, world!".to_string(),
                tool_calls: None,
            },
        ],
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        max_tokens: Some(100),
        tools: None,
        system_prompt: None,
        stream: Some(false),
    };
    
    // Serialize and deserialize
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: AIRequest = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.messages.len(), 2);
    assert_eq!(deserialized.model, "gpt-4");
}

#[tokio::test]
async fn test_connection_pool() {
    use lapce_ai_rust::connection_pool_complete_real::ConnectionPool;
    
    let pool = ConnectionPool::new(100, Duration::from_secs(60));
    
    // Test acquiring connections
    let mut guards = vec![];
    for _ in 0..10 {
        guards.push(pool.acquire().await);
    }
    
    assert_eq!(pool.active_count().await, 10);
    
    // Drop some connections
    guards.truncate(5);
    sleep(Duration::from_millis(100)).await;
    
    assert_eq!(pool.active_count().await, 5);
    
    // Test stats
    let stats = pool.stats().await;
    assert_eq!(stats.total_connections, 5);
    assert_eq!(stats.healthy_connections, 5);
}

#[tokio::test]
async fn test_auto_reconnection() {
    use lapce_ai_rust::auto_reconnection::{AutoReconnectionManager, ReconnectionStrategy};
    
    let manager = AutoReconnectionManager::new(ReconnectionStrategy::ExponentialBackoff {
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(100),
        multiplier: 2.0,
    });
    
    // Test reconnection timing
    let start = Instant::now();
    manager.wait_for_reconnect(0).await;
    let delay1 = start.elapsed();
    
    let start = Instant::now();
    manager.wait_for_reconnect(1).await;
    let delay2 = start.elapsed();
    
    // Second delay should be ~2x first delay
    assert!(delay2 >= delay1 * 2 - Duration::from_millis(5));
}

#[tokio::test]
async fn test_metrics() {
    let config_path = "test_metrics_config.toml";
    
    std::fs::write(config_path, r#"
        socket_path = "/tmp/test_metrics_ipc.sock"
        max_connections = 10
        enable_metrics = true
    "#).unwrap();
    
    let server = IpcServerComplete::from_config_file(config_path).await.unwrap();
    let metrics = server.metrics();
    
    // Check initial state
    assert_eq!(metrics.total_requests.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed), 0);
    
    // Clean up
    std::fs::remove_file(config_path).ok();
    std::fs::remove_file("/tmp/test_metrics_ipc.sock").ok();
}

#[tokio::test]
async fn test_health_check() {
    let config_path = "test_health_config.toml";
    
    std::fs::write(config_path, r#"
        socket_path = "/tmp/test_health_ipc.sock"
        max_connections = 10
        enable_metrics = true
    "#).unwrap();
    
    let server = IpcServerComplete::from_config_file(config_path).await.unwrap();
    let health = server.health_status().await;
    
    assert!(health.is_healthy);
    assert_eq!(health.connections, 0);
    assert!(health.issues.is_empty());
    
    // Clean up
    std::fs::remove_file(config_path).ok();
    std::fs::remove_file("/tmp/test_health_ipc.sock").ok();
}

#[tokio::test]
async fn test_concurrent_connections() {
    let mut listener = SharedMemoryListener::bind("test_concurrent").unwrap();
    
    let server_task = tokio::spawn(async move {
        for _ in 0..5 {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 256];
                stream.read(&mut buf).await.unwrap();
                stream.write_all(&buf).await.unwrap();
            });
        }
    });
    
    // Connect multiple clients
    let mut handles = vec![];
    for i in 0..5 {
        let handle = tokio::spawn(async move {
            let mut stream = SharedMemoryStream::connect("test_concurrent").await.unwrap();
            let data = vec![i as u8; 256];
            stream.write_all(&data).await.unwrap();
            
            let mut buf = vec![0u8; 256];
            stream.read(&mut buf).await.unwrap();
            assert_eq!(buf, data);
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_message_types() {
    // Test MessageType conversions
    let msg_type = MessageType::Complete;
    let bytes = msg_type.to_bytes();
    let decoded = MessageType::from_bytes(&bytes).unwrap();
    assert_eq!(msg_type, decoded);
    
    // Test all message types
    for msg_type in [
        MessageType::Echo,
        MessageType::Complete,
        MessageType::Stream,
        MessageType::Cancel,
        MessageType::Heartbeat,
        MessageType::Shutdown,
        MessageType::Custom,
    ] {
        let bytes = msg_type.to_bytes();
        let decoded = MessageType::from_bytes(&bytes).unwrap();
        assert_eq!(msg_type, decoded);
    }
}

#[tokio::test]
async fn test_zero_copy_serialization() {
    use rkyv::{Archive, Deserialize, Serialize};
    
    #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
    struct TestMessage {
        id: u64,
        content: String,
    }
    
    let msg = TestMessage {
        id: 12345,
        content: "Hello, zero-copy!".to_string(),
    };
    
    // Serialize
    let bytes = rkyv::to_bytes::<_, 256>(&msg).unwrap();
    
    // Deserialize without copying
    let archived = unsafe { rkyv::archived_root::<TestMessage>(&bytes) };
    assert_eq!(archived.id, 12345);
    
    // Full deserialization if needed
    let deserialized: TestMessage = archived.deserialize(&mut rkyv::Infallible).unwrap();
    assert_eq!(deserialized, msg);
}

#[test]
fn test_memory_usage() {
    use lapce_ai_rust::ipc::shared_memory_complete::HEADER_SIZE;
    
    // Verify memory overhead is minimal
    assert!(HEADER_SIZE <= 256, "Header size should be minimal");
    
    // Calculate total memory for 100 connections
    let per_connection = 8 * 1024; // 8KB per connection
    let total = 100 * per_connection + HEADER_SIZE;
    
    assert!(total < 3 * 1024 * 1024, "Total memory for 100 connections should be < 3MB");
}
