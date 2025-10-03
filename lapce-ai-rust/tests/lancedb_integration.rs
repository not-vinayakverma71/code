/// Production Integration Tests for LanceDB
/// MUST PASS ALL 8 PERFORMANCE REQUIREMENTS

use lapce_ai_rust::lancedb::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use std::sync::Arc;
use tokio::task::JoinSet;

#[tokio::test]
async fn test_memory_usage_under_10mb() {
    let temp_dir = TempDir::new().unwrap();
    let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
    
    // Create test files
    let mut test_files = Vec::new();
    for i in 0..100 {
        let file = temp_dir.path().join(format!("test_{}.rs", i));
        std::fs::write(&file, generate_test_code(i)).unwrap();
        test_files.push(file);
    }
    
    // Index files
    db.index_files(test_files).await.unwrap();
    
    // Measure memory
    let memory_mb = get_memory_usage_mb();
    println!("Memory usage: {:.2} MB", memory_mb);
    
    assert!(memory_mb < 10.0, "Memory usage {:.2}MB exceeds 10MB limit", memory_mb);
}

#[tokio::test]
async fn test_query_latency_under_5ms() {
    let temp_dir = TempDir::new().unwrap();
    let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
    
    // Index test files
    let mut test_files = Vec::new();
    for i in 0..500 {
        let file = temp_dir.path().join(format!("test_{}.rs", i));
        std::fs::write(&file, generate_test_code(i)).unwrap();
        test_files.push(file);
    }
    db.index_files(test_files).await.unwrap();
    
    // Test query latency
    let queries = vec![
        "async function",
        "error handling",
        "impl trait",
        "match expression",
        "tokio spawn",
    ];
    
    for query in queries {
        let start = Instant::now();
        let _results = db.search(query, 10).await.unwrap();
        let latency = start.elapsed();
        
        println!("Query '{}' latency: {:?}", query, latency);
        assert!(latency < Duration::from_millis(5), 
            "Query latency {:?} exceeds 5ms limit for '{}'", latency, query);
    }
}

#[tokio::test]
async fn test_indexing_throughput_over_1000_files_per_sec() {
    let temp_dir = TempDir::new().unwrap();
    let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
    
    // Create 2000 test files
    let mut test_files = Vec::new();
    for i in 0..2000 {
        let file = temp_dir.path().join(format!("test_{}.rs", i));
        std::fs::write(&file, generate_test_code(i)).unwrap();
        test_files.push(file);
    }
    
    let file_count = test_files.len();
    let start = Instant::now();
    db.index_files(test_files).await.unwrap();
    let elapsed = start.elapsed();
    
    let throughput = file_count as f64 / elapsed.as_secs_f64();
    println!("Indexing throughput: {:.2} files/sec", throughput);
    
    assert!(throughput > 1000.0, 
        "Indexing throughput {:.2} files/sec below 1000 files/sec target", throughput);
}

#[tokio::test]
async fn test_accuracy_over_90_percent() {
    let temp_dir = TempDir::new().unwrap();
    let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
    
    // Create test files with known content
    let test_cases = vec![
        ("async_handler.rs", "async fn handle_request(req: Request) -> Result<Response>"),
        ("error_handler.rs", "fn handle_error(e: Error) -> String"),
        ("parser.rs", "fn parse_json(input: &str) -> serde_json::Value"),
        ("server.rs", "async fn start_server(port: u16) -> Result<()>"),
        ("client.rs", "impl Client { async fn connect(&self) -> Result<Connection> }"),
    ];
    
    let mut test_files = Vec::new();
    for (name, content) in &test_cases {
        let file = temp_dir.path().join(name);
        std::fs::write(&file, content).unwrap();
        test_files.push(file);
    }
    
    db.index_files(test_files).await.unwrap();
    
    // Test queries and expected results
    let test_queries = vec![
        ("async function", vec!["async_handler.rs", "server.rs", "client.rs"]),
        ("error handling", vec!["error_handler.rs", "async_handler.rs"]),
        ("parse json", vec!["parser.rs"]),
    ];
    
    let mut correct = 0;
    let mut total = 0;
    
    for (query, expected_files) in test_queries {
        let results = db.search(query, 10).await.unwrap();
        
        for expected_file in expected_files {
            total += 1;
            if results.iter().any(|r| r.path.file_name().unwrap() == expected_file) {
                correct += 1;
            }
        }
    }
    
    let accuracy = correct as f64 / total as f64;
    println!("Accuracy: {:.2}%", accuracy * 100.0);
    
    assert!(accuracy > 0.9, "Accuracy {:.2}% below 90% target", accuracy * 100.0);
}

#[tokio::test]
async fn test_incremental_update_under_100ms() {
    let temp_dir = TempDir::new().unwrap();
    let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
    
    // Initial indexing
    let mut test_files = Vec::new();
    for i in 0..50 {
        let file = temp_dir.path().join(format!("test_{}.rs", i));
        std::fs::write(&file, generate_test_code(i)).unwrap();
        test_files.push(file);
    }
    db.index_files(test_files.clone()).await.unwrap();
    
    // Modify a file and test incremental update
    let file_to_update = &test_files[0];
    std::fs::write(file_to_update, "// Updated content\n".to_owned() + &generate_test_code(999)).unwrap();
    
    let start = Instant::now();
    db.incremental_update(file_to_update.clone()).await.unwrap();
    let latency = start.elapsed();
    
    println!("Incremental update latency: {:?}", latency);
    assert!(latency < Duration::from_millis(100),
        "Incremental update {:?} exceeds 100ms limit", latency);
}

#[tokio::test]
async fn test_cache_hit_rate_over_80_percent() {
    let temp_dir = TempDir::new().unwrap();
    let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
    
    // Index some files
    let mut test_files = Vec::new();
    for i in 0..50 {
        let file = temp_dir.path().join(format!("test_{}.rs", i));
        std::fs::write(&file, generate_test_code(i)).unwrap();
        test_files.push(file);
    }
    db.index_files(test_files).await.unwrap();
    
    // Run repeated queries to build cache
    let queries = vec!["async", "function", "error", "impl", "trait"];
    
    // First pass - cache miss
    for query in &queries {
        db.search(query, 10).await.unwrap();
    }
    
    // Second and third pass - should hit cache
    for _ in 0..2 {
        for query in &queries {
            db.search(query, 10).await.unwrap();
        }
    }
    
    let metrics = db.get_metrics();
    let hit_rate = metrics.cache_hit_rate;
    println!("Cache hit rate: {:.2}%", hit_rate * 100.0);
    
    assert!(hit_rate > 0.8, "Cache hit rate {:.2}% below 80% target", hit_rate * 100.0);
}

#[tokio::test]
async fn test_handle_100_concurrent_queries() {
    let temp_dir = TempDir::new().unwrap();
    let db = Arc::new(LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap());
    
    // Index test files
    let mut test_files = Vec::new();
    for i in 0..100 {
        let file = temp_dir.path().join(format!("test_{}.rs", i));
        std::fs::write(&file, generate_test_code(i)).unwrap();
        test_files.push(file);
    }
    db.index_files(test_files).await.unwrap();
    
    // Launch 100 concurrent queries
    let mut handles = JoinSet::new();
    let queries = vec!["async", "function", "error", "impl", "trait"];
    
    let start = Instant::now();
    for i in 0..100 {
        let db_clone = db.clone();
        let query = queries[i % queries.len()].to_string();
        
        handles.spawn(async move {
            db_clone.search(&query, 10).await
        });
    }
    
    // Wait for all queries
    let mut success_count = 0;
    while let Some(result) = handles.join_next().await {
        match result {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => panic!("Query failed: {}", e),
            Err(e) => panic!("Task failed: {}", e),
        }
    }
    
    let elapsed = start.elapsed();
    println!("100 concurrent queries completed in {:?}", elapsed);
    
    assert_eq!(success_count, 100, "Not all concurrent queries succeeded");
    assert!(elapsed < Duration::from_secs(5), 
        "Concurrent queries took too long: {:?}", elapsed);
}

#[tokio::test]
async fn test_scale_to_100k_files() {
    // This test requires significant resources
    if std::env::var("RUN_SCALE_TEST").is_err() {
        println!("Skipping scale test. Set RUN_SCALE_TEST=1 to run");
        return;
    }
    
    let temp_dir = TempDir::new().unwrap();
    let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
    
    // Index 100K files in batches
    let batch_size = 1000;
    let total_files = 100_000;
    
    for batch_num in 0..(total_files / batch_size) {
        let mut batch_files = Vec::new();
        for i in 0..batch_size {
            let file_num = batch_num * batch_size + i;
            let file = temp_dir.path().join(format!("test_{}.rs", file_num));
            std::fs::write(&file, generate_test_code(file_num)).unwrap();
            batch_files.push(file);
        }
        
        println!("Indexing batch {}/{}", batch_num + 1, total_files / batch_size);
        db.index_files(batch_files).await.unwrap();
    }
    
    // Test search still works
    let start = Instant::now();
    let results = db.search("async function", 10).await.unwrap();
    let latency = start.elapsed();
    
    println!("Search in 100K files took {:?}", latency);
    assert!(!results.is_empty(), "No results found in 100K files");
    assert!(latency < Duration::from_millis(50),
        "Search in 100K files too slow: {:?}", latency);
    
    let metrics = db.get_metrics();
    assert_eq!(metrics.total_indexed_files, total_files);
}

// Helper functions

fn generate_test_code(seed: usize) -> String {
    let functions = vec![
        "async fn process_data(input: Vec<u8>) -> Result<String>",
        "fn handle_error(e: Error) -> Response",
        "impl Display for CustomType",
        "trait Handler: Send + Sync",
        "pub struct Server { port: u16 }",
        "match result { Ok(v) => v, Err(e) => return }",
        "tokio::spawn(async move { process().await });",
        "let mut cache = HashMap::new();",
    ];
    
    let code = format!(
        r#"
// Test file {}
use std::collections::HashMap;
use tokio::task;

{}

fn main() {{
    println!("Test {}", seed);
}}
"#,
        seed,
        functions[seed % functions.len()],
    );
    
    code
}

fn get_memory_usage_mb() -> f64 {
    use std::fs;
    use std::process;
    
    let pid = process::id();
    let status_path = format!("/proc/{}/status", pid);
    
    if let Ok(content) = fs::read_to_string(status_path) {
        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
    }
    
    // Fallback to rough estimate
    100.0
}
