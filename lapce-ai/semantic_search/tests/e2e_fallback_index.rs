// E2E test for fallback indexing without CST
// Tests real indexing and search with AWS Titan embeddings

use semantic_search::error::Result;
use semantic_search::search::semantic_search_engine::{SemanticSearchEngine, SearchConfig, ChunkMetadata};
use semantic_search::embeddings::service_factory::CodeIndexServiceFactory;
use semantic_search::embeddings::config::TitanConfig;
use semantic_search::database::config_manager::{CodeIndexConfigManager, CodeIndexConfig, EmbedderProvider};
use semantic_search::database::cache_manager::CacheManager;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[tokio::test]
async fn test_fallback_indexing_and_search() -> Result<()> {
    // Skip test if AWS credentials are not available
    if std::env::var("AWS_ACCESS_KEY_ID").is_err() || 
       std::env::var("AWS_SECRET_ACCESS_KEY").is_err() {
        eprintln!("Skipping E2E test: AWS credentials not configured");
        return Ok(());
    }

    // Create temp directory for test
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();
    let db_path = workspace_path.join("test_lancedb");

    // Setup configuration
    let config = CodeIndexConfig {
        enabled: true,
        embedder_provider: EmbedderProvider::AwsTitan,
        model_id: Some("amazon.titan-embed-text-v1".to_string()),
        model_dimension: Some(1536),
        open_ai_options: None,
        open_ai_compatible_options: None,
        gemini_options: None,
    };

    let config_manager = Arc::new(CodeIndexConfigManager::new(config));
    let cache_manager = Arc::new(CacheManager::new(workspace_path.clone()));

    // Create service factory and embedder
    let factory = CodeIndexServiceFactory::new(
        config_manager.clone(),
        workspace_path.clone(),
        cache_manager.clone(),
    );

    let embedder = factory.create_embedder().await?;

    // Validate embedder configuration
    let validation = factory.validate_embedder(&embedder).await?;
    assert!(validation.valid, "Embedder validation failed: {:?}", validation.error);

    // Create search engine
    let search_config = SearchConfig {
        db_path: db_path.to_string_lossy().to_string(),
        cache_size: 100,
        cache_ttl: 60,
        batch_size: 10,
        max_results: 10,
        min_score: 0.3,
        optimal_batch_size: Some(5),
        max_embedding_dim: Some(1536),
        index_nprobes: Some(10),
    };

    let engine = Arc::new(SemanticSearchEngine::new(search_config, embedder).await?);

    // Create test code samples
    let test_files = vec![
        (
            "test1.rs",
            r#"
fn calculate_fibonacci(n: u32) -> u32 {
    if n <= 1 {
        return n;
    }
    calculate_fibonacci(n - 1) + calculate_fibonacci(n - 2)
}

fn main() {
    let result = calculate_fibonacci(10);
    println!("Fibonacci of 10 is: {}", result);
}
"#,
        ),
        (
            "test2.py",
            r#"
def merge_sort(arr):
    if len(arr) <= 1:
        return arr
    
    mid = len(arr) // 2
    left = merge_sort(arr[:mid])
    right = merge_sort(arr[mid:])
    
    return merge(left, right)

def merge(left, right):
    result = []
    i = j = 0
    
    while i < len(left) and j < len(right):
        if left[i] <= right[j]:
            result.append(left[i])
            i += 1
        else:
            result.append(right[j])
            j += 1
    
    result.extend(left[i:])
    result.extend(right[j:])
    return result
"#,
        ),
        (
            "test3.js",
            r#"
class BinarySearchTree {
    constructor() {
        this.root = null;
    }
    
    insert(value) {
        const newNode = { value, left: null, right: null };
        
        if (!this.root) {
            this.root = newNode;
            return;
        }
        
        let current = this.root;
        while (true) {
            if (value < current.value) {
                if (!current.left) {
                    current.left = newNode;
                    break;
                }
                current = current.left;
            } else {
                if (!current.right) {
                    current.right = newNode;
                    break;
                }
                current = current.right;
            }
        }
    }
    
    search(value) {
        let current = this.root;
        while (current) {
            if (value === current.value) return true;
            current = value < current.value ? current.left : current.right;
        }
        return false;
    }
}
"#,
        ),
    ];

    // Index test files
    println!("Indexing {} test files...", test_files.len());
    
    for (filename, content) in &test_files {
        let path = workspace_path.join(filename);
        
        // Create chunk metadata
        let lines: Vec<&str> = content.lines().collect();
        let chunk = ChunkMetadata {
            path: path.clone(),
            content: content.to_string(),
            start_line: 1,
            end_line: lines.len(),
            language: Some(detect_language(filename)),
            metadata: std::collections::HashMap::new(),
        };
        
        // Generate embedding
        let embedding_response = engine.embedder
            .create_embeddings(vec![content.to_string()], None)
            .await?;
        
        assert_eq!(embedding_response.embeddings.len(), 1);
        let embedding = embedding_response.embeddings.into_iter().next().unwrap();
        
        // Insert into database
        let stats = engine.batch_insert(vec![embedding], vec![chunk]).await?;
        assert_eq!(stats.files_indexed, 1);
        assert_eq!(stats.chunks_created, 1);
    }

    // Test search queries
    let test_queries = vec![
        ("fibonacci recursive function", 1),  // Should find test1.rs
        ("merge sort algorithm", 1),          // Should find test2.py
        ("binary search tree", 1),            // Should find test3.js
        ("sorting algorithm", 2),             // Should find merge sort and maybe BST
        ("data structure", 1),                // Should find BST
    ];

    println!("\nRunning {} search queries...", test_queries.len());
    
    for (query, min_expected) in test_queries {
        println!("  Searching for: '{}'", query);
        
        let results = engine.search(query, 5, None).await?;
        
        assert!(
            results.len() >= min_expected,
            "Query '{}' returned {} results, expected at least {}",
            query,
            results.len(),
            min_expected
        );
        
        // Verify results have valid structure
        for result in &results {
            assert!(!result.path.is_empty(), "Result has empty path");
            assert!(!result.content.is_empty(), "Result has empty content");
            assert!(result.start_line > 0, "Invalid start_line");
            assert!(result.end_line >= result.start_line, "Invalid line range");
            assert!(result.score > 0.0 && result.score <= 1.0, "Invalid score: {}", result.score);
        }
        
        if !results.is_empty() {
            println!("    Found {} results, top match: {} (score: {:.3})",
                results.len(),
                Path::new(&results[0].path).file_name().unwrap().to_string_lossy(),
                results[0].score
            );
        }
    }

    // Test cache hit rate
    println!("\nTesting cache performance...");
    
    // Run same query twice
    let query = "fibonacci";
    let results1 = engine.search(query, 5, None).await?;
    let results2 = engine.search(query, 5, None).await?;
    
    // Results should be identical
    assert_eq!(results1.len(), results2.len());
    for (r1, r2) in results1.iter().zip(results2.iter()) {
        assert_eq!(r1.path, r2.path);
        assert_eq!(r1.start_line, r2.start_line);
        assert_eq!(r1.end_line, r2.end_line);
    }

    println!("✅ All E2E tests passed!");
    
    Ok(())
}

fn detect_language(filename: &str) -> String {
    match filename.split('.').last() {
        Some("rs") => "rust",
        Some("py") => "python",
        Some("js") => "javascript",
        Some("ts") => "typescript",
        _ => "text",
    }.to_string()
}
