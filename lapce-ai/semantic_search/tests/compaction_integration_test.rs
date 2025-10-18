// Integration test for periodic index compaction
use semantic_search::search::semantic_search_engine::{SemanticSearchEngine, SearchConfig, ChunkMetadata};
use semantic_search::index::periodic_compaction::IndexCompactionService;
use semantic_search::embeddings::service_factory::create_embedder;
use std::sync::Arc;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_compaction_runs_with_metrics() {
    // Setup test database
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_compaction_db");
    
    let config = SearchConfig {
        db_path: db_path.to_string_lossy().to_string(),
        cache_size: 100,
        cache_ttl: 60,
        batch_size: 10,
        max_results: 10,
        min_score: 0.5,
        optimal_batch_size: Some(10),
        max_embedding_dim: Some(1536),
        index_nprobes: Some(10),
    };
    
    // Create embedder (will use mock if AWS not available)
    let embedder = create_embedder().await.expect("Failed to create embedder");
    
    // Create engine
    let engine = Arc::new(
        SemanticSearchEngine::new(config, embedder)
            .await
            .expect("Failed to create search engine")
    );
    
    // Insert some test data
    let test_chunks = vec![
        ChunkMetadata {
            path: PathBuf::from("test1.rs"),
            content: "fn main() { println!(\"Hello\"); }".to_string(),
            start_line: 1,
            end_line: 3,
            language: Some("rust".to_string()),
            metadata: std::collections::HashMap::new(),
        },
        ChunkMetadata {
            path: PathBuf::from("test2.rs"),
            content: "struct Point { x: i32, y: i32 }".to_string(),
            start_line: 1,
            end_line: 1,
            language: Some("rust".to_string()),
            metadata: std::collections::HashMap::new(),
        },
    ];
    
    // Generate embeddings
    let embeddings: Vec<Vec<f32>> = test_chunks.iter()
        .map(|_| vec![0.1; 1536])
        .collect();
    
    engine.batch_insert(embeddings, test_chunks)
        .await
        .expect("Failed to insert test data");
    
    // Create compaction service
    let compaction_service = Arc::new(IndexCompactionService::new(engine.clone()));
    
    // Trigger manual compaction
    let result = compaction_service.compact_now().await;
    assert!(result.is_ok(), "Compaction should succeed");
    
    // Verify metrics were recorded (check Prometheus registry)
    let metrics = prometheus::gather();
    let compaction_metrics: Vec<_> = metrics.iter()
        .filter(|m| m.get_name().contains("index_operations"))
        .collect();
    
    assert!(!compaction_metrics.is_empty(), "Compaction metrics should be recorded");
}

#[tokio::test]
async fn test_compaction_backpressure() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_backpressure_db");
    
    let config = SearchConfig {
        db_path: db_path.to_string_lossy().to_string(),
        cache_size: 100,
        cache_ttl: 60,
        batch_size: 10,
        max_results: 10,
        min_score: 0.5,
        optimal_batch_size: Some(10),
        max_embedding_dim: Some(1536),
        index_nprobes: Some(10),
    };
    
    let embedder = create_embedder().await.expect("Failed to create embedder");
    let engine = Arc::new(
        SemanticSearchEngine::new(config, embedder)
            .await
            .expect("Failed to create search engine")
    );
    
    let compaction_service = Arc::new(IndexCompactionService::new(engine.clone()));
    
    // Start first compaction (will hold semaphore)
    let service1 = compaction_service.clone();
    let handle1 = tokio::spawn(async move {
        service1.compact_now().await
    });
    
    // Give first compaction time to acquire semaphore
    sleep(Duration::from_millis(10)).await;
    
    // Try second compaction immediately (should be blocked by semaphore)
    let service2 = compaction_service.clone();
    let handle2 = tokio::spawn(async move {
        service2.compact_now().await
    });
    
    // Wait for both to complete
    let result1 = handle1.await.unwrap();
    let result2 = handle2.await.unwrap();
    
    // Both should succeed (second waits for first)
    assert!(result1.is_ok(), "First compaction should succeed");
    assert!(result2.is_ok(), "Second compaction should succeed after waiting");
}

#[tokio::test]
async fn test_compaction_preserves_recall() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_recall_db");
    
    let config = SearchConfig {
        db_path: db_path.to_string_lossy().to_string(),
        cache_size: 100,
        cache_ttl: 60,
        batch_size: 10,
        max_results: 10,
        min_score: 0.0, // Accept all results for testing
        optimal_batch_size: Some(10),
        max_embedding_dim: Some(1536),
        index_nprobes: Some(10),
    };
    
    let embedder = create_embedder().await.expect("Failed to create embedder");
    let engine = Arc::new(
        SemanticSearchEngine::new(config, embedder)
            .await
            .expect("Failed to create search engine")
    );
    
    // Insert test data
    let test_chunks = vec![
        ChunkMetadata {
            path: PathBuf::from("test.rs"),
            content: "async fn search_code() { }".to_string(),
            start_line: 1,
            end_line: 1,
            language: Some("rust".to_string()),
            metadata: std::collections::HashMap::new(),
        },
    ];
    
    let embeddings = vec![vec![0.5; 1536]];
    
    engine.batch_insert(embeddings, test_chunks)
        .await
        .expect("Failed to insert test data");
    
    // Search before compaction
    let results_before = engine.search("search", 10, None)
        .await
        .expect("Search before compaction failed");
    
    let count_before = results_before.len();
    
    // Run compaction
    let compaction_service = IndexCompactionService::new(engine.clone());
    compaction_service.compact_now()
        .await
        .expect("Compaction failed");
    
    // Search after compaction
    let results_after = engine.search("search", 10, None)
        .await
        .expect("Search after compaction failed");
    
    let count_after = results_after.len();
    
    // Recall should be preserved (same or better)
    assert!(
        count_after >= count_before,
        "Recall should be preserved after compaction: before={}, after={}",
        count_before, count_after
    );
}

#[tokio::test]
async fn test_compaction_improves_or_maintains_latency() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_latency_db");
    
    let config = SearchConfig {
        db_path: db_path.to_string_lossy().to_string(),
        cache_size: 100,
        cache_ttl: 60,
        batch_size: 10,
        max_results: 10,
        min_score: 0.0,
        optimal_batch_size: Some(10),
        max_embedding_dim: Some(1536),
        index_nprobes: Some(10),
    };
    
    let embedder = create_embedder().await.expect("Failed to create embedder");
    let engine = Arc::new(
        SemanticSearchEngine::new(config, embedder)
            .await
            .expect("Failed to create search engine")
    );
    
    // Insert multiple chunks to create fragmentation
    let mut test_chunks = Vec::new();
    for i in 0..50 {
        test_chunks.push(ChunkMetadata {
            path: PathBuf::from(format!("test{}.rs", i)),
            content: format!("fn test_function_{}() {{ }}", i),
            start_line: 1,
            end_line: 1,
            language: Some("rust".to_string()),
            metadata: std::collections::HashMap::new(),
        });
    }
    
    let embeddings: Vec<Vec<f32>> = (0..50)
        .map(|i| vec![0.1 + (i as f32 * 0.01); 1536])
        .collect();
    
    engine.batch_insert(embeddings, test_chunks)
        .await
        .expect("Failed to insert test data");
    
    // Measure latency before compaction
    let start = std::time::Instant::now();
    let _ = engine.search("test", 10, None).await;
    let latency_before = start.elapsed();
    
    // Run compaction
    let compaction_service = IndexCompactionService::new(engine.clone());
    compaction_service.compact_now()
        .await
        .expect("Compaction failed");
    
    // Measure latency after compaction
    let start = std::time::Instant::now();
    let _ = engine.search("test", 10, None).await;
    let latency_after = start.elapsed();
    
    // Latency should not significantly degrade (allow 2x tolerance)
    assert!(
        latency_after < latency_before * 2,
        "Latency should not significantly degrade after compaction: before={:?}, after={:?}",
        latency_before, latency_after
    );
}
