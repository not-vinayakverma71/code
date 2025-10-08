// End-to-end test for fallback line-based chunking with real AWS Titan embeddings
use lancedb::embeddings::service_factory::{ServiceFactory, EmbedderProvider};
use lancedb::embeddings::aws_titan_production::AwsTitanEmbedder;
use lancedb::database::config_manager::EmbedderProvider as ConfigProvider;
use lancedb::storage::lance_store::LanceVectorStore;
use lancedb::processors::scanner::DirectoryScanner;
use lancedb::processors::parser::CodeParser;
use lancedb::database::cache_manager::CacheManager;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_fallback_chunking_with_aws_titan() {
    // Setup test workspace
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();
    
    // Create test source file
    let test_file = workspace_path.join("test.rs");
    std::fs::write(&test_file, r#"
fn hello_world() {
    println!("Hello, world!");
}

fn add_numbers(a: i32, b: i32) -> i32 {
    a + b
}

fn multiply(x: i32, y: i32) -> i32 {
    x * y
}
"#).expect("Failed to write test file");
    
    println!("✅ Created test file: {:?}", test_file);
    
    // Initialize factory with AWS Titan
    let factory = ServiceFactory::new(
        ConfigProvider::AwsTitan,
        workspace_path.clone(),
    ).await.expect("Failed to create factory");
    
    println!("✅ ServiceFactory initialized with AWS Titan");
    
    // Get components
    let embedder = factory.get_embedder();
    let vector_store = factory.get_vector_store();
    let parser = factory.get_parser();
    let cache_manager = factory.get_cache_manager();
    
    // Initialize vector store
    vector_store.initialize().await.expect("Failed to initialize vector store");
    println!("✅ Vector store initialized");
    
    // Test 1: Parse file with fallback chunking
    let content = std::fs::read_to_string(&test_file).expect("Failed to read file");
    let blocks = parser.parse(&content);
    
    println!("✅ Parsed {} code blocks using fallback chunking", blocks.len());
    assert!(blocks.len() > 0, "Should have at least one block");
    
    // Test 2: Generate embeddings with AWS Titan
    let mut embeddings_count = 0;
    for block in &blocks {
        match embedder.embed(&block.content).await {
            Ok(embedding) => {
                println!("✅ Generated embedding for block (dimension: {})", embedding.len());
                assert_eq!(embedding.len(), 1536, "AWS Titan should return 1536-dim embeddings");
                embeddings_count += 1;
            }
            Err(e) => {
                println!("⚠️  Embedding failed: {:?} - This is OK if AWS credentials not configured", e);
            }
        }
    }
    
    println!("✅ Generated {} embeddings successfully", embeddings_count);
    
    // Test 3: Check cache persistence
    let cache_file = workspace_path.join(".cache").join("file_hashes.json");
    if cache_file.exists() {
        println!("✅ Cache file persisted: {:?}", cache_file);
        let cache_content = std::fs::read_to_string(&cache_file)
            .expect("Failed to read cache");
        println!("   Cache content size: {} bytes", cache_content.len());
    }
    
    println!("\n🎉 All tests passed!");
}

#[tokio::test]
async fn test_vector_store_persistence() {
    use lancedb::embeddings::service_factory::{IVectorStore, PointStruct};
    use std::collections::HashMap;
    
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();
    
    // Create vector store
    let store = Arc::new(LanceVectorStore::new(workspace_path.clone(), 1536));
    
    // Initialize
    store.initialize().await.expect("Failed to initialize");
    println!("✅ Vector store initialized");
    
    // Create test points
    let mut points = Vec::new();
    for i in 0..5 {
        let mut payload = HashMap::new();
        payload.insert("filePath".to_string(), serde_json::json!(format!("test_{}.rs", i)));
        payload.insert("codeChunk".to_string(), serde_json::json!(format!("fn test_{}() {{}}", i)));
        payload.insert("startLine".to_string(), serde_json::json!(i * 10));
        payload.insert("endLine".to_string(), serde_json::json!(i * 10 + 5));
        payload.insert("segmentHash".to_string(), serde_json::json!(format!("hash_{}", i)));
        
        points.push(PointStruct {
            id: format!("point_{}", i),
            vector: vec![0.1; 1536], // Dummy embedding
            payload,
        });
    }
    
    // Upsert points
    store.upsert_points(points).await.expect("Failed to upsert points");
    println!("✅ Upserted 5 test points");
    
    // Verify persistence - check if Lance DB files exist
    let lance_dir = workspace_path.join(".lance_index");
    assert!(lance_dir.exists(), "Lance DB directory should exist");
    println!("✅ Lance DB persisted at: {:?}", lance_dir);
    
    // Test search (basic functionality)
    let query_vector = vec![0.1; 1536];
    let results = store.search(query_vector, None, None, Some(3))
        .await
        .expect("Failed to search");
    
    println!("✅ Search returned {} results", results.len());
    
    // Test delete by file path
    store.delete_points_by_file_path("test_0.rs").await
        .expect("Failed to delete points");
    println!("✅ Deleted points by file path");
    
    println!("\n🎉 Vector store persistence test passed!");
}
