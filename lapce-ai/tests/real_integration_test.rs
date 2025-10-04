// REAL integration test - actually test things work
use lapce_ai_rust::working_shared_memory::WorkingSharedMemory;
use lapce_ai_rust::working_cache_system::WorkingCacheSystem;
use lapce_ai_rust::vector_search::VectorSearch;

#[test]
fn test_shared_memory_actually_works() {
    let mut shm = WorkingSharedMemory::create("test", 1024 * 1024).unwrap();
    
    // Test write
    let data = vec![1, 2, 3, 4, 5];
    assert!(shm.write(&data));
    
    // Test read
    let read_data = shm.read().unwrap();
    assert_eq!(read_data, data);
    
    // Benchmark throughput
    let start = std::time::Instant::now();
    let iterations = 100_000;
    for _ in 0..iterations {
        shm.write(&data);
    }
    let elapsed = start.elapsed();
    let throughput = iterations as f64 / elapsed.as_secs_f64();
    println!("REAL SharedMemory throughput: {:.2} msg/sec", throughput);
    
    // Check if we actually meet the target
    assert!(throughput > 1_000_000.0, "Failed to meet 1M msg/sec target. Got: {}", throughput);
}

#[tokio::test]
async fn test_cache_actually_works() {
    let cache = WorkingCacheSystem::new().await.unwrap();
    
    // Test set/get
    let key = "test_key";
    let value = vec![1, 2, 3, 4, 5];
    
    cache.set(key, value.clone()).await.unwrap();
    let retrieved = cache.get(key).await.unwrap();
    assert_eq!(retrieved, value);
    
    // Benchmark
    let start = std::time::Instant::now();
    for i in 0..1000 {
        cache.set(&format!("key_{}", i), vec![i as u8; 64]).await.unwrap();
    }
    let elapsed = start.elapsed();
    let ops_per_sec = 1000.0 / elapsed.as_secs_f64();
    println!("REAL Cache throughput: {:.2} ops/sec", ops_per_sec);
    
    assert!(ops_per_sec > 10_000.0, "Cache too slow: {} ops/sec", ops_per_sec);
}

#[test]
fn test_vector_search_actually_works() {
    let mut search = VectorSearch::new(128);
    
    // Add vectors
    for i in 0..100 {
        let vec: Vec<f32> = (0..128).map(|j| (i * j) as f32 * 0.001).collect();
        search.add_vector(vec, format!("doc_{}", i)).unwrap();
    }
    
    // Search
    let query: Vec<f32> = (0..128).map(|i| i as f32 * 0.01).collect();
    let results = search.search(&query, 10);
    
    assert_eq!(results.len(), 10);
    assert!(results[0].1 > 0.0); // Has similarity score
    
    // Benchmark search latency
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        search.search(&query, 10);
    }
    let elapsed = start.elapsed();
    let avg_latency_ms = elapsed.as_millis() as f64 / 1000.0;
    println!("REAL Vector search latency: {:.2}ms", avg_latency_ms);
    
    assert!(avg_latency_ms < 10.0, "Search too slow: {}ms", avg_latency_ms);
}

#[tokio::test] 
async fn test_connection_pool_actually_works() {
    use lapce_ai_rust::working_connection_pool::{WorkingConnectionPool, ConnectionConfig};
    
    let config = ConnectionConfig {
        max_connections: 100,
        min_connections: 10,
        ..Default::default()
    };
    
    let pool = WorkingConnectionPool::new(config).await.unwrap();
    
    // Test acquiring connections
    let mut connections = vec![];
    for _ in 0..50 {
        let conn = pool.acquire().await.unwrap();
        connections.push(conn);
    }
    
    assert_eq!(connections.len(), 50);
    
    // Release all
    for conn in connections {
        pool.release(conn).await;
    }
    
    let stats = pool.stats().await;
    println!("REAL Pool stats - acquired: {}, released: {}", stats.total_acquired, stats.total_released);
    assert_eq!(stats.total_acquired, 50);
    assert_eq!(stats.total_released, 50);
}

#[test]
fn test_memory_usage() {
    // Get initial memory
    let initial_memory = get_memory_usage_kb();
    
    // Create large shared memory
    let _shm = WorkingSharedMemory::create("mem_test", 64 * 1024 * 1024).unwrap();
    
    // Check memory after
    let after_memory = get_memory_usage_kb();
    let used_kb = after_memory - initial_memory;
    let used_mb = used_kb as f64 / 1024.0;
    
    println!("REAL Memory usage: {:.2}MB", used_mb);
    
    // We said <3MB but that's for core functionality, not including 64MB buffer
    // Check reasonable usage
    assert!(used_mb < 100.0, "Memory usage too high: {}MB", used_mb);
}

fn get_memory_usage_kb() -> i64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse().unwrap_or(0);
            }
        }
    }
    0
}
