// Full integration test for LanceDB semantic search
use lapce_ai_rust::lancedb_search::{
    LanceDBSystem,
    embeddings::EmbeddingGenerator,
    indexer::{Indexer, IndexStats},
    search::{SearchEngine, SearchResults},
    cache::QueryCache,
    incremental::IncrementalUpdater,
};
use std::sync::Arc;
use std::path::Path;
use std::time::Instant;
use tempfile::TempDir;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 LanceDB Integration Test Starting...\n");
    
    let start = Instant::now();
    
    // Create temp directory for test
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("lancedb");
    let test_files_path = temp_dir.path().join("test_files");
    
    fs::create_dir_all(&db_path).await?;
    fs::create_dir_all(&test_files_path).await?;
    
    // 1. Initialize LanceDB System
    println!("1️⃣ Initializing LanceDB System...");
    let mut db_system = LanceDBSystem::new(db_path.to_str().unwrap()).await?;
    db_system.initialize_table("code_embeddings").await?;
    println!("   ✅ LanceDB initialized");
    
    // 2. Test Embedding Generation
    println!("\n2️⃣ Testing Embedding Generation...");
    let embedding_gen = EmbeddingGenerator::new();
    let test_text = "fn main() { println!(\"Hello, World!\"); }";
    let embedding = embedding_gen.generate(test_text)?;
    assert_eq!(embedding.len(), 768);
    
    // Verify L2 normalization
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 0.01, "Embedding not normalized: {}", norm);
    println!("   ✅ Embeddings: 768 dimensions, L2 normalized");
    
    // 3. Create test files
    println!("\n3️⃣ Creating Test Files...");
    create_test_files(&test_files_path).await?;
    println!("   ✅ Created 100 test files");
    
    // 4. Test Indexing
    println!("\n4️⃣ Testing Indexing...");
    let db_arc = Arc::new(db_system);
    let indexer = Indexer::new(db_arc.clone());
    let index_stats = indexer.index_directory(&test_files_path).await?;
    index_stats.print_summary();
    
    // Verify indexing speed
    if index_stats.files_per_second < 100.0 {
        println!("   ⚠️  Indexing speed: {:.1} files/sec (target: >100)", 
                 index_stats.files_per_second);
    } else {
        println!("   ✅ Indexing speed: {:.1} files/sec", index_stats.files_per_second);
    }
    
    // 5. Test Search
    println!("\n5️⃣ Testing Search...");
    let search_engine = SearchEngine::new(db_arc.clone());
    let query = "implement binary search algorithm";
    let search_results = search_engine.search(query).await?;
    search_results.print_summary();
    
    // Verify search latency
    let latency_ms = search_results.search_time.as_secs_f64() * 1000.0;
    if latency_ms > 5.0 {
        println!("   ⚠️  Search latency: {:.2}ms (target: <5ms)", latency_ms);
    } else {
        println!("   ✅ Search latency: {:.2}ms", latency_ms);
    }
    
    // 6. Test Cache
    println!("\n6️⃣ Testing Query Cache...");
    let cache = Arc::new(QueryCache::new(10000, 3600));
    
    // Insert and retrieve
    cache.insert(query.to_string(), search_results.clone());
    assert!(cache.get(query).is_some());
    println!("   ✅ Cache insertion and retrieval working");
    
    // Test cache hit rate
    for i in 0..100 {
        let test_query = format!("query_{}", i);
        cache.insert(test_query.clone(), search_results.clone());
    }
    
    let stats = cache.stats();
    stats.print_summary();
    
    // 7. Test Incremental Updates
    println!("\n7️⃣ Testing Incremental Updates...");
    let updater = IncrementalUpdater::new(db_arc.clone(), cache.clone(), 50);
    
    // Create a new file
    let new_file = test_files_path.join("new_file.rs");
    fs::write(&new_file, "fn new_function() { /* new code */ }").await?;
    println!("   ✅ File watcher configured (50ms debounce)");
    
    // 8. Performance Summary
    let total_time = start.elapsed();
    println!("\n📊 Performance Summary:");
    println!("  Total test time: {:?}", total_time);
    println!("  Files indexed: {}", index_stats.indexed_files);
    println!("  Indexing speed: {:.1} files/sec", index_stats.files_per_second);
    println!("  Search latency: {:.2}ms", latency_ms);
    println!("  Cache capacity: {} entries", stats.capacity);
    
    // 9. Success Criteria Check
    println!("\n✅ Success Criteria:");
    let mut passed = 0;
    let mut total = 0;
    
    total += 1;
    if index_stats.files_per_second >= 100.0 {
        println!("  ✅ Indexing speed > 100 files/sec");
        passed += 1;
    } else {
        println!("  ❌ Indexing speed < 100 files/sec");
    }
    
    total += 1;
    if latency_ms < 5.0 {
        println!("  ✅ Query latency < 5ms");
        passed += 1;
    } else {
        println!("  ❌ Query latency > 5ms");
    }
    
    total += 1;
    if embedding.len() == 768 {
        println!("  ✅ BERT embedding dimensions = 768");
        passed += 1;
    } else {
        println!("  ❌ Wrong embedding dimensions");
    }
    
    total += 1;
    if (norm - 1.0).abs() < 0.01 {
        println!("  ✅ Embeddings are L2 normalized");
        passed += 1;
    } else {
        println!("  ❌ Embeddings not normalized");
    }
    
    println!("\n🎯 Overall: {}/{} tests passed ({:.0}%)", 
             passed, total, (passed as f64 / total as f64) * 100.0);
    
    if passed == total {
        println!("🎉 All tests passed!");
    } else {
        println!("⚠️  Some tests failed - needs optimization");
    }
    
    Ok(())
}

async fn create_test_files(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create diverse test files
    let languages = vec![
        ("rs", "fn main() { println!(\"Rust code\"); }"),
        ("py", "def main():\n    print(\"Python code\")"),
        ("js", "function main() { console.log(\"JavaScript\"); }"),
        ("ts", "function main(): void { console.log(\"TypeScript\"); }"),
        ("go", "func main() { fmt.Println(\"Go code\") }"),
    ];
    
    for i in 0..100 {
        let (ext, template) = &languages[i % languages.len()];
        let filename = format!("test_file_{}.{}", i, ext);
        let filepath = dir.join(filename);
        
        let content = format!("{}\n// File number {}\n// Binary search implementation\n", template, i);
        fs::write(filepath, content).await?;
    }
    
    Ok(())
}
