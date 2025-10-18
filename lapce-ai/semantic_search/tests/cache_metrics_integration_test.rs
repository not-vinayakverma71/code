// SEM-008-A/B: Integration test for cache metrics correctness
use lancedb::search::semantic_search_engine::{SemanticSearchEngine, SearchConfig};
use lancedb::search::search_metrics::{export_metrics, SearchMetrics};
use lancedb::embeddings::service_factory::IEmbedder;
use std::sync::Arc;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_cache_metrics_no_double_counting() {
    // Create temp directory for test database
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_cache_metrics_db");
    
    // Create a mock embedder for testing
    struct MockEmbedder;
    
    #[async_trait::async_trait]
    impl IEmbedder for MockEmbedder {
        async fn create_embeddings(
            &self,
            texts: Vec<String>,
            _model: Option<String>,
        ) -> lancedb::error::Result<lancedb::embeddings::embedder_interface::EmbeddingResponse> {
            // Return mock embeddings
            let embeddings = texts.iter().map(|_| vec![0.1_f32; 1536]).collect();
            Ok(lancedb::embeddings::embedder_interface::EmbeddingResponse {
                embeddings,
                model: "mock".to_string(),
                usage: None,
            })
        }
        
        fn embedding_dim(&self) -> usize {
            1536
        }
    }
    
    let config = SearchConfig {
        db_path: db_path.to_str().unwrap().to_string(),
        cache_size: 100,
        cache_ttl: 300,
        batch_size: 10,
        max_results: 10,
        min_score: 0.0,
        index_nprobes: Some(4),
        max_embedding_dim: Some(1536),
        optimal_batch_size: Some(10),
    };
    
    let embedder: Arc<dyn IEmbedder> = Arc::new(MockEmbedder);
    let engine = Arc::new(SemanticSearchEngine::new(config, embedder).await.unwrap());
    
    // Get initial metrics
    let initial_metrics = export_metrics();
    let initial_hits = count_metric(&initial_metrics, "semantic_search_cache_hits_total");
    let initial_misses = count_metric(&initial_metrics, "semantic_search_cache_misses_total");
    
    // First query - should be a cache miss
    let query1 = "test query for cache metrics";
    let _ = engine.search(query1, 10, None).await;
    
    // Check metrics after first query
    let metrics_after_miss = export_metrics();
    let hits_after_miss = count_metric(&metrics_after_miss, "semantic_search_cache_hits_total");
    let misses_after_miss = count_metric(&metrics_after_miss, "semantic_search_cache_misses_total");
    
    assert_eq!(hits_after_miss, initial_hits, "Cache hits should not increase on miss");
    assert_eq!(misses_after_miss, initial_misses + 1, "Cache misses should increase by exactly 1");
    
    // Second identical query - should be a cache hit
    let _ = engine.search(query1, 10, None).await;
    
    // Check metrics after cache hit
    let metrics_after_hit = export_metrics();
    let hits_after_hit = count_metric(&metrics_after_hit, "semantic_search_cache_hits_total");
    let misses_after_hit = count_metric(&metrics_after_hit, "semantic_search_cache_misses_total");
    
    assert_eq!(hits_after_hit, hits_after_miss + 1, "Cache hits should increase by exactly 1");
    assert_eq!(misses_after_hit, misses_after_miss, "Cache misses should not increase on hit");
    
    // Verify cache size metric is updated
    assert!(metrics_after_hit.contains("semantic_search_cache_size"), "Cache size metric should be present");
    
    // Verify latency histograms are present
    assert!(metrics_after_hit.contains("semantic_search_latency_seconds"), "Search latency histogram should be present");
    assert!(metrics_after_hit.contains("semantic_search_cache_hit_latency_seconds"), "Cache hit latency histogram should be present");
}

#[tokio::test]
async fn test_cache_metrics_with_filters() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_cache_filters_db");
    
    struct MockEmbedder;
    
    #[async_trait::async_trait]
    impl IEmbedder for MockEmbedder {
        async fn create_embeddings(
            &self,
            texts: Vec<String>,
            _model: Option<String>,
        ) -> lancedb::error::Result<lancedb::embeddings::embedder_interface::EmbeddingResponse> {
            let embeddings = texts.iter().map(|_| vec![0.2_f32; 1536]).collect();
            Ok(lancedb::embeddings::embedder_interface::EmbeddingResponse {
                embeddings,
                model: "mock".to_string(),
                usage: None,
            })
        }
        
        fn embedding_dim(&self) -> usize {
            1536
        }
    }
    
    let config = SearchConfig {
        db_path: db_path.to_str().unwrap().to_string(),
        cache_size: 100,
        cache_ttl: 300,
        batch_size: 10,
        max_results: 10,
        min_score: 0.0,
        index_nprobes: Some(4),
        max_embedding_dim: Some(1536),
        optimal_batch_size: Some(10),
    };
    
    let embedder: Arc<dyn IEmbedder> = Arc::new(MockEmbedder);
    let engine = Arc::new(SemanticSearchEngine::new(config, embedder).await.unwrap());
    
    let query = "test with filters";
    
    // Query with filter1 - cache miss
    let filter1 = lancedb::search::semantic_search_engine::SearchFilters {
        language: Some("rust".to_string()),
        ..Default::default()
    };
    
    let initial_metrics = export_metrics();
    let initial_misses = count_metric(&initial_metrics, "semantic_search_cache_misses_total");
    
    let _ = engine.search(query, 10, Some(filter1.clone())).await;
    
    let metrics_after_filter1 = export_metrics();
    let misses_after_filter1 = count_metric(&metrics_after_filter1, "semantic_search_cache_misses_total");
    assert_eq!(misses_after_filter1, initial_misses + 1, "First query with filter should be a miss");
    
    // Same query with different filter - should be another cache miss
    let filter2 = lancedb::search::semantic_search_engine::SearchFilters {
        language: Some("python".to_string()),
        ..Default::default()
    };
    
    let _ = engine.search(query, 10, Some(filter2)).await;
    
    let metrics_after_filter2 = export_metrics();
    let misses_after_filter2 = count_metric(&metrics_after_filter2, "semantic_search_cache_misses_total");
    assert_eq!(misses_after_filter2, misses_after_filter1 + 1, "Query with different filter should be a miss");
    
    // Repeat with filter1 - should be a cache hit
    let hits_before = count_metric(&metrics_after_filter2, "semantic_search_cache_hits_total");
    
    let _ = engine.search(query, 10, Some(filter1)).await;
    
    let metrics_after_repeat = export_metrics();
    let hits_after = count_metric(&metrics_after_repeat, "semantic_search_cache_hits_total");
    let misses_after_repeat = count_metric(&metrics_after_repeat, "semantic_search_cache_misses_total");
    
    assert_eq!(hits_after, hits_before + 1, "Repeated query with same filter should be a hit");
    assert_eq!(misses_after_repeat, misses_after_filter2, "Misses should not increase on cache hit");
}

// Helper function to count a specific metric value
fn count_metric(metrics_text: &str, metric_name: &str) -> u64 {
    metrics_text
        .lines()
        .find(|line| line.starts_with(metric_name) && !line.starts_with("#"))
        .and_then(|line| {
            // Parse the metric value (format: "metric_name value")
            line.split_whitespace()
                .nth(1)
                .and_then(|v| v.parse::<f64>().ok())
                .map(|v| v as u64)
        })
        .unwrap_or(0)
}
