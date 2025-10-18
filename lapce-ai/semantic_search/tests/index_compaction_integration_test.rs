// SEM-009-C: Integration tests for index compaction with real data
use lancedb::search::semantic_search_engine::{SemanticSearchEngine, SearchConfig, ChunkMetadata};
use lancedb::embeddings::service_factory::IEmbedder;
use lancedb::index::periodic_compaction::IndexCompactionService;
use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;
use tempfile::TempDir;

struct TestEmbedder;

#[async_trait::async_trait]
impl IEmbedder for TestEmbedder {
    async fn create_embeddings(
        &self,
        texts: Vec<String>,
        _model: Option<String>,
    ) -> lancedb::error::Result<lancedb::embeddings::embedder_interface::EmbeddingResponse> {
        // Generate unique embeddings based on text hash for better testing
        let embeddings = texts.iter().map(|text| {
            let hash = text.chars().fold(0u32, |acc, c| acc.wrapping_add(c as u32));
            let base_val = (hash % 100) as f32 / 100.0;
            let mut embedding = vec![base_val; 1536];
            // Add some variation
            for i in 0..10 {
                embedding[i] = base_val + (i as f32 * 0.01);
            }
            embedding
        }).collect();
        
        Ok(lancedb::embeddings::embedder_interface::EmbeddingResponse {
            embeddings,
            model: "test".to_string(),
            usage: None,
        })
    }
    
    fn embedding_dim(&self) -> usize {
        1536
    }
}

#[tokio::test]
async fn test_compaction_with_real_indexing() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_compaction_real_db");
    
    let config = SearchConfig {
        db_path: db_path.to_str().unwrap().to_string(),
        cache_size: 100,
        cache_ttl: 300,
        batch_size: 50,
        max_results: 10,
        min_score: 0.0,
        index_nprobes: Some(4),
        max_embedding_dim: Some(1536),
        optimal_batch_size: Some(50),
    };
    
    let embedder: Arc<dyn IEmbedder> = Arc::new(TestEmbedder);
    let engine = Arc::new(SemanticSearchEngine::new(config, embedder).await.unwrap());
    
    // Phase 1: Index initial documents
    let initial_docs = 100;
    let embeddings: Vec<Vec<f32>> = (0..initial_docs).map(|i| {
        let mut emb = vec![i as f32 / 1000.0; 1536];
        emb[0] = i as f32;
        emb
    }).collect();
    
    let metadata: Vec<ChunkMetadata> = (0..initial_docs).map(|i| {
        ChunkMetadata {
            path: PathBuf::from(format!("src/file_{}.rs", i)),
            content: format!("// Function {} implementation\nfn process_{}_data() {{\n    // Processing logic\n}}", i, i),
            start_line: i * 10,
            end_line: (i * 10) + 5,
            language: Some("rust".to_string()),
            metadata: HashMap::new(),
        }
    }).collect();
    
    let stats = engine.batch_insert(embeddings, metadata).await.unwrap();
    assert_eq!(stats.chunks_created, initial_docs);
    
    // Phase 2: Search before compaction
    let query = "process data implementation";
    let results_before = engine.search(query, 10, None).await.unwrap();
    let recall_before = results_before.len();
    
    // Phase 3: Run manual compaction
    let compaction_service = IndexCompactionService::new(engine.clone());
    compaction_service.compact_now().await.unwrap();
    
    // Phase 4: Search after compaction
    let results_after = engine.search(query, 10, None).await.unwrap();
    let recall_after = results_after.len();
    
    // Verify recall is maintained or improved
    assert!(recall_after >= recall_before, 
        "Recall should not degrade after compaction: before={}, after={}", 
        recall_before, recall_after);
    
    // Phase 5: Index more documents after compaction
    let additional_docs = 50;
    let new_embeddings: Vec<Vec<f32>> = (0..additional_docs).map(|i| {
        let mut emb = vec![(i + initial_docs) as f32 / 1000.0; 1536];
        emb[0] = (i + initial_docs) as f32;
        emb
    }).collect();
    
    let new_metadata: Vec<ChunkMetadata> = (0..additional_docs).map(|i| {
        ChunkMetadata {
            path: PathBuf::from(format!("src/new_file_{}.rs", i)),
            content: format!("// New function {}\nfn handle_{}_request() {{\n    // Handler\n}}", i, i),
            start_line: i * 10,
            end_line: (i * 10) + 5,
            language: Some("rust".to_string()),
            metadata: HashMap::new(),
        }
    }).collect();
    
    let new_stats = engine.batch_insert(new_embeddings, new_metadata).await.unwrap();
    assert_eq!(new_stats.chunks_created, additional_docs);
    
    // Phase 6: Verify search still works after post-compaction inserts
    let final_results = engine.search("handle request", 10, None).await.unwrap();
    assert!(!final_results.is_empty(), "Should find results after post-compaction inserts");
    
    println!("✅ Compaction test passed:");
    println!("  - Initial docs: {}", initial_docs);
    println!("  - Recall before compaction: {}", recall_before);
    println!("  - Recall after compaction: {}", recall_after);
    println!("  - Additional docs: {}", additional_docs);
    println!("  - Final search results: {}", final_results.len());
}

#[tokio::test]
async fn test_compaction_preserves_data_integrity() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_integrity_db");
    
    let config = SearchConfig {
        db_path: db_path.to_str().unwrap().to_string(),
        cache_size: 100,
        cache_ttl: 300,
        batch_size: 20,
        max_results: 50,
        min_score: 0.0,
        index_nprobes: Some(4),
        max_embedding_dim: Some(1536),
        optimal_batch_size: Some(20),
    };
    
    let embedder: Arc<dyn IEmbedder> = Arc::new(TestEmbedder);
    let engine = Arc::new(SemanticSearchEngine::new(config, embedder).await.unwrap());
    
    // Index documents with known content
    let test_content = vec![
        ("async function", "async fn process_data() { /* async processing */ }"),
        ("error handling", "fn handle_error(e: Error) { log::error!(\"{}\", e); }"),
        ("database query", "fn query_db(sql: &str) -> Result<Vec<Row>> { /* query */ }"),
        ("http request", "async fn make_request(url: &str) -> Response { /* request */ }"),
        ("json parsing", "fn parse_json(data: &str) -> serde_json::Value { /* parse */ }"),
    ];
    
    let embeddings: Vec<Vec<f32>> = test_content.iter().enumerate().map(|(i, _)| {
        let mut emb = vec![i as f32 / 10.0; 1536];
        emb[i] = 1.0; // Unique marker
        emb
    }).collect();
    
    let metadata: Vec<ChunkMetadata> = test_content.iter().enumerate().map(|(i, (_, content))| {
        ChunkMetadata {
            path: PathBuf::from(format!("test_{}.rs", i)),
            content: content.to_string(),
            start_line: i * 10,
            end_line: (i * 10) + 5,
            language: Some("rust".to_string()),
            metadata: HashMap::new(),
        }
    }).collect();
    
    engine.batch_insert(embeddings, metadata).await.unwrap();
    
    // Search for each unique term before compaction
    let mut results_before = HashMap::new();
    for (query, _) in &test_content {
        let results = engine.search(query, 10, None).await.unwrap();
        results_before.insert(query.to_string(), results.len());
    }
    
    // Run compaction
    let compaction_service = IndexCompactionService::new(engine.clone());
    compaction_service.compact_now().await.unwrap();
    
    // Search for each unique term after compaction
    let mut results_after = HashMap::new();
    for (query, _) in &test_content {
        let results = engine.search(query, 10, None).await.unwrap();
        results_after.insert(query.to_string(), results.len());
    }
    
    // Verify all queries still return results
    for (query, count_before) in &results_before {
        let count_after = results_after.get(query).unwrap_or(&0);
        assert!(*count_after > 0, "Query '{}' should still return results after compaction", query);
        assert!(*count_after >= *count_before, 
            "Query '{}' should not lose results: before={}, after={}", 
            query, count_before, count_after);
    }
    
    println!("✅ Data integrity test passed - all queries preserved after compaction");
}
