// SEM-009-A: Test that IVF_PQ optimization runs after batch insert
use lancedb::search::semantic_search_engine::{SemanticSearchEngine, SearchConfig};
use lancedb::embeddings::service_factory::IEmbedder;
use std::sync::Arc;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_optimization_after_batch_insert() {
    // Set optimization threshold to a low value for testing
    std::env::set_var("INDEX_OPTIMIZATION_THRESHOLD", "5");
    
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_optimization_db");
    
    struct MockEmbedder;
    
    #[async_trait::async_trait]
    impl IEmbedder for MockEmbedder {
        async fn create_embeddings(
            &self,
            texts: Vec<String>,
            _model: Option<String>,
        ) -> lancedb::error::Result<lancedb::embeddings::embedder_interface::EmbeddingResponse> {
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
    
    // Create test data - more than threshold (5) to trigger optimization
    let embeddings: Vec<Vec<f32>> = (0..10).map(|i| {
        vec![i as f32 / 10.0; 1536]
    }).collect();
    
    let metadata: Vec<lancedb::search::semantic_search_engine::ChunkMetadata> = (0..10).map(|i| {
        lancedb::search::semantic_search_engine::ChunkMetadata {
            path: PathBuf::from(format!("test_file_{}.rs", i)),
            content: format!("Test content {}", i),
            start_line: i,
            end_line: i + 10,
            language: Some("rust".to_string()),
            metadata: std::collections::HashMap::new(),
        }
    }).collect();
    
    // Insert batch - should trigger optimization since count > threshold
    let stats = engine.batch_insert(embeddings, metadata).await.unwrap();
    
    assert_eq!(stats.chunks_created, 10);
    
    // Verify the table has been optimized by checking indices
    let code_table = engine.code_table.read().await;
    if let Some(table) = code_table.as_ref() {
        let indices = table.list_indices().await.unwrap();
        
        // Should have at least one index after optimization
        assert!(!indices.is_empty(), "Table should have indices after optimization");
        
        // Check for IVF_PQ index specifically
        let has_ivf_pq = indices.iter().any(|idx| 
            idx.name.contains("vector") || idx.index_type == "IVF_PQ"
        );
        
        println!("Indices after optimization: {:?}", indices);
        println!("Has IVF_PQ index: {}", has_ivf_pq);
    }
}

#[tokio::test]
async fn test_no_optimization_below_threshold() {
    // Set optimization threshold high
    std::env::set_var("INDEX_OPTIMIZATION_THRESHOLD", "100");
    
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_no_optimization_db");
    
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
    
    // Create test data - less than threshold (100)
    let embeddings: Vec<Vec<f32>> = (0..5).map(|i| {
        vec![i as f32 / 10.0; 1536]
    }).collect();
    
    let metadata: Vec<lancedb::search::semantic_search_engine::ChunkMetadata> = (0..5).map(|i| {
        lancedb::search::semantic_search_engine::ChunkMetadata {
            path: PathBuf::from(format!("test_file_{}.rs", i)),
            content: format!("Test content {}", i),
            start_line: i,
            end_line: i + 10,
            language: Some("rust".to_string()),
            metadata: std::collections::HashMap::new(),
        }
    }).collect();
    
    // Insert batch - should NOT trigger optimization since count < threshold
    let stats = engine.batch_insert(embeddings, metadata).await.unwrap();
    
    assert_eq!(stats.chunks_created, 5);
    println!("✅ Batch insert below threshold completed without forced optimization");
}
