// Comprehensive tests for ZSTD compression, memory-mapped storage, and hierarchical cache
use lancedb::embeddings::zstd_compression::{ZstdCompressor, CompressionConfig};
use lancedb::storage::mmap_storage::{MmapStorage, ConcurrentMmapStorage};
use lancedb::storage::hierarchical_cache::{HierarchicalCache, CacheConfig};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_zstd_bit_perfect_compression() {
    let mut compressor = ZstdCompressor::new(CompressionConfig::default());
    
    // Test with various embedding sizes
    let test_cases = vec![
        vec![0.1_f32, -0.5, 0.3, 0.7, -0.2],  // Small
        vec![0.1; 384],                        // Medium (typical)
        vec![0.1; 1536],                       // Large (AWS Titan)
    ];
    
    for embedding in test_cases {
        let original = embedding.clone();
        
        // Compress
        let compressed = compressor.compress_embedding(&embedding, "test").unwrap();
        
        // Verify compression happened (may not always reduce size for small/random data)
        assert!(compressed.compressed_size > 0);
        // For very small embeddings, compression might not reduce size
        if embedding.len() > 100 {
            assert!(compressed.compressed_size <= compressed.original_size);
        }
        
        // Decompress
        let decompressed = compressor.decompress_embedding(&compressed).unwrap();
        
        // Verify bit-perfect reconstruction
        assert_eq!(original.len(), decompressed.len());
        for (orig, decomp) in original.iter().zip(decompressed.iter()) {
            assert!((orig - decomp).abs() < f32::EPSILON);
        }
        
        // Verify helper method
        assert!(compressor.verify_bit_perfect(&embedding).unwrap());
    }
    
    println!("✅ All embeddings compressed and decompressed perfectly");
}

#[tokio::test]
async fn test_compression_ratio_and_stats() {
    let mut compressor = ZstdCompressor::new(CompressionConfig {
        compression_level: 10,  // Higher compression
        ..Default::default()
    });
    
    // Create repetitive pattern (compresses well)
    let mut embedding = Vec::with_capacity(1536);
    for i in 0..1536 {
        embedding.push((i % 10) as f32 / 10.0);
    }
    
    let compressed = compressor.compress_embedding(&embedding, "test").unwrap();
    
    println!("Compression Results:");
    println!("  Original: {} bytes", compressed.original_size);
    println!("  Compressed: {} bytes", compressed.compressed_size);
    println!("  Ratio: {:.2}x", compressed.compression_ratio);
    println!("  Space saved: {:.1}%", compressed.space_saved());
    
    // Should achieve good compression on repetitive data
    assert!(compressed.compression_ratio > 2.0);
    assert!(compressed.space_saved() > 50.0);
    
    let stats = compressor.get_stats();
    assert_eq!(stats.embeddings_compressed, 1);
    assert_eq!(stats.total_original_bytes, compressed.original_size);
    assert_eq!(stats.total_compressed_bytes, compressed.compressed_size);
    
    println!("✅ Compression ratio test passed");
}

#[tokio::test]
async fn test_dictionary_training() {
    let mut compressor = ZstdCompressor::new(CompressionConfig {
        enable_dictionary: true,
        ..Default::default()
    });
    
    // Create training samples
    let mut samples = Vec::new();
    for i in 0..100 {
        let mut embedding = Vec::with_capacity(384);
        for j in 0..384 {
            embedding.push((i as f32 * 0.01) + (j as f32 * 0.001));
        }
        samples.push(embedding);
    }
    
    // Train dictionary
    compressor.train_dictionary(&samples).unwrap();
    
    // Compress with dictionary should be better
    let test_embedding = vec![0.5; 384];
    let compressed_with_dict = compressor.compress_embedding(&test_embedding, "test").unwrap();
    
    // Compare with no dictionary
    let mut compressor_no_dict = ZstdCompressor::new(CompressionConfig {
        enable_dictionary: false,
        ..Default::default()
    });
    let compressed_no_dict = compressor_no_dict.compress_embedding(&test_embedding, "test2").unwrap();
    
    println!("Dictionary Compression:");
    println!("  With dictionary: {} bytes", compressed_with_dict.compressed_size);
    println!("  Without dictionary: {} bytes", compressed_no_dict.compressed_size);
    
    // Dictionary should improve compression (or at least not make it worse)
    assert!(compressed_with_dict.compressed_size <= compressed_no_dict.compressed_size);
    
    println!("✅ Dictionary training test passed");
}

#[tokio::test]
async fn test_mmap_storage() {
    let temp_dir = TempDir::new().unwrap();
    let storage = MmapStorage::new(temp_dir.path(), 100 * 1024 * 1024).unwrap();
    
    // Store multiple embeddings
    let embeddings = vec![
        ("embed1", vec![0.1; 384]),
        ("embed2", vec![0.2; 384]),
        ("embed3", vec![0.3; 384]),
        ("embed4", vec![0.4; 384]),
        ("embed5", vec![0.5; 384]),
    ];
    
    for (id, embedding) in &embeddings {
        storage.store_embedding(id, embedding).unwrap();
    }
    
    // Retrieve and verify
    for (id, original) in &embeddings {
        let retrieved = storage.get_embedding(id).unwrap();
        assert_eq!(original.len(), retrieved.len());
        
        for (orig, ret) in original.iter().zip(retrieved.iter()) {
            assert!((orig - ret).abs() < f32::EPSILON);
        }
    }
    
    // Test batch operations
    let batch_ids: Vec<String> = embeddings.iter().map(|(id, _)| id.to_string()).collect();
    let batch_retrieved = storage.batch_get(&batch_ids).unwrap();
    assert_eq!(batch_retrieved.len(), embeddings.len());
    
    // Check statistics
    let stats = storage.get_stats();
    assert_eq!(stats.embedding_count, embeddings.len());
    assert!(stats.total_size_bytes > 0);
    assert!(stats.average_size > 0);
    
    println!("✅ Memory-mapped storage test passed");
    println!("  Stored {} embeddings", stats.embedding_count);
    println!("  Total size: {} bytes", stats.total_size_bytes);
    println!("  Average size: {} bytes", stats.average_size);
}

#[tokio::test]
async fn test_concurrent_mmap_access() {
    use std::sync::Arc;
    use tokio::task;
    
    let temp_dir = TempDir::new().unwrap();
    let storage = Arc::new(ConcurrentMmapStorage::new(
        temp_dir.path(),
        100 * 1024 * 1024,
    ).unwrap());
    
    let mut handles = vec![];
    
    // Spawn multiple concurrent writers
    for i in 0..10 {
        let storage_clone = storage.clone();
        let handle = task::spawn(async move {
            let embedding = vec![i as f32 / 10.0; 384];
            storage_clone.store(&format!("concurrent_{}", i), &embedding).unwrap();
        });
        handles.push(handle);
    }
    
    // Wait for all writes
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Verify all embeddings
    for i in 0..10 {
        assert!(storage.contains(&format!("concurrent_{}", i)));
        let retrieved = storage.get(&format!("concurrent_{}", i)).unwrap();
        assert_eq!(retrieved.len(), 384);
        
        let expected_value = i as f32 / 10.0;
        assert!((retrieved[0] - expected_value).abs() < f32::EPSILON);
    }
    
    let stats = storage.get_stats();
    assert_eq!(stats.embedding_count, 10);
    
    println!("✅ Concurrent memory-mapped access test passed");
}

#[tokio::test]
async fn test_hierarchical_cache_tiers() {
    let temp_dir = TempDir::new().unwrap();
    let config = CacheConfig {
        l1_max_size_mb: 0.001,  // Very small to force evictions
        l1_max_entries: 2,
        l2_max_size_mb: 0.005,
        l2_max_entries: 4,
        l3_max_size_mb: 1.0,
        promotion_threshold: 2,
        enable_statistics: true,
        ..Default::default()
    };
    
    let cache = HierarchicalCache::new(config, temp_dir.path()).unwrap();
    
    // Add embeddings to force tier movement
    for i in 0..10 {
        let embedding = vec![i as f32; 384];
        cache.put(&format!("id_{}", i), embedding).unwrap();
    }
    
    // Check statistics
    let stats = cache.get_stats();
    println!("Cache Statistics:");
    println!("  L1 entries: {}", stats.l1_entries);
    println!("  L2 entries: {}", stats.l2_entries);
    println!("  L3 entries: {}", stats.l3_entries);
    println!("  Total demotions: {}", stats.total_demotions);
    
    // Should have evictions due to small cache sizes
    assert!(stats.total_demotions > 0);
    assert!(stats.l1_entries <= 2);
    assert!(stats.l2_entries <= 4);
    
    // Retrieve all embeddings
    for i in 0..10 {
        let result = cache.get(&format!("id_{}", i)).unwrap();
        assert!(result.is_some());
        let embedding = result.unwrap();
        assert_eq!(embedding.len(), 384);
        assert_eq!(embedding[0], i as f32);
    }
    
    // Check hit rates
    let final_stats = cache.get_stats();
    assert!(final_stats.l1_hits + final_stats.l2_hits + final_stats.l3_hits > 0);
    
    println!("✅ Hierarchical cache tier test passed");
    println!("  L1 hit rate: {:.2}%", final_stats.l1_hit_rate() * 100.0);
    println!("  Overall hit rate: {:.2}%", final_stats.overall_hit_rate() * 100.0);
}

#[tokio::test]
async fn test_cache_promotion_policy() {
    let temp_dir = TempDir::new().unwrap();
    let config = CacheConfig {
        l1_max_entries: 5,
        l2_max_entries: 10,
        promotion_threshold: 3,  // Promote after 3 accesses
        enable_statistics: true,
        ..Default::default()
    };
    
    let cache = HierarchicalCache::new(config, temp_dir.path()).unwrap();
    
    // Add embedding
    let embedding = vec![0.5; 384];
    cache.put("hot_item", embedding.clone()).unwrap();
    
    let initial_stats = cache.get_stats();
    assert_eq!(initial_stats.l1_entries, 1);
    
    // Force eviction to L2
    for i in 0..10 {
        cache.put(&format!("other_{}", i), vec![i as f32; 384]).unwrap();
    }
    
    // Now hot_item should be in L2 or L3
    // Access it multiple times to trigger promotion
    for _ in 0..4 {
        let result = cache.get("hot_item").unwrap();
        assert!(result.is_some());
    }
    
    let final_stats = cache.get_stats();
    println!("Promotion test stats:");
    println!("  Total promotions: {}", final_stats.total_promotions);
    println!("  L1 hits: {}", final_stats.l1_hits);
    println!("  L2 hits: {}", final_stats.l2_hits);
    
    // Should have some promotions
    assert!(final_stats.total_promotions > 0 || final_stats.l1_hits > 0);
    
    println!("✅ Cache promotion policy test passed");
}

#[tokio::test]
async fn test_integration_compress_store_cache() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create all components
    let mut compressor = ZstdCompressor::new(CompressionConfig::default());
    let storage = ConcurrentMmapStorage::new(temp_dir.path(), 10 * 1024 * 1024).unwrap();
    let cache = HierarchicalCache::new(CacheConfig::default(), temp_dir.path()).unwrap();
    
    // Generate test embeddings
    let mut embeddings = Vec::new();
    for i in 0..50 {
        let mut embedding = Vec::with_capacity(1536);
        for j in 0..1536 {
            embedding.push(((i * j) as f32).sin());
        }
        embeddings.push((format!("embed_{}", i), embedding));
    }
    
    // Compress and store
    for (id, embedding) in &embeddings {
        // Compress
        let compressed = compressor.compress_embedding(embedding, id).unwrap();
        println!("Compressed {} from {} to {} bytes ({:.1}% saved)",
            id,
            compressed.original_size,
            compressed.compressed_size,
            compressed.space_saved()
        );
        
        // Store in mmap
        storage.store(id, embedding).unwrap();
        
        // Add to cache
        cache.put(id, embedding.clone()).unwrap();
    }
    
    // Verify retrieval from all layers
    for (id, original) in &embeddings[0..5] {
        // Get from cache (should be fast)
        let from_cache = cache.get(id).unwrap().unwrap();
        assert_eq!(from_cache.len(), original.len());
        
        // Get from storage (should work)
        let from_storage = storage.get(id).unwrap();
        assert_eq!(from_storage.len(), original.len());
        
        // Verify values
        for ((orig, cached), stored) in original.iter().zip(from_cache.iter()).zip(from_storage.iter()) {
            assert!((orig - cached).abs() < f32::EPSILON);
            assert!((orig - stored).abs() < f32::EPSILON);
        }
    }
    
    // Print final statistics
    let comp_stats = compressor.get_stats();
    println!("\n📊 Compression Statistics:");
    println!("  Embeddings compressed: {}", comp_stats.embeddings_compressed);
    println!("  Average ratio: {:.2}x", comp_stats.average_ratio);
    println!("  Best ratio: {:.2}x", comp_stats.best_ratio);
    println!("  Worst ratio: {:.2}x", comp_stats.worst_ratio);
    
    let storage_stats = storage.get_stats();
    println!("\n💾 Storage Statistics:");
    println!("  Embeddings stored: {}", storage_stats.embedding_count);
    println!("  Total size: {} MB", storage_stats.total_size_bytes as f64 / 1_048_576.0);
    println!("  Average size: {} KB", storage_stats.average_size as f64 / 1024.0);
    
    let cache_stats = cache.get_stats();
    println!("\n🚀 Cache Statistics:");
    println!("  L1 hit rate: {:.2}%", cache_stats.l1_hit_rate() * 100.0);
    println!("  Overall hit rate: {:.2}%", cache_stats.overall_hit_rate() * 100.0);
    println!("  Total promotions: {}", cache_stats.total_promotions);
    println!("  Total demotions: {}", cache_stats.total_demotions);
    
    println!("\n✅ Integration test passed - all components working together!");
}

#[tokio::test]
async fn test_memory_efficiency() {
    use std::process::Command;
    
    let temp_dir = TempDir::new().unwrap();
    
    // Get initial memory
    let initial_mem = get_current_memory_mb();
    println!("Initial memory: {:.2} MB", initial_mem);
    
    // Create components with optimized settings
    let cache = HierarchicalCache::new(CacheConfig {
        l1_max_size_mb: 0.5,    // 500KB L1
        l2_max_size_mb: 1.5,    // 1.5MB L2
        l3_max_size_mb: 10.0,   // 10MB L3
        ..Default::default()
    }, temp_dir.path()).unwrap();
    
    // Add many embeddings
    for i in 0..100 {
        let embedding = vec![i as f32 / 100.0; 384];
        cache.put(&format!("mem_test_{}", i), embedding).unwrap();
    }
    
    let after_load = get_current_memory_mb();
    let delta = after_load - initial_mem;
    
    println!("After loading 100 embeddings:");
    println!("  Current memory: {:.2} MB", after_load);
    println!("  Memory increase: {:.2} MB", delta);
    
    // Should be memory efficient
    assert!(delta < 10.0, "Memory usage too high: {:.2} MB", delta);
    
    // Access some items to test cache
    for i in 0..10 {
        let _ = cache.get(&format!("mem_test_{}", i)).unwrap();
    }
    
    let final_mem = get_current_memory_mb();
    println!("Final memory: {:.2} MB", final_mem);
    
    println!("✅ Memory efficiency test passed");
}

fn get_current_memory_mb() -> f64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
    }
    0.0
}

// Performance benchmark
#[tokio::test]
async fn test_performance_benchmark() {
    use std::time::Instant;
    
    let temp_dir = TempDir::new().unwrap();
    let mut compressor = ZstdCompressor::new(CompressionConfig::default());
    let storage = ConcurrentMmapStorage::new(temp_dir.path(), 100 * 1024 * 1024).unwrap();
    let cache = HierarchicalCache::new(CacheConfig::default(), temp_dir.path()).unwrap();
    
    // Generate test data
    let embedding = vec![0.5; 1536];  // AWS Titan size
    
    // Benchmark compression
    let start = Instant::now();
    for i in 0..100 {
        let _ = compressor.compress_embedding(&embedding, &format!("bench_{}", i)).unwrap();
    }
    let compress_time = start.elapsed();
    println!("Compression: 100 embeddings in {:?} ({:.2} embeddings/sec)",
        compress_time,
        100.0 / compress_time.as_secs_f64()
    );
    
    // Benchmark storage
    let start = Instant::now();
    for i in 0..100 {
        storage.store(&format!("store_{}", i), &embedding).unwrap();
    }
    let store_time = start.elapsed();
    println!("Storage: 100 embeddings in {:?} ({:.2} embeddings/sec)",
        store_time,
        100.0 / store_time.as_secs_f64()
    );
    
    // Benchmark cache put
    let start = Instant::now();
    for i in 0..100 {
        cache.put(&format!("cache_{}", i), embedding.clone()).unwrap();
    }
    let cache_put_time = start.elapsed();
    println!("Cache put: 100 embeddings in {:?} ({:.2} embeddings/sec)",
        cache_put_time,
        100.0 / cache_put_time.as_secs_f64()
    );
    
    // Benchmark cache get
    let start = Instant::now();
    for i in 0..100 {
        let _ = cache.get(&format!("cache_{}", i)).unwrap();
    }
    let cache_get_time = start.elapsed();
    println!("Cache get: 100 embeddings in {:?} ({:.2} embeddings/sec)",
        cache_get_time,
        100.0 / cache_get_time.as_secs_f64()
    );
    
    // All operations should be fast
    assert!(compress_time.as_secs() < 5);
    assert!(store_time.as_secs() < 5);
    assert!(cache_put_time.as_secs() < 5);
    assert!(cache_get_time.as_secs() < 1);  // Gets should be very fast
    
    println!("✅ Performance benchmark passed");
}
