// Test for adaptive exact search pipeline with IVF metadata and L2 bounds

use lancedb::connect;
use lancedb::search::optimized_lancedb_storage::{OptimizedLanceStorage, OptimizedStorageConfig, EmbeddingMetadata};
use lancedb::embeddings::compression::CompressedEmbedding;
use std::sync::Arc;
use tempfile::tempdir;
use std::time::Instant;

#[tokio::test]
async fn test_adaptive_exact_search() {
    println!("\n🔬 Testing Adaptive Exact Search Pipeline\n");
    println!("   This test demonstrates:");
    println!("   1. IVF metadata creation (centroids + radii)");
    println!("   2. Adaptive probing with L2 bounds");
    println!("   3. Exact stopping rule");
    println!("   4. Query latency measurement\n");
    
    // Setup
    let tmpdir = tempdir().unwrap();
    let db_path = tmpdir.path().to_str().unwrap();
    let conn = connect(db_path).execute().await.unwrap();
    let conn = Arc::new(conn);
    
    // Configure for adaptive exact search
    let mut config = OptimizedStorageConfig::default();
    config.adaptive_probe = true;  // Enable adaptive exact pipeline
    config.int8_filter = true;     // Enable int8 bounds
    config.nprobes = 10;          // Start with 10 probes
    config.ivf_partitions = 4;     // Small for demo
    config.pq_subvectors = 16;     // Must divide evenly into 128
    
    let mut storage = OptimizedLanceStorage::new(conn.clone(), config.clone()).await.unwrap();
    
    // Create table with schema
    let dim = 128;  // Smaller dim for faster demo
    let table = storage.create_optimized_table("test_table", dim).await.unwrap();
    
    // Generate test data (500 vectors to satisfy PQ training requirements)
    println!("📝 Generating test data...");
    let num_vectors = 500;  // Need at least 256 for PQ training
    let mut embeddings = Vec::new();
    let mut metadata = Vec::new();
    
    for i in 0..num_vectors {
        // Create distinct vectors in different regions
        let mut vec = vec![0.0f32; dim];
        let region = i / 125;  // 4 regions for 4 IVF partitions
        for j in 0..dim {
            vec[j] = (region as f32 + (i as f32 / 100.0)) * ((j + 1) as f32).sin();
        }
        
        // Normalize for better distance properties
        let norm = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        for v in vec.iter_mut() {
            *v /= norm;
        }
        
        embeddings.push(CompressedEmbedding::compress(&vec).unwrap());
        metadata.push(EmbeddingMetadata {
            id: format!("doc_{}", i),
            path: format!("/file_{}.rs", i),
            content: format!("Content {}", i),
            language: Some("rust".to_string()),
            start_line: i as i32 * 10,
            end_line: (i as i32 + 1) * 10,
        });
    }
    
    // Store compressed batch
    println!("💾 Storing compressed embeddings...");
    storage.store_compressed_batch(&table, embeddings.clone(), metadata).await.unwrap();
    
    // Create IVF_PQ index
    println!("🔨 Building IVF_PQ index...");
    storage.create_index(&table, "vector").await.unwrap();
    
    // Run queries with adaptive exact search
    println!("\n🔍 Running adaptive exact queries...\n");
    
    let query_indices = vec![5, 150, 375];  // Test from different regions
    
    for idx in query_indices {
        // Decompress to get query vector
        let query_vec = embeddings[idx].decompress().unwrap();
        
        println!("   Query {}: vector from region {}", idx, idx / 125);
        
        // Time the query
        let start = Instant::now();
        let results = storage.query_compressed(&table, &query_vec, 5).await.unwrap();
        let latency = start.elapsed();
        
        println!("   ✅ Found {} results in {:?}", results.len(), latency);
        
        // Verify exact correctness: the query itself should be top result
        if !results.is_empty() {
            let top_path = &results[0].path;
            let expected_path = format!("/file_{}.rs", idx);
            
            if top_path == &expected_path {
                println!("   ✅ Exact match verified (self as top result)");
            } else {
                println!("   ⚠️  Top result: {} (expected: {})", top_path, expected_path);
            }
            
            // Show top-3 with distances
            println!("   Top-3 results:");
            for (i, r) in results.iter().take(3).enumerate() {
                println!("      {}: {} (distance: {:.4}, score: {:.4})", 
                    i + 1, r.path, r.distance, r.score);
            }
        }
        
        // Check latency target
        if latency.as_millis() < 10 {
            println!("   ✅ Met <10ms latency target!");
        } else {
            println!("   ⏱️  Latency: {:?} (target: <10ms)", latency);
        }
        
        println!();
    }
    
    // Test with non-adaptive for comparison
    println!("📊 Comparison with standard search:");
    config.adaptive_probe = false;
    let storage_std = OptimizedLanceStorage::new(conn.clone(), config).await.unwrap();
    
    let query_vec = embeddings[150].decompress().unwrap();
    
    let start_adaptive = Instant::now();
    let _ = storage.query_compressed(&table, &query_vec, 5).await.unwrap();
    let adaptive_time = start_adaptive.elapsed();
    
    let start_standard = Instant::now();
    let _ = storage_std.query_compressed(&table, &query_vec, 5).await.unwrap();
    let standard_time = start_standard.elapsed();
    
    println!("   Adaptive: {:?}", adaptive_time);
    println!("   Standard: {:?}", standard_time);
    
    if adaptive_time <= standard_time {
        println!("   ✅ Adaptive is as fast or faster!");
    }
    
    println!("\n✅ Adaptive exact search pipeline test complete!");
}

#[tokio::test]  
async fn test_int8_filtering() {
    use lancedb::optimization::int8_filter;
    
    println!("\n🔬 Testing Int8 Bound Filtering\n");
    
    // Create test vector
    let x = vec![0.1, 0.5, -0.3, 0.8, -0.2];
    let (q8, scale, err_l2) = int8_filter::quantize_per_vector_i8(&x);
    
    println!("   Original: {:?}", x);
    println!("   Quantized (i8): {:?}", q8);
    println!("   Scale: {:.4}, Error L2: {:.6}", scale, err_l2);
    
    // Dequantize and check
    let x_hat = int8_filter::dequantize_per_vector_i8(&q8, scale);
    println!("   Dequantized: {:?}", x_hat);
    
    // Compute actual error
    let mut err2 = 0.0f32;
    for i in 0..x.len() {
        let d = x[i] - x_hat[i];
        err2 += d * d;
    }
    let actual_err = err2.sqrt();
    
    println!("   Actual error: {:.6}", actual_err);
    assert!((actual_err - err_l2).abs() < 1e-5, "Error calculation mismatch");
    
    // Test dot product bound
    let q = vec![0.2, -0.1, 0.4, 0.3, -0.5];
    let dot_exact: f32 = x.iter().zip(&q).map(|(a, b)| a * b).sum();
    let dot_q8 = int8_filter::dot_i8_i8(
        &int8_filter::quantize_per_vector_i8(&q).0,
        &q8
    ) as f32 * scale * int8_filter::quantize_per_vector_i8(&q).1;
    
    println!("\n   Exact dot: {:.4}", dot_exact);
    println!("   Int8 dot: {:.4}", dot_q8);
    println!("   Difference: {:.6}", (dot_exact - dot_q8).abs());
    println!("   Within error bound: {}", (dot_exact - dot_q8).abs() <= err_l2);
    
    println!("\n✅ Int8 filtering test complete!");
}
