// Cache Effectiveness Tests - SEM-008-C
use lancedb::search::semantic_search_engine::SemanticSearchEngine;
use lancedb::search::search_metrics::SearchMetrics;
use std::sync::Arc;
use std::path::PathBuf;

#[tokio::test]
async fn test_cache_hit_rate_repeated_queries() {
    let config = lancedb::search::semantic_search_engine::SearchConfig {
        db_path: PathBuf::from("./test_cache_db"),
        max_embedding_dim: Some(1536),
        index_params: Default::default(),
    };
    
    let engine = Arc::new(SemanticSearchEngine::new(config).await.unwrap());
    
    // Get initial metrics from the engine's internal metrics
    let initial_summary = engine.metrics.summary();
    let initial_queries = initial_summary.total_queries;
    
    // First query - cache miss
    let query1 = "async function implementation";
    let _ = engine.search(query1, 10, None).await;
    
    // Repeated queries - should be cache hits
    for _ in 0..10 {
        let _ = engine.search(query1, 10, None).await;
    }
    
    // Get final metrics from the engine
    let final_summary = engine.metrics.summary();
    let total_new_queries = final_summary.total_queries - initial_queries;
    let hit_rate = final_summary.cache_hit_rate;
    
    // Should have >80% hit rate for repeated queries (10 hits out of 11 total = 90.9%)
    assert!(total_new_queries == 11, "Should have 11 queries total");
    assert!(hit_rate > 80.0, "Cache hit rate {:.2}% should be >80% for repeated queries", hit_rate);
    
    println!("✅ Cache hit rate test passed: {:.2}% hit rate", hit_rate);
}

#[tokio::test]
async fn test_cache_hit_rate_similar_queries() {
    let config = lancedb::search::semantic_search_engine::SearchConfig {
        db_path: PathBuf::from("./test_cache_similar_db"),
        max_embedding_dim: Some(1536),
        index_params: Default::default(),
    };
    
    let engine = Arc::new(SemanticSearchEngine::new(config).await.unwrap());
    
    // Similar queries
    let queries = vec![
        "async function",
        "async functions",
        "asynchronous function",
        "async fn",
    ];
    
    // First round - cache misses
    for query in &queries {
        let _ = engine.search(query, 10, None).await;
    }
    
    // Second round - should have cache hits for repeated queries
    for query in &queries {
        let _ = engine.search(query, 10, None).await;
    }
    
    let summary = engine.metrics.summary();
    // With 4 unique queries run twice = 8 total, 4 hits = 50% hit rate
    let hit_rate = summary.cache_hit_rate;
    
    // Should have ≥50% hit rate for repeated similar queries
    assert!(hit_rate >= 50.0, "Cache hit rate {:.2}% should be ≥50% for similar queries", hit_rate);
    println!("✅ Similar queries cache test passed: {:.2}% hit rate", hit_rate);
}

#[tokio::test]
async fn test_cache_isolation_by_filters() {
    let config = lancedb::search::semantic_search_engine::SearchConfig {
        db_path: PathBuf::from("./test_cache_filters_db"),
        max_embedding_dim: Some(1536),
        index_params: Default::default(),
    };
    
    let engine = Arc::new(SemanticSearchEngine::new(config).await.unwrap());
    
    let query = "test query";
    
    // Query with different filters
    let filter1 = lancedb::search::semantic_search_engine::SearchFilters {
        language: Some("rust".to_string()),
        ..Default::default()
    };
    
    let filter2 = lancedb::search::semantic_search_engine::SearchFilters {
        language: Some("python".to_string()),
        ..Default::default()
    };
    
    // First query with filter1
    let results1 = engine.search(query, 10, Some(filter1.clone())).await.unwrap();
    
    // Same query with filter2 - should be cache miss
    let results2 = engine.search(query, 10, Some(filter2)).await.unwrap();
    
    // Repeat with filter1 - should be cache hit
    let results1_cached = engine.search(query, 10, Some(filter1)).await.unwrap();
    
    // Results should be consistent for cached query
    assert_eq!(results1.len(), results1_cached.len());
}

#[tokio::test]
async fn test_cache_latency() {
    let config = lancedb::search::semantic_search_engine::SearchConfig {
        db_path: PathBuf::from("./test_cache_latency_db"),
        max_embedding_dim: Some(1536),
        index_params: Default::default(),
    };
    
    let engine = Arc::new(SemanticSearchEngine::new(config).await.unwrap());
    
    let query = "test latency query";
    
    // First query - cache miss
    let start = std::time::Instant::now();
    let _ = engine.search(query, 10, None).await;
    let miss_duration = start.elapsed();
    
    // Second query - cache hit
    let start = std::time::Instant::now();
    let _ = engine.search(query, 10, None).await;
    let hit_duration = start.elapsed();
    
    // Cache hit should be <5ms
    assert!(hit_duration.as_millis() < 5, "Cache hit latency {:?} should be <5ms", hit_duration);
    
    // Cache hit should be faster than miss
    assert!(hit_duration < miss_duration, "Cache hit should be faster than miss");
}
