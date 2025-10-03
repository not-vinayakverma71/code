// Full System Integration Test - Verifies all components working together
use lancedb::search::{
    SemanticSearchEngine, SearchConfig, CodeIndexer, IncrementalIndexer, HybridSearcher
};
use lancedb::embeddings::aws_titan_robust::{RobustAwsTitan, RobustConfig};
use lancedb::embeddings::aws_titan_production::AwsTier;
use lancedb::embeddings::embedder_interface::IEmbedder;
use std::sync::Arc;
use std::time::Instant;
use tempfile::tempdir;
use std::path::PathBuf;

#[tokio::test]
async fn test_full_system_integration() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║           FULL SYSTEM INTEGRATION TEST                        ║");
    println!("║         Testing All Components with AWS Titan                 ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // Setup
    let tmpdir = tempdir().unwrap();
    let db_path = tmpdir.path().join("test_db").to_str().unwrap().to_string();
    
    // Initialize AWS Titan
    println!("1️⃣ Initializing AWS Titan Embedder");
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
    println!("   ✅ AWS Titan initialized");
    
    // Test embedding generation
    println!("\n2️⃣ Testing Embedding Generation");
    let test_text = "This is a test for semantic search";
    let embedding_result = embedder.create_embeddings(vec![test_text.to_string()], None).await
        .expect("Failed to generate embedding");
    assert!(!embedding_result.embeddings.is_empty());
    println!("   ✅ Generated embedding with dimension: {}", embedding_result.embeddings[0].len());
    
    // Initialize SemanticSearchEngine
    println!("\n3️⃣ Initializing Semantic Search Engine");
    let config = SearchConfig {
        db_path: db_path.clone(),
        cache_size: 1000,
        cache_ttl: 300,
        batch_size: 100,
        max_results: 10,
        min_score: 0.0,
        index_nprobes: Some(10),
        max_embedding_dim: Some(1536),
        optimal_batch_size: Some(100),
    };
    
    let search_engine = Arc::new(
        SemanticSearchEngine::new(config, embedder.clone() as Arc<dyn IEmbedder>).await
            .expect("Failed to create search engine")
    );
    
    println!("   ✅ Search engine initialized with LanceDB");
    
    // Test CodeIndexer
    println!("\n4️⃣ Testing Code Indexer");
    let code_indexer = CodeIndexer::new(search_engine.clone());
    
    // Create test files
    let test_files_dir = tmpdir.path().join("test_code");
    std::fs::create_dir_all(&test_files_dir).unwrap();
    
    // Create test Rust file
    let rust_file = test_files_dir.join("test.rs");
    std::fs::write(&rust_file, r#"
fn main() {
    println!("Hello, world!");
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_add() {
    assert_eq!(add(2, 3), 5);
}
"#).unwrap();
    
    // Create test Python file  
    let py_file = test_files_dir.join("test.py");
    std::fs::write(&py_file, r#"
def main():
    print("Hello from Python")
    
def add(a, b):
    return a + b
    
if __name__ == "__main__":
    main()
"#).unwrap();
    
    // Index the test directory
    let start = Instant::now();
    let index_stats = code_indexer.index_repository(&test_files_dir).await
        .expect("Failed to index repository");
    println!("   ✅ Indexed {} files, {} chunks in {:?}", 
        index_stats.files_indexed, index_stats.chunks_created, start.elapsed());
    
    // Test optimize_index
    println!("\n5️⃣ Testing Index Optimization");
    search_engine.optimize_index().await.expect("Failed to optimize index");
    println!("   ✅ Index optimized");
    
    // Test IncrementalIndexer
    println!("\n6️⃣ Testing Incremental Indexer");
    let incremental_indexer = IncrementalIndexer::new(
        search_engine.clone(),
        Arc::new(lancedb::search::improved_cache::ImprovedQueryCache::new(300, 1000))
    );
    
    // Test handle_change
    let change = lancedb::search::incremental_indexer::FileChange {
        path: rust_file.clone(),
        kind: lancedb::search::incremental_indexer::ChangeKind::Modify,
    };
    incremental_indexer.handle_change(change).await
        .expect("Failed to handle change");
    
    let pending = incremental_indexer.pending_changes().await;
    println!("   ✅ Handled change, pending: {}", pending);
    
    // Test HybridSearcher
    println!("\n7️⃣ Testing Hybrid Search");
    let hybrid_searcher = HybridSearcher::new(search_engine.clone())
        .with_fusion_weight(0.7);
    
    // Create FTS index
    hybrid_searcher.create_fts_index().await
        .expect("Failed to create FTS index");
    println!("   ✅ FTS index created");
    
    // Test search methods
    println!("\n8️⃣ Testing Search Methods");
    
    // Semantic search
    let semantic_results = search_engine.search("function", 5, None).await
        .expect("Semantic search failed");
    println!("   ✅ Semantic search returned {} results", semantic_results.len());
    
    // Hybrid search
    let hybrid_results = hybrid_searcher.search("print", 5, None).await
        .expect("Hybrid search failed");
    println!("   ✅ Hybrid search returned {} results", hybrid_results.len());
    
    // Test cache
    println!("\n9️⃣ Testing Query Cache");
    let start = Instant::now();
    let _ = search_engine.search("function", 5, None).await.unwrap();
    let first_time = start.elapsed();
    
    let start = Instant::now();
    let _ = search_engine.search("function", 5, None).await.unwrap();
    let cached_time = start.elapsed();
    
    println!("   First query: {:?}", first_time);
    println!("   Cached query: {:?}", cached_time);
    assert!(cached_time < first_time, "Cache should be faster");
    println!("   ✅ Cache working (speedup: {:.1}x)", 
        first_time.as_secs_f64() / cached_time.as_secs_f64());
    
    // Test batch_insert
    println!("\n🔟 Testing Batch Insert");
    let test_chunks = vec![
        lancedb::search::semantic_search_engine::ChunkMetadata {
            path: PathBuf::from("test.rs"),
            content: "test content".to_string(),
            start_line: 1,
            end_line: 5,
            language: Some("rust".to_string()),
            metadata: std::collections::HashMap::new(),
        }
    ];
    
    let embeddings = embedder.create_embeddings(
        vec!["test content".to_string()], 
        None
    ).await.expect("Failed to generate embeddings");
    
    let batch_stats = search_engine.batch_insert(
        embeddings.embeddings,
        test_chunks
    ).await.expect("Failed to batch insert");
    println!("   ✅ Batch inserted {} chunks", batch_stats.chunks_created);
    
    println!("\n✨ All Components Working Successfully!");
    println!("═══════════════════════════════════════════════════════════════");
    
    // Final performance check
    println!("\n📊 Final Performance Check:");
    let mut latencies = Vec::new();
    for _ in 0..10 {
        let start = Instant::now();
        let _ = search_engine.search("test", 5, None).await.unwrap();
        latencies.push(start.elapsed());
    }
    
    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[(latencies.len() * 95 / 100).min(latencies.len() - 1)];
    
    println!("   P50 Latency: {:?}", p50);
    println!("   P95 Latency: {:?}", p95);
    
    assert!(p50.as_millis() < 100, "P50 should be under 100ms");
    assert!(p95.as_millis() < 200, "P95 should be under 200ms");
}
