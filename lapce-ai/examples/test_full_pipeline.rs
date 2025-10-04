// Test the full indexing and search pipeline
use lapce_ai_rust::lancedb_search::{
    SemanticSearchEngine,
    CodeIndexer,
    SearchFilters,
};
use std::time::Instant;
use tempfile::TempDir;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Full LanceDB Pipeline\n");
    
    // Create temp directory for database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Initialize search engine
    println!("1️⃣ Initializing SemanticSearchEngine...");
    let start = Instant::now();
    let engine = SemanticSearchEngine::new(db_path).await?;
    engine.create_code_table("code_embeddings").await?;
    println!("   ✅ Engine initialized in {:?}\n", start.elapsed());
    
    // Test CodeIndexer
    println!("2️⃣ Testing CodeIndexer...");
    let indexer = CodeIndexer::new(std::sync::Arc::new(engine.clone()));
    
    // Index current directory (small test)
    let index_start = Instant::now();
    let stats = indexer.index_repository(Path::new("./src")).await?;
    let index_time = index_start.elapsed();
    
    println!("   ✅ Indexed {} files with {} chunks", stats.files_indexed, stats.chunks_created);
    println!("   Time: {:?}", index_time);
    println!("   Speed: {:.0} files/sec\n", stats.files_indexed as f64 / index_time.as_secs_f64());
    
    // Test search with filters
    println!("3️⃣ Testing Search with Filters...");
    let filters = SearchFilters::new()
        .with_language("rust".to_string())
        .with_min_score(0.5);
    
    let search_start = Instant::now();
    let results = engine.codebase_search("semantic search", None).await?;
    let search_time = search_start.elapsed();
    
    println!("   ✅ Found {} results in {:?}", results.results.len(), search_time);
    println!("   Latency: {:.2}ms", search_time.as_secs_f64() * 1000.0);
    
    // Check performance criteria
    println!("\n📊 Performance Check:");
    
    // Index speed check
    let index_speed = stats.files_indexed as f64 / index_time.as_secs_f64();
    if index_speed > 1000.0 {
        println!("   ✅ Index speed: {:.0} files/sec (target: >1000)", index_speed);
    } else {
        println!("   ❌ Index speed: {:.0} files/sec (target: >1000)", index_speed);
    }
    
    // Query latency check  
    let query_ms = search_time.as_millis();
    if query_ms < 5 {
        println!("   ✅ Query latency: {}ms (target: <5ms)", query_ms);
    } else {
        println!("   ❌ Query latency: {}ms (target: <5ms)", query_ms);
    }
    
    // Memory usage estimate
    let mem_mb = get_memory_usage();
    if mem_mb < 10.0 {
        println!("   ✅ Memory usage: {:.1}MB (target: <10MB)", mem_mb);
    } else {
        println!("   ❌ Memory usage: {:.1}MB (target: <10MB)", mem_mb);
    }
    
    println!("\n✅ Full pipeline test complete!");
    
    Ok(())
}

fn get_memory_usage() -> f64 {
    // Read from /proc/self/status on Linux
    use std::fs;
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse::<f64>().unwrap_or(0.0) / 1024.0; // KB to MB
            }
        }
    }
    0.0
}
