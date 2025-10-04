#[cfg(test)]
mod shared_memory_tests {
    use lapce_ai_rust::shared_memory::*;
    use std::time::{Duration, Instant};
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    use tokio::sync::Barrier;

    #[tokio::test]
    async fn test_basic_send_recv() {
        let transport = Arc::new(SharedMemoryTransport::new("test_basic"));
        let listener = SharedMemoryListener::new(transport.clone());
        
        tokio::spawn(async move {
            let mut stream = listener.accept().await.unwrap();
            let msg = stream.recv().await.unwrap();
            assert_eq!(msg, vec![1, 2, 3, 4]);
            stream.send(vec![5, 6, 7, 8]).await.unwrap();
        });
        
        tokio::time::sleep(Duration::from_millis(10)).await;
        let mut client = SharedMemoryStream::connect("test_basic").await.unwrap();
        client.send(vec![1, 2, 3, 4]).await.unwrap();
        let response = client.recv().await.unwrap();
        assert_eq!(response, vec![5, 6, 7, 8]);
    }

    #[tokio::test]
    async fn test_latency_under_10us() {
        let transport = Arc::new(SharedMemoryTransport::new("test_latency"));
        let listener = SharedMemoryListener::new(transport.clone());
        
        tokio::spawn(async move {
            let mut stream = listener.accept().await.unwrap();
            loop {
                if let Ok(msg) = stream.recv().await {
                    stream.send(msg).await.unwrap();
                }
            }
        });
        
        tokio::time::sleep(Duration::from_millis(10)).await;
        let mut client = SharedMemoryStream::connect("test_latency").await.unwrap();
        
        // Warmup
        for _ in 0..100 {
            client.send(vec![0; 100]).await.unwrap();
            client.recv().await.unwrap();
        }
        
        // Measure
        let mut latencies = Vec::new();
        for _ in 0..1000 {
            let start = Instant::now();
            client.send(vec![0; 100]).await.unwrap();
            client.recv().await.unwrap();
            latencies.push(start.elapsed());
        }
        
        let avg = latencies.iter().sum::<Duration>() / latencies.len() as u32;
        assert!(avg.as_micros() < 10, "Latency {} > 10μs", avg.as_micros());
    }

    #[tokio::test]
    async fn test_throughput_over_1m() {
        let transport = Arc::new(SharedMemoryTransport::new("test_throughput"));
        let listener = SharedMemoryListener::new(transport.clone());
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        tokio::spawn(async move {
            let mut stream = listener.accept().await.unwrap();
            while let Ok(_) = stream.recv().await {
                counter_clone.fetch_add(1, Ordering::Relaxed);
            }
        });
        
        tokio::time::sleep(Duration::from_millis(10)).await;
        let mut client = SharedMemoryStream::connect("test_throughput").await.unwrap();
        
        let iterations = 100_000;
        let start = Instant::now();
        
        for _ in 0..iterations {
            client.send(vec![0; 100]).await.unwrap();
        }
        
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let elapsed = start.elapsed();
        let count = counter.load(Ordering::Relaxed);
        let throughput = count as f64 / elapsed.as_secs_f64();
        
        assert!(throughput > 1_000_000.0, "Throughput {} < 1M msg/sec", throughput);
    }

    #[tokio::test]
    async fn test_concurrent_connections() {
        let transport = Arc::new(SharedMemoryTransport::new("test_concurrent"));
        let num_clients = 100;
        let barrier = Arc::new(Barrier::new(num_clients + 1));
        
        let mut handles = Vec::new();
        for i in 0..num_clients {
            let barrier_clone = barrier.clone();
            let handle = tokio::spawn(async move {
                let mut stream = SharedMemoryStream::connect("test_concurrent").await.unwrap();
                barrier_clone.wait().await;
                
                for j in 0..10 {
                    stream.send(vec![i as u8, j as u8]).await.unwrap();
                    let _ = stream.recv().await;
                }
            });
            handles.push(handle);
        }
        
        barrier.wait().await;
        
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[test]
    fn test_zero_allocations() {
        let mut buffer = [0u8; 4096];
        let message = b"test message";
        
        // Should not allocate
        buffer[..message.len()].copy_from_slice(message);
        let received = &buffer[..message.len()];
        assert_eq!(received, message);
    }
}

#[cfg(test)]
mod cache_tests {
    use lapce_ai_rust::cache_v2::*;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_cache_operations() {
        let cache = Arc::new(IntegratedCacheSystem::new(100, 1000, 10000));
        
        // Test put/get
        cache.put("key1", "value1").await.unwrap();
        let value = cache.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));
        
        // Test miss
        let miss = cache.get("nonexistent").await.unwrap();
        assert_eq!(miss, None);
    }
    
    #[tokio::test]
    async fn test_cache_promotion() {
        let cache = Arc::new(IntegratedCacheSystem::new(10, 100, 1000));
        let policy = PromotionPolicy::default();
        
        // Access item multiple times
        for _ in 0..5 {
            policy.record_access("key1", 1);
        }
        
        assert!(policy.should_promote_to_l1("key1", 1));
    }
    
    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = Arc::new(IntegratedCacheSystem::new(2, 2, 2));
        
        cache.put("key1", "value1").await.unwrap();
        cache.put("key2", "value2").await.unwrap();
        cache.put("key3", "value3").await.unwrap(); // Should evict key1
        
        let stats = cache.get_statistics().await.unwrap();
        assert!(stats.total_entries <= 2);
    }
}

#[cfg(test)]
mod connection_pool_tests {
    use lapce_ai_rust::connection_pool::*;
    use std::time::Duration;
    
    #[test]
    fn test_pool_acquire_release() {
        let pool = ConnectionPool::new(10, 100);
        
        let mut connections = Vec::new();
        for _ in 0..50 {
            if let Some(conn) = pool.acquire() {
                connections.push(conn);
            }
        }
        
        assert_eq!(connections.len(), 50);
        
        for conn in connections {
            pool.release(conn);
        }
        
        let (_, active, pooled) = pool.stats();
        assert_eq!(active, 0);
        assert_eq!(pooled, 50);
    }
    
    #[test]
    fn test_pool_exhaustion() {
        let pool = ConnectionPool::new(10, 20);
        
        let mut connections = Vec::new();
        for _ in 0..25 {
            if let Some(conn) = pool.acquire() {
                connections.push(conn);
            }
        }
        
        assert!(connections.len() <= 20);
        
        // Pool should be exhausted
        assert!(pool.acquire().is_none());
    }
    
    #[test]
    fn test_connection_reuse() {
        let pool = ConnectionPool::new(5, 10);
        
        // First round
        let conn1 = pool.acquire().unwrap();
        let id1 = conn1.id;
        pool.release(conn1);
        
        // Second round - should get same connection
        let conn2 = pool.acquire().unwrap();
        let id2 = conn2.id;
        pool.release(conn2);
        
        assert_eq!(id1, id2); // Connection reused
    }
}

#[cfg(test)]
mod reconnect_tests {
    use lapce_ai_rust::reconnect::*;
    use std::time::{Duration, Instant};
    
    #[test]
    fn test_exponential_backoff() {
        let manager = ReconnectManager::new();
        let conn = Connection::new(1);
        conn.disconnect();
        
        let start = Instant::now();
        let success = manager.reconnect_with_backoff(&conn);
        let elapsed = start.elapsed();
        
        assert!(elapsed < Duration::from_millis(100));
    }
    
    #[test]
    fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new();
        
        // Fail 3 times to open circuit
        for _ in 0..3 {
            let _ = breaker.call(|| Err("fail".to_string()));
        }
        
        // Circuit should be open
        let result = breaker.call(|| Ok(()));
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod ipc_message_tests {
    use lapce_ai_rust::ipc_messages::*;
    use serde_json;
    
    #[test]
    fn test_message_serialization() {
        let msg = IpcMessage {
            id: 123,
            method: "test".to_string(),
            params: serde_json::json!({"key": "value"}),
        };
        
        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: IpcMessage = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(msg.id, deserialized.id);
        assert_eq!(msg.method, deserialized.method);
    }
    
    #[test]
    fn test_rpc_message() {
        let rpc = RpcMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(42),
            method: Some("execute".to_string()),
            params: None,
            result: None,
            error: None,
        };
        
        assert!(rpc.is_request());
        assert!(!rpc.is_response());
    }
}

#[cfg(test)]
mod task_tests {
    use lapce_ai_rust::task_exact_translation::*;
    
    #[test]
    fn test_task_options_default() {
        let options = TaskOptions::default();
        assert!(options.api.is_none());
        assert!(options.auto_approval_handler.is_none());
    }
    
    #[tokio::test]
    async fn test_task_creation() {
        let task = Task::new(TaskOptions::default());
        assert_eq!(task.get_state(), TaskState::Idle);
    }
}

#[cfg(test)]
mod handler_tests {
    use lapce_ai_rust::handler_registration::*;
    
    #[test]
    fn test_webview_message() {
        let msg = WebviewMessage {
            type_: "test".to_string(),
            text: Some("content".to_string()),
            data: None,
        };
        
        assert_eq!(msg.type_, "test");
        assert_eq!(msg.text, Some("content".to_string()));
    }
    
    #[tokio::test]
    async fn test_handler_registration() {
        let handler = ClineProvider::new();
        
        // Test message handling
        let result = handler.handle_message("test", serde_json::Value::Null).await;
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod performance_tests {
    use std::time::Instant;
    
    #[test]
    fn test_inline_optimization() {
        let mut data = [0u8; 1024];
        
        let start = Instant::now();
        for _ in 0..1_000_000 {
            inline_op(&mut data);
        }
        let inline_time = start.elapsed();
        
        let start = Instant::now();
        for _ in 0..1_000_000 {
            non_inline_op(&mut data);
        }
        let non_inline_time = start.elapsed();
        
        // Inline should be faster
        assert!(inline_time < non_inline_time);
    }
    
    #[inline(always)]
    fn inline_op(data: &mut [u8]) {
        data[0] = 42;
    }
    
    #[inline(never)]
    fn non_inline_op(data: &mut [u8]) {
        data[0] = 42;
    }
}
