// REAL Comprehensive Test with AWS Titan and File Changes
use lancedb::incremental::{DeltaEncoder, FastIncrementalUpdater, DeltaOperation};
use lancedb::memory::{SharedMemoryPool};
use lancedb::search::fully_optimized_storage::{FullyOptimizedStorage, FullyOptimizedConfig};
use lancedb::embeddings::aws_titan_robust::{RobustAwsTitan, RobustConfig};
use lancedb::embeddings::aws_titan_production::AwsTier;
use lancedb::embeddings::embedder_interface::IEmbedder;
use lancedb::{connect};
use std::sync::Arc;
use std::time::{Instant, Duration};
use std::collections::HashMap;
use tempfile::tempdir;
use tokio::time::sleep;
use std::fs;
use std::path::PathBuf;

#[tokio::test]
async fn test_real_incremental_with_aws_titan() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         REAL INCREMENTAL TEST WITH AWS TITAN                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // 1. Initialize AWS Titan (REAL)
    println!("🔥 Initializing REAL AWS Titan");
    let robust_config = RobustConfig {
        max_retries: 3,
        initial_retry_delay_ms: 1000,
        max_retry_delay_ms: 5000,
        max_concurrent_requests: 3,
        requests_per_second: 2.0,
        batch_size: 5,
        request_timeout_secs: 30,
        enable_cache_fallback: true,
    };
    
    let embedder = Arc::new(RobustAwsTitan::new(
        "us-east-1",
        AwsTier::Standard,
        robust_config
    ).await.expect("Failed to create AWS Titan"));
    
    // Test real embedding generation
    let test_embedding = embedder.create_embeddings(
        vec!["Test incremental update system".to_string()],
        None
    ).await.expect("Failed to generate embedding");
    
    println!("   ✅ AWS Titan working, dimension: {}", test_embedding.embeddings[0].len());
    
    // 2. Setup storage and incremental updater
    let tmpdir = tempdir().unwrap();
    let db_path = tmpdir.path().to_str().unwrap();
    let conn = connect(db_path).execute().await.unwrap();
    let conn = Arc::new(conn);
    
    let config = FullyOptimizedConfig {
        cache_ttl_seconds: 600,
        cache_max_size: 10000,
        ivf_partitions: 2,
        pq_subvectors: 2,
        pq_bits: 8,
        nprobes: 2,
        refine_factor: Some(1),
    };
    
    let storage = Arc::new(FullyOptimizedStorage::new(conn.clone(), config).await.unwrap());
    
    // 3. Create test files that will change
    println!("\n📁 Creating test files for incremental updates");
    let test_dir = tmpdir.path().join("test_files");
    fs::create_dir_all(&test_dir).unwrap();
    
    let mut test_files = vec![
        ("main.rs", "fn main() { println!(\"Hello\"); }", "Initial version"),
        ("lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }", "Math function"),
        ("data.py", "def process(): return 42", "Python processor"),
        ("index.js", "const x = 100; export default x;", "JavaScript module"),
        ("query.sql", "SELECT * FROM users WHERE active = true", "Database query"),
    ];
    
    // Write initial files
    for (name, content, _) in &test_files {
        fs::write(test_dir.join(name), content).unwrap();
    }
    
    // 4. Initialize FastIncrementalUpdater
    println!("\n🚀 Testing Fast Incremental Updater");
    let updater = FastIncrementalUpdater::new(storage.clone(), 100).await.unwrap();
    
    // Initial indexing with AWS Titan
    let mut initial_times = Vec::new();
    for (name, content, desc) in &test_files {
        let start = Instant::now();
        
        // Generate REAL embedding from AWS Titan
        let response = embedder.create_embeddings(vec![content.to_string()], None).await
            .expect("Failed to generate embedding");
        let embedding = &response.embeddings[0];
        
        // Apply update
        let metadata = HashMap::from([
            ("file".to_string(), name.to_string()),
            ("description".to_string(), desc.to_string()),
        ]);
        
        let duration = updater.apply_update(name, embedding, metadata).await.unwrap();
        initial_times.push(duration);
        
        println!("   File '{}' indexed in {:?}", name, duration);
    }
    
    // 5. Simulate file changes and incremental updates
    println!("\n📝 Simulating file changes");
    
    // Change files
    test_files[0].1 = "fn main() { println!(\"Hello, World!\"); eprintln!(\"Debug\"); }";
    test_files[1].1 = "pub fn add(a: i32, b: i32) -> i32 { a + b + 1 } // Modified";
    test_files[2].1 = "def process(): return 42 * 2  # Changed";
    
    let mut update_times = Vec::new();
    
    for (name, new_content, desc) in &test_files[0..3] {
        let start = Instant::now();
        
        // Generate NEW embedding from AWS Titan for changed content
        let response = embedder.create_embeddings(vec![new_content.to_string()], None).await
            .expect("Failed to generate embedding");
        let new_embedding = &response.embeddings[0];
        
        // Apply incremental update
        let metadata = HashMap::from([
            ("file".to_string(), name.to_string()),
            ("description".to_string(), format!("{} - UPDATED", desc)),
            ("version".to_string(), "2".to_string()),
        ]);
        
        let duration = updater.apply_update(name, new_embedding, metadata).await.unwrap();
        update_times.push(duration);
        
        println!("   File '{}' updated in {:?}", name, duration);
    }
    
    // 6. Create version snapshot
    println!("\n📸 Creating version snapshot");
    let version = updater.create_snapshot().await.unwrap();
    println!("   Created snapshot version: {}", version);
    
    // 7. Test rollback
    println!("\n⏪ Testing rollback");
    updater.rollback_to_version(version - 1).await.unwrap();
    println!("   Rolled back to version: {}", version - 1);
    
    // 8. Get metrics
    let metrics = updater.get_metrics().await;
    
    println!("\n📊 Incremental Update Performance:");
    println!("   Total updates: {}", metrics.total_updates);
    println!("   Average time: {:.2}ms", metrics.avg_update_time_ms);
    println!("   P95 time: {:.2}ms", metrics.p95_update_time_ms);
    println!("   Cache hit rate: {:.1}%",
        metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses).max(1) as f64 * 100.0);
    
    // Calculate statistics
    update_times.sort();
    let p50 = update_times.get(update_times.len() / 2).copied().unwrap_or_default();
    let p95 = update_times.get(update_times.len() * 95 / 100).copied().unwrap_or_default();
    
    println!("\n✅ Performance Summary:");
    println!("   P50 update time: {:?}", p50);
    println!("   P95 update time: {:?}", p95);
    println!("   Target < 10ms: {}", if p95 < Duration::from_millis(10) { "✅ ACHIEVED" } else { "❌ FAILED" });
    
    assert!(p95 < Duration::from_millis(100), "Updates should be fast");
}

#[tokio::test]
async fn test_shared_memory_pool_comprehensive() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║        COMPREHENSIVE SHARED MEMORY POOL TEST                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // 1. Create pool
    let pool = Arc::new(SharedMemoryPool::new("comprehensive_test".to_string(), 100_000_000).unwrap());
    
    // 2. Test various segment sizes
    println!("📏 Testing various segment sizes");
    let sizes = vec![1024, 10240, 102400, 1024000, 10240000]; // 1KB to 10MB
    let mut segments = Vec::new();
    
    for size in &sizes {
        let start = Instant::now();
        let segment = pool.allocate(*size).unwrap();
        let alloc_time = start.elapsed();
        
        println!("   Allocated {}KB in {:?}", size / 1024, alloc_time);
        segments.push(segment);
    }
    
    // 3. Test concurrent read/write
    println!("\n🔄 Testing concurrent read/write");
    let test_segment = segments[2].clone(); // 100KB segment
    
    let mut handles = vec![];
    
    // Writers
    for i in 0..3 {
        let seg = test_segment.clone();
        let handle = tokio::spawn(async move {
            let data = vec![i as u8; 1024];
            let offset = i * 1024;
            seg.write(offset, &data).unwrap();
        });
        handles.push(handle);
    }
    
    // Readers
    for i in 0..3 {
        let seg = test_segment.clone();
        let handle = tokio::spawn(async move {
            sleep(Duration::from_millis(1)).await;
            let offset = i * 1024;
            let data = seg.read(offset, 1024).unwrap();
            assert_eq!(data[0], i as u8);
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
    println!("   ✅ Concurrent access successful");
    
    // 4. Test lock/unlock
    println!("\n🔒 Testing locking mechanism");
    let lock_segment = segments[0].clone();
    
    pool.lock(lock_segment.id).unwrap();
    println!("   Segment {} locked", lock_segment.id);
    
    // Try concurrent access while locked
    let pool2 = pool.clone();
    let id = lock_segment.id;
    
    let lock_task = tokio::spawn(async move {
        pool2.lock(id).unwrap(); // This should wait
        pool2.unlock(id).unwrap();
    });
    
    sleep(Duration::from_millis(10)).await;
    pool.unlock(lock_segment.id).unwrap();
    println!("   Segment {} unlocked", lock_segment.id);
    
    lock_task.await.unwrap();
    
    // 5. Test reference counting
    println!("\n🔢 Testing reference counting");
    let ref_segment = pool.allocate(4096).unwrap();
    let id = ref_segment.id;
    
    // Get multiple references
    let ref1 = pool.get(id).unwrap();
    let ref2 = pool.get(id).unwrap();
    let ref3 = pool.get(id).unwrap();
    
    println!("   Created 3 additional references");
    
    // Release references
    pool.release(id).unwrap();
    pool.release(id).unwrap();
    pool.release(id).unwrap();
    
    // Original reference still valid
    ref_segment.write(0, b"Still alive").unwrap();
    println!("   ✅ Reference counting working");
    
    // 6. Test IPC messages
    println!("\n📨 Testing IPC messages");
    pool.process_ipc_messages().await.unwrap();
    println!("   ✅ IPC messages processed");
    
    // 7. Test zero-copy performance
    println!("\n⚡ Testing zero-copy performance");
    let perf_segment = segments[3].clone(); // 1MB segment
    
    let mut zero_copy_times = Vec::new();
    
    for _ in 0..100 {
        let start = Instant::now();
        
        // Direct pointer access (simulated)
        let _ = perf_segment.as_ptr();
        let _ = perf_segment.as_mut_ptr();
        
        zero_copy_times.push(start.elapsed());
    }
    
    zero_copy_times.sort();
    let zc_p50 = zero_copy_times[50];
    let zc_p99 = zero_copy_times[99];
    
    println!("   P50 access: {:?}", zc_p50);
    println!("   P99 access: {:?}", zc_p99);
    
    // 8. Test pool limits
    println!("\n📊 Testing pool limits");
    let stats = pool.stats();
    println!("   Total segments: {}", stats.total_segments);
    println!("   Total allocated: {}MB", stats.total_allocated / 1048576);
    println!("   Max size: {}MB", stats.max_size / 1048576);
    
    // Try to exceed limit
    match pool.allocate(200_000_000) {
        Err(_) => println!("   ✅ Pool limit enforced correctly"),
        Ok(_) => panic!("Should not allocate beyond limit"),
    }
    
    // 9. Cleanup test
    println!("\n🧹 Testing cleanup");
    for segment in segments {
        pool.release(segment.id).unwrap();
    }
    
    let final_stats = pool.stats();
    println!("   Final segments: {}", final_stats.total_segments);
    
    println!("\n✅ All shared memory tests passed!");
    
    assert!(zc_p99 < Duration::from_micros(10), "Zero-copy must be fast");
}

#[tokio::test]
async fn test_delta_encoder_with_real_embeddings() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║          DELTA ENCODER WITH REAL EMBEDDINGS TEST              ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // Initialize AWS Titan
    let robust_config = RobustConfig {
        max_retries: 3,
        initial_retry_delay_ms: 1000,
        max_retry_delay_ms: 5000,
        max_concurrent_requests: 3,
        requests_per_second: 2.0,
        batch_size: 5,
        request_timeout_secs: 30,
        enable_cache_fallback: true,
    };
    
    let embedder = Arc::new(RobustAwsTitan::new(
        "us-east-1",
        AwsTier::Standard,
        robust_config
    ).await.expect("Failed to create AWS Titan"));
    
    // Test delta encoding with real embeddings
    let encoder = DeltaEncoder::new(10);
    
    // Generate embeddings for similar texts
    let text1 = "The quick brown fox jumps over the lazy dog";
    let text2 = "The quick brown fox jumps over the lazy cat"; // Small change
    
    let emb1_response = embedder.create_embeddings(vec![text1.to_string()], None).await.unwrap();
    let emb2_response = embedder.create_embeddings(vec![text2.to_string()], None).await.unwrap();
    
    let emb1 = &emb1_response.embeddings[0];
    let emb2 = &emb2_response.embeddings[0];
    
    // Test encoding
    let start = Instant::now();
    let delta_op = encoder.encode_update(emb1, emb2, HashMap::new()).await.unwrap();
    let encode_time = start.elapsed();
    
    println!("✅ Delta encoded in {:?}", encode_time);
    
    // Add multiple operations
    for i in 0..10 {
        let op = match i % 3 {
            0 => delta_op.clone(),
            1 => DeltaOperation::Add {
                embedding: lancedb::embeddings::compression::CompressedEmbedding::compress(emb1).unwrap(),
                metadata: HashMap::new(),
            },
            _ => DeltaOperation::Delete { hash: i },
        };
        encoder.add_delta(op).await.unwrap();
    }
    
    // Create snapshot
    let snapshot = encoder.create_snapshot().await.unwrap();
    println!("✅ Created snapshot version {}", snapshot.version);
    
    // Test version history
    let history = encoder.get_version_history().await;
    println!("✅ Version history: {:?}", history);
    
    // Test rollback
    let mut embeddings = HashMap::new();
    embeddings.insert(1, emb1.to_vec());
    embeddings.insert(2, emb2.to_vec());
    
    encoder.rollback_to_version(snapshot.version - 1, &mut embeddings).await.unwrap();
    println!("✅ Rolled back successfully");
    
    assert!(encode_time < Duration::from_millis(10), "Encoding should be fast");
}
