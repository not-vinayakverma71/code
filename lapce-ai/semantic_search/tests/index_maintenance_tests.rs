// Index Maintenance Tests - SEM-009-C
use lancedb::search::semantic_search_engine::{SemanticSearchEngine, SearchConfig};
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::test]
async fn test_index_creation_after_batch_insert() {
    let config = SearchConfig {
        db_path: PathBuf::from("./test_index_creation"),
        max_embedding_dim: Some(1536),
        index_params: Default::default(),
    };
    
    let engine = Arc::new(SemanticSearchEngine::new(config).await.unwrap());
    
    // Insert batch of data
    let embeddings = vec![vec![0.1; 1536]; 100];
    let chunks = vec![
        lancedb::search::semantic_search_engine::ChunkMetadata {
            path: PathBuf::from("test.rs"),
            content: "test content".to_string(),
            start_line: 1,
            end_line: 10,
            language: Some("rust".to_string()),
            metadata: Default::default(),
        }; 100
    ];
    
    let stats = engine.batch_insert(embeddings, chunks).await.unwrap();
    assert_eq!(stats.chunks_created, 100);
    
    // Verify index is created/updated
    engine.optimize_index().await.unwrap();
    
    // Search should work with index
    let results = engine.search("test", 10, None).await.unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_recall_stability_after_updates() {
    let config = SearchConfig {
        db_path: PathBuf::from("./test_recall_stability"),
        max_embedding_dim: Some(1536),
        index_params: Default::default(),
    };
    
    let engine = Arc::new(SemanticSearchEngine::new(config).await.unwrap());
    
    // Initial insert
    let embeddings1 = vec![vec![0.1; 1536]; 50];
    let chunks1 = vec![
        lancedb::search::semantic_search_engine::ChunkMetadata {
            path: PathBuf::from("file1.rs"),
            content: "async function implementation".to_string(),
            start_line: 1,
            end_line: 10,
            language: Some("rust".to_string()),
            metadata: Default::default(),
        }; 50
    ];
    
    engine.batch_insert(embeddings1, chunks1).await.unwrap();
    
    // Search before update
    let results_before = engine.search("async", 10, None).await.unwrap();
    let recall_before = results_before.len();
    
    // Update with more data
    let embeddings2 = vec![vec![0.2; 1536]; 50];
    let chunks2 = vec![
        lancedb::search::semantic_search_engine::ChunkMetadata {
            path: PathBuf::from("file2.rs"),
            content: "sync function implementation".to_string(),
            start_line: 1,
            end_line: 10,
            language: Some("rust".to_string()),
            metadata: Default::default(),
        }; 50
    ];
    
    engine.batch_insert(embeddings2, chunks2).await.unwrap();
    engine.optimize_index().await.unwrap();
    
    // Search after update
    let results_after = engine.search("async", 10, None).await.unwrap();
    
    // Recall should not degrade significantly
    assert!(results_after.len() >= recall_before, 
            "Recall degraded: {} -> {}", recall_before, results_after.len());
}

#[tokio::test]
async fn test_index_compaction() {
    let config = SearchConfig {
        db_path: PathBuf::from("./test_compaction"),
        max_embedding_dim: Some(1536),
        index_params: Default::default(),
    };
    
    let engine = Arc::new(SemanticSearchEngine::new(config).await.unwrap());
    
    // Multiple small inserts
    for i in 0..10 {
        let embeddings = vec![vec![0.1 * i as f32; 1536]; 10];
        let chunks = vec![
            lancedb::search::semantic_search_engine::ChunkMetadata {
                path: PathBuf::from(format!("file{}.rs", i)),
                content: format!("content {}", i),
                start_line: 1,
                end_line: 10,
                language: Some("rust".to_string()),
                metadata: Default::default(),
            }; 10
        ];
        
        engine.batch_insert(embeddings, chunks).await.unwrap();
    }
    
    // Compact index
    engine.optimize_index().await.unwrap();
    
    // Verify search still works
    let results = engine.search("content", 10, None).await.unwrap();
    assert!(!results.is_empty());
}
