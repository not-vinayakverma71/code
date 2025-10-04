/// Test LanceDB on Real Lapce Codebase
/// Indexes and searches the actual /home/verma/lapce directory

use std::path::PathBuf;
use std::time::Instant;
use std::sync::Arc;

fn main() {
    println!("🚀 Testing LanceDB on /home/verma/lapce");
    println!("📊 Monitor with: watch -n 1 'free -h && echo && mpstat 1 1'\n");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        test_lapce_codebase().await;
    });
}

async fn test_lapce_codebase() {
    use lapce_ai_rust::lancedb::complete_engine::{SemanticSearchEngine, CodeIndexer, SearchFilters};
    
    // Initialize engine
    println!("Initializing semantic search engine...");
    let db_path = PathBuf::from("/tmp/lapce_lancedb");
    let engine = Arc::new(
        SemanticSearchEngine::new(db_path).await
            .expect("Failed to create engine")
    );
    
    // Create indexer
    let indexer = CodeIndexer::new(engine.clone());
    
    // Index the Lapce codebase
    println!("\n📁 Indexing /home/verma/lapce...");
    let start = Instant::now();
    
    let stats = indexer.index_repository(&PathBuf::from("/home/verma/lapce")).await
        .expect("Failed to index repository");
    
    let elapsed = start.elapsed();
    
    println!("\n✅ Indexing Complete!");
    println!("  Files indexed: {}", stats.total_files);
    println!("  Chunks created: {}", stats.chunks_created);
    println!("  Time taken: {:?}", elapsed);
    println!("  Throughput: {:.0} files/sec", stats.files_per_sec);
    
    // Test searches
    println!("\n🔍 Running test searches...");
    
    let queries = vec![
        "SharedMemoryTransport",
        "async fn handle",
        "impl Display",
        "tokio spawn",
        "error handling",
        "cache implementation",
        "semantic search",
        "LanceDB",
    ];
    
    for query in queries {
        let start = Instant::now();
        let results = engine.search(query, 5, None).await.unwrap();
        let latency = start.elapsed();
        
        println!("\nQuery: '{}'", query);
        println!("  Latency: {:.2}ms", latency.as_secs_f64() * 1000.0);
        println!("  Results: {}", results.len());
        
        for (i, result) in results.iter().take(3).enumerate() {
            println!("    {}. {} (score: {:.3})", 
                i + 1,
                result.path.display(),
                result.score
            );
        }
    }
    
    // Test memory usage
    println!("\n💾 Memory Statistics:");
    let memory_mb = get_memory_usage();
    println!("  Current memory usage: {:.2}MB", memory_mb);
    
    // Test cache performance
    println!("\n📊 Cache Performance:");
    let hit_rate = engine.metrics.cache_hit_rate();
    let avg_latency = engine.metrics.avg_latency_ms();
    println!("  Cache hit rate: {:.1}%", hit_rate * 100.0);
    println!("  Average latency: {:.2}ms", avg_latency);
    
    // Test concurrent queries
    println!("\n🔄 Testing concurrent queries...");
    let start = Instant::now();
    let mut handles = Vec::new();
    
    for i in 0..50 {
        let engine_clone = engine.clone();
        let query = format!("test query {}", i % 10);
        
        let handle = tokio::spawn(async move {
            engine_clone.search(&query, 10, None).await
        });
        handles.push(handle);
    }
    
    let mut success = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success += 1;
        }
    }
    
    println!("  Concurrent queries: {}/50 successful in {:?}", success, start.elapsed());
    
    println!("\n✅ All tests complete!");
}

fn get_memory_usage() -> f64 {
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
    
    0.0
}
