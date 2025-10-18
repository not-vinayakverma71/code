// Day 6: Comprehensive integration tests for core modules
use lapce_ai_rust::*;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[tokio::test]
async fn test_shm_cache_integration() {
    println!("\n=== Testing SharedMemory + Cache Integration ===");
    
    // Create components
    let mut shm = optimized_shared_memory::lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer::create("integration", 8 * 1024 * 1024).unwrap();
    let cache = optimized_cache::OptimizedCache::new();
    
    // Write data to SharedMemory
    let test_data = vec![0xAB; 1024];
    assert!(shm.write(&test_data));
    
    // Read from SharedMemory and cache it
    if let Some(data) = shm.read() {
        cache.set("shm_data".to_string(), data.clone()).await.unwrap();
        
        // Verify cache contains the data
        tokio::time::sleep(Duration::from_millis(20)).await; // Let buffer flush
        let cached = cache.get("shm_data").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), data);
    }
    
    println!("  ✓ SharedMemory + Cache integration works");
}

#[tokio::test]
async fn test_vector_cache_integration() {
    println!("\n=== Testing Vector Search + Cache Integration ===");
    
    let mut search = optimized_vector_search::OptimizedVectorSearch::new(128);
    let cache = optimized_cache::OptimizedCache::new();
    
    // Add vectors
    for i in 0..100 {
        let vec: Vec<f32> = (0..128).map(|j| ((i * j) as f32).sin()).collect();
        search.add_vector(vec.clone(), format!("doc_{}", i)).unwrap();
        
        // Cache the vector
        let vec_bytes: Vec<u8> = vec.iter().flat_map(|f| f.to_le_bytes()).collect();
        cache.set(format!("vec_{}", i), vec_bytes).await.unwrap();
    }
    
    // Search and cache results
    let query: Vec<f32> = (0..128).map(|i| (i as f32 * 0.01).cos()).collect();
    let results = search.search(&query, 10);
    
    // Cache search results
    for (idx, score, metadata) in &results {
        let result_data = format!("{}:{}", score, metadata).into_bytes();
        cache.set(format!("result_{}", idx), result_data).await.unwrap();
    }
    
    tokio::time::sleep(Duration::from_millis(20)).await;
    
    // Verify cached results
    let cached_result = cache.get("result_0").await;
    assert!(cached_result.is_some());
    
    println!("  ✓ Vector Search + Cache integration works");
}

#[tokio::test]
async fn test_connection_pool_shm() {
    println!("\n=== Testing Connection Pool + SharedMemory ===");
    
    let config = working_connection_pool::ConnectionConfig {
        max_connections: 10,
        min_connections: 2,
        ..Default::default()
    };
    
    let pool = working_connection_pool::WorkingConnectionPool::new(config).await.unwrap();
    
    // Multiple connections writing to SharedMemory
    let mut handles = vec![];
    
    for i in 0..5 {
        let pool_clone = pool.clone();
        handles.push(tokio::spawn(async move {
            let conn = pool_clone.acquire().await.unwrap();
            
            // Each connection writes to SharedMemory
            let data = vec![i as u8; 256];
            let success = conn.write(&data).await;
            
            pool_clone.release(conn).await;
            success
        }));
    }
    
    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap() {
            success_count += 1;
        }
    }
    
    assert_eq!(success_count, 5);
    println!("  ✓ Connection Pool + SharedMemory integration works");
}

#[tokio::test]
async fn test_full_stack_integration() {
    println!("\n=== Testing Full Stack Integration ===");
    
    // All components together
    let mut shm = optimized_shared_memory::lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer::create("full_stack", 16 * 1024 * 1024).unwrap();
    let cache = optimized_cache::OptimizedCache::new();
    let mut search = optimized_vector_search::OptimizedVectorSearch::new(256);
    let embeddings = minilm_embeddings::MiniLMEmbeddings::new().await.unwrap();
    
    // Simulate real workflow
    let documents = vec![
        "Rust provides memory safety without garbage collection",
        "Python is great for data science and machine learning",
        "JavaScript runs in browsers and Node.js servers",
    ];
    
    // Process documents
    for (idx, doc) in documents.iter().enumerate() {
        // Generate embedding
        let embedding = embeddings.embed(doc);
        
        // Add to vector search
        search.add_vector(embedding.clone(), doc.to_string()).unwrap();
        
        // Write to SharedMemory
        let data = doc.as_bytes();
        assert!(shm.write(data));
        
        // Cache the document
        cache.set(format!("doc_{}", idx), data.to_vec()).await.unwrap();
    }
    
    // Query the system
    let query = "memory management and safety";
    let query_embedding = embeddings.embed(query);
    
    // Search
    let results = search.search(&query_embedding, 3);
    assert_eq!(results.len(), 3);
    assert!(results[0].2.contains("Rust")); // Best match should be Rust doc
    
    // Verify cache
    tokio::time::sleep(Duration::from_millis(20)).await;
    let cached_doc = cache.get("doc_0").await;
    assert!(cached_doc.is_some());
    
    println!("  ✓ Full stack integration works");
}

#[test]
fn test_concurrent_operations() {
    println!("\n=== Testing Concurrent Operations ===");
    
    use std::thread;
    
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    runtime.block_on(async {
        let shm = Arc::new(Mutex::new(
            optimized_shared_memory::lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer::create("concurrent", 32 * 1024 * 1024).unwrap()
        ));
        let cache = Arc::new(optimized_cache::OptimizedCache::new());
        
        let mut handles = vec![];
        
        // Spawn multiple async tasks
        for i in 0..10 {
            let shm_clone = shm.clone();
            let cache_clone = cache.clone();
            
            handles.push(tokio::spawn(async move {
                let data = vec![i as u8; 1024];
                
                // Write to SharedMemory
                {
                    let mut shm_guard = shm_clone.lock().await;
                    shm_guard.write(&data);
                }
                
                // Write to cache
                cache_clone.set(format!("task_{}", i), data).await.unwrap();
                
                // Read back from cache
                tokio::time::sleep(Duration::from_millis(10)).await;
                cache_clone.get(&format!("task_{}", i)).await.is_some()
            }));
        }
        
        let mut success_count = 0;
        for handle in handles {
            if handle.await.unwrap() {
                success_count += 1;
            }
        }
        
        assert_eq!(success_count, 10);
        println!("  ✓ Concurrent operations work correctly");
    });
}

#[test]
fn test_performance_targets() {
    println!("\n=== Verifying Performance Targets ===");
    
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    runtime.block_on(async {
        // Test SharedMemory throughput
        let mut shm = optimized_shared_memory::lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer::create("perf", 16 * 1024 * 1024).unwrap();
        let data = vec![0xFF; 256];
        
        let start = Instant::now();
        let iterations = 100_000;
        for _ in 0..iterations {
            shm.write(&data);
            shm.read();
        }
        let elapsed = start.elapsed();
        let throughput = iterations as f64 / elapsed.as_secs_f64();
        
        println!("  SharedMemory: {:.2}M msg/sec", throughput / 1_000_000.0);
        assert!(throughput > 1_000_000.0, "SharedMemory below 1M msg/sec");
        
        // Test Cache performance
        let cache = optimized_cache::OptimizedCache::new();
        let cache_start = Instant::now();
        for i in 0..10_000 {
            cache.set(format!("k{}", i), vec![i as u8; 64]).await.unwrap();
        }
        let cache_elapsed = cache_start.elapsed();
        let cache_ops = 10_000.0 / cache_elapsed.as_secs_f64();
        
        println!("  Cache: {:.0} writes/sec", cache_ops);
        assert!(cache_ops > 100_000.0, "Cache below 100K writes/sec");
        
        // Test Vector Search latency
        let mut search = optimized_vector_search::OptimizedVectorSearch::new(128);
        for i in 0..1000 {
            let vec: Vec<f32> = (0..128).map(|j| ((i * j) as f32).sin()).collect();
            search.add_vector(vec, format!("doc_{}", i)).unwrap();
        }
        
        let query: Vec<f32> = (0..128).map(|i| (i as f32 * 0.01).cos()).collect();
        let search_start = Instant::now();
        for _ in 0..1000 {
            search.search(&query, 10);
        }
        let search_elapsed = search_start.elapsed();
        let avg_latency_us = search_elapsed.as_micros() as f64 / 1000.0;
        
        println!("  Vector Search: {:.1}μs avg latency", avg_latency_us);
        assert!(avg_latency_us < 1000.0, "Vector search too slow");
        
        println!("\n  ✅ All performance targets met!");
    });
}
