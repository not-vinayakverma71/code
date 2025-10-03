// REAL Benchmark with AWS Titan - Production Testing
use lancedb::incremental::{DeltaEncoder, FastIncrementalUpdater};
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

#[tokio::test]
async fn benchmark_incremental_updates_with_aws() {
    println!("\n╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║           REAL BENCHMARK: INCREMENTAL UPDATES WITH AWS TITAN               ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝\n");
    
    // Initialize AWS Titan
    let robust_config = RobustConfig {
        max_retries: 3,
        initial_retry_delay_ms: 500,
        max_retry_delay_ms: 2000,
        max_concurrent_requests: 2,
        requests_per_second: 3.0,
        batch_size: 5,
        request_timeout_secs: 30,
        enable_cache_fallback: true,
    };
    
    let embedder = Arc::new(RobustAwsTitan::new(
        "us-east-1",
        AwsTier::Standard,
        robust_config
    ).await.expect("Failed to create AWS Titan"));
    
    // Setup storage
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
    let updater = FastIncrementalUpdater::new(storage.clone(), 100).await.unwrap();
    
    // Test documents for real semantic changes
    let test_docs = vec![
        ("doc1", "Machine learning is transforming artificial intelligence", "Initial"),
        ("doc2", "Deep neural networks enable complex pattern recognition", "Initial"),
        ("doc3", "Natural language processing helps computers understand text", "Initial"),
        ("doc4", "Computer vision algorithms detect objects in images", "Initial"),
        ("doc5", "Reinforcement learning optimizes decision making processes", "Initial"),
    ];
    
    println!("📝 Phase 1: Initial Indexing");
    println!("─────────────────────────────");
    
    let mut initial_times = Vec::new();
    let mut embeddings_cache = HashMap::new();
    
    for (id, content, version) in &test_docs {
        let start = Instant::now();
        
        // Generate real embedding
        let response = embedder.create_embeddings(vec![content.to_string()], None).await
            .expect("Failed to generate embedding");
        let embedding = response.embeddings[0].clone();
        
        // Cache for later comparison
        embeddings_cache.insert(id.to_string(), embedding.clone());
        
        // Apply initial update
        let metadata = HashMap::from([
            ("content".to_string(), content.to_string()),
            ("version".to_string(), version.to_string()),
        ]);
        
        let update_duration = updater.apply_update(id, &embedding, metadata).await.unwrap();
        let total_time = start.elapsed();
        initial_times.push(total_time);
        
        println!("   {} indexed: {:?} (update: {:?})", id, total_time, update_duration);
    }
    
    // Simulate document changes
    println!("\n📝 Phase 2: Incremental Updates (Document Changes)");
    println!("───────────────────────────────────────────────────");
    
    let changes = vec![
        ("doc1", "Machine learning and deep learning are transforming artificial intelligence systems"),
        ("doc2", "Deep convolutional neural networks enable complex visual pattern recognition"),
        ("doc3", "Advanced natural language processing helps computers understand context in text"),
    ];
    
    let mut update_times = Vec::new();
    let mut delta_sizes = Vec::new();
    
    for (id, new_content) in &changes {
        let start = Instant::now();
        
        // Get old embedding
        let old_embedding = &embeddings_cache[*id];
        
        // Generate new embedding
        let response = embedder.create_embeddings(vec![new_content.to_string()], None).await
            .expect("Failed to generate new embedding");
        let new_embedding = &response.embeddings[0];
        
        // Calculate similarity (cosine)
        let similarity = cosine_similarity(old_embedding, new_embedding);
        
        // Count changed dimensions
        let changes: Vec<_> = old_embedding.iter().zip(new_embedding)
            .enumerate()
            .filter(|(_, (a, b))| (*a - *b).abs() > 0.001)
            .collect();
        let delta_size = changes.len();
        delta_sizes.push(delta_size);
        
        // Apply incremental update
        let metadata = HashMap::from([
            ("content".to_string(), new_content.to_string()),
            ("version".to_string(), "Updated".to_string()),
        ]);
        
        let update_duration = updater.apply_update(id, new_embedding, metadata).await.unwrap();
        let total_time = start.elapsed();
        update_times.push(update_duration);
        
        println!("   {} updated: {:?} (similarity: {:.3}, delta: {} dims)", 
            id, update_duration, similarity, delta_size);
        
        // Update cache
        embeddings_cache.insert(id.to_string(), new_embedding.clone());
    }
    
    // Test batch updates
    println!("\n📝 Phase 3: Batch Updates");
    println!("────────────────────────");
    
    let batch_texts = vec![
        ("batch1", "Transformer models revolutionized NLP"),
        ("batch2", "GPU acceleration speeds up training"),
        ("batch3", "Distributed computing enables large models"),
        ("batch4", "Edge computing brings AI to devices"),
        ("batch5", "Quantum computing promises exponential speedup"),
    ];
    
    let start = Instant::now();
    let mut batch_updates = Vec::new();
    
    // Generate embeddings for batch
    for (id, text) in &batch_texts {
        let response = embedder.create_embeddings(vec![text.to_string()], None).await
            .expect("Failed to generate batch embedding");
        let embedding = response.embeddings[0].clone();
        
        let metadata = HashMap::from([
            ("content".to_string(), text.to_string()),
            ("type".to_string(), "batch".to_string()),
        ]);
        
        batch_updates.push((id.to_string(), embedding, metadata));
    }
    
    let batch_durations = updater.batch_apply_updates(batch_updates).await.unwrap();
    let batch_time = start.elapsed();
    
    println!("   Batch of {} updates: {:?}", batch_texts.len(), batch_time);
    println!("   Average per update: {:?}", batch_time / batch_texts.len() as u32);
    
    // Create snapshot
    println!("\n📸 Version Control");
    println!("──────────────────");
    let version = updater.create_snapshot().await.unwrap();
    println!("   Created snapshot version: {}", version);
    
    // Get final metrics
    let metrics = updater.get_metrics().await;
    
    // Calculate statistics
    update_times.sort();
    let p50 = update_times.get(update_times.len() / 2).copied().unwrap_or_default();
    let p95 = update_times.get(update_times.len() * 95 / 100).copied().unwrap_or_default();
    let p99 = update_times.get(update_times.len() * 99 / 100).copied().unwrap_or_default();
    
    println!("\n╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║                     INCREMENTAL UPDATES PERFORMANCE                        ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝");
    
    println!("\n📊 Update Latency:");
    println!("   • P50: {:?}", p50);
    println!("   • P95: {:?}", p95);
    println!("   • P99: {:?}", p99);
    println!("   • Target < 10ms: {}", if p95 < Duration::from_millis(10) { "✅ ACHIEVED" } else { "❌ MISSED" });
    
    println!("\n📊 Delta Encoding:");
    let avg_delta = delta_sizes.iter().sum::<usize>() / delta_sizes.len().max(1);
    println!("   • Average delta size: {} dimensions (of 1536)", avg_delta);
    println!("   • Compression ratio: {:.1}%", (1536 - avg_delta) as f64 / 1536.0 * 100.0);
    
    println!("\n📊 Overall Metrics:");
    println!("   • Total updates: {}", metrics.total_updates);
    println!("   • Cache hit rate: {:.1}%", 
        metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses).max(1) as f64 * 100.0);
    println!("   • Average update time: {:.2}ms", metrics.avg_update_time_ms);
    
    assert!(p95 < Duration::from_millis(100), "P95 should be reasonable");
}

#[tokio::test]
async fn benchmark_shared_memory_pool() {
    println!("\n╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║                  REAL BENCHMARK: SHARED MEMORY POOL                        ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝\n");
    
    let pool = Arc::new(SharedMemoryPool::new("benchmark_pool".to_string(), 500_000_000).unwrap()); // 500MB
    
    println!("📝 Phase 1: Allocation Performance");
    println!("──────────────────────────────────");
    
    let sizes = vec![
        (1024, "1KB"),
        (10 * 1024, "10KB"),
        (100 * 1024, "100KB"),
        (1024 * 1024, "1MB"),
        (10 * 1024 * 1024, "10MB"),
        (50 * 1024 * 1024, "50MB"),
    ];
    
    let mut allocation_times = Vec::new();
    let mut segments = Vec::new();
    
    for (size, label) in &sizes {
        let start = Instant::now();
        let segment = pool.allocate(*size).unwrap();
        let alloc_time = start.elapsed();
        allocation_times.push((*size, alloc_time));
        
        println!("   Allocated {}: {:?} ({:.2} MB/s)", 
            label, 
            alloc_time,
            *size as f64 / 1_048_576.0 / alloc_time.as_secs_f64());
        
        segments.push(segment);
    }
    
    println!("\n📝 Phase 2: Zero-Copy Read/Write");
    println!("─────────────────────────────────");
    
    // Test on 1MB segment
    let test_segment = &segments[3];
    let test_size = 1024 * 1024;
    
    // Write test
    let test_data: Vec<u8> = (0..test_size).map(|i| (i % 256) as u8).collect();
    
    let start = Instant::now();
    test_segment.write(0, &test_data).unwrap();
    let write_time = start.elapsed();
    
    // Read test
    let start = Instant::now();
    let read_data = test_segment.read(0, test_size).unwrap();
    let read_time = start.elapsed();
    
    // Verify data
    assert_eq!(read_data[0], 0);
    assert_eq!(read_data[1000], (1000 % 256) as u8);
    
    println!("   Write 1MB: {:?} ({:.2} GB/s)", 
        write_time,
        1.0 / write_time.as_secs_f64());
    println!("   Read 1MB: {:?} ({:.2} GB/s)", 
        read_time,
        1.0 / read_time.as_secs_f64());
    
    println!("\n📝 Phase 3: Concurrent Access");
    println!("─────────────────────────────");
    
    let concurrent_segment = pool.allocate(10 * 1024 * 1024).unwrap(); // 10MB
    let segment_id = concurrent_segment.id;
    
    // Spawn multiple tasks
    let mut handles = vec![];
    let num_tasks = 10;
    
    for i in 0..num_tasks {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            
            // Get segment
            let segment = pool_clone.get(segment_id).unwrap();
            
            // Write to different offsets
            let offset = i * 1024 * 1024;
            let data = vec![i as u8; 1024 * 1024];
            segment.write(offset, &data).unwrap();
            
            // Read back
            let read = segment.read(offset, 1024).unwrap();
            assert_eq!(read[0], i as u8);
            
            // Release
            pool_clone.release(segment_id).unwrap();
            
            start.elapsed()
        });
        handles.push(handle);
    }
    
    let mut concurrent_times = Vec::new();
    for handle in handles {
        concurrent_times.push(handle.await.unwrap());
    }
    
    concurrent_times.sort();
    let concurrent_p50 = concurrent_times[concurrent_times.len() / 2];
    let concurrent_p99 = concurrent_times[concurrent_times.len() * 99 / 100];
    
    println!("   {} concurrent tasks:", num_tasks);
    println!("   • P50: {:?}", concurrent_p50);
    println!("   • P99: {:?}", concurrent_p99);
    
    println!("\n📝 Phase 4: Memory Pressure Test");
    println!("────────────────────────────────");
    
    // Allocate until near limit
    let mut pressure_segments = Vec::new();
    let chunk_size = 10 * 1024 * 1024; // 10MB chunks
    let mut total_allocated = 0usize;
    
    loop {
        match pool.allocate(chunk_size) {
            Ok(seg) => {
                total_allocated += chunk_size;
                pressure_segments.push(seg);
                if total_allocated >= 200_000_000 { // Stop at 200MB
                    break;
                }
            }
            Err(_) => break,
        }
    }
    
    println!("   Allocated {}MB in {} segments", 
        total_allocated / 1_048_576, 
        pressure_segments.len());
    
    // Release half
    for _ in 0..pressure_segments.len() / 2 {
        let seg = pressure_segments.pop().unwrap();
        pool.release(seg.id).unwrap();
    }
    
    let stats = pool.stats();
    println!("   After release: {}MB in {} segments", 
        stats.total_allocated / 1_048_576,
        stats.total_segments);
    
    println!("\n📝 Phase 5: IPC Simulation");
    println!("──────────────────────────");
    
    // Process IPC messages
    let start = Instant::now();
    pool.process_ipc_messages().await.unwrap();
    let ipc_time = start.elapsed();
    println!("   IPC message processing: {:?}", ipc_time);
    
    // Final stats
    let final_stats = pool.stats();
    
    println!("\n╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║                      SHARED MEMORY POOL PERFORMANCE                        ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝");
    
    println!("\n📊 Allocation Performance:");
    for (size, time) in &allocation_times {
        let mb = *size as f64 / 1_048_576.0;
        let throughput = mb / time.as_secs_f64();
        println!("   • {:.1}MB: {:?} ({:.0} MB/s)", mb, time, throughput);
    }
    
    println!("\n📊 Zero-Copy Performance:");
    println!("   • Write throughput: {:.2} GB/s", 1.0 / write_time.as_secs_f64());
    println!("   • Read throughput: {:.2} GB/s", 1.0 / read_time.as_secs_f64());
    println!("   • Access time: < {:?}", read_time.min(write_time));
    
    println!("\n📊 Concurrent Access:");
    println!("   • Tasks: {}", num_tasks);
    println!("   • P50 latency: {:?}", concurrent_p50);
    println!("   • P99 latency: {:?}", concurrent_p99);
    
    println!("\n📊 Pool Statistics:");
    println!("   • Max size: {}MB", final_stats.max_size / 1_048_576);
    println!("   • Current allocated: {}MB", final_stats.total_allocated / 1_048_576);
    println!("   • Active segments: {}", final_stats.total_segments);
    
    assert!(write_time < Duration::from_millis(10), "Write should be fast");
    assert!(read_time < Duration::from_millis(10), "Read should be fast");
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}
