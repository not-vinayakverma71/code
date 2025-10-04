/// Test Fast Semantic Search - Verify 1000+ files/sec target
use std::time::Instant;
use tempfile::tempdir;

#[path = "../src/fast_semantic_search.rs"]
mod fast_semantic_search;

use fast_semantic_search::FastSemanticSearch;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TESTING FAST SEMANTIC SEARCH - 1000+ FILES/SEC TARGET");
    println!("{}", "=".repeat(80));
    
    let search = FastSemanticSearch::new().await?;
    
    // Test at different scales
    for size in [100, 1000, 5000] {
        println!("\n📊 Testing with {} files:", size);
        
        let dir = tempdir()?;
        
        // Generate test files
        let start = Instant::now();
        for i in 0..size {
            let path = dir.path().join(format!("file_{:05}.rs", i));
            let content = generate_code(i);
            std::fs::write(path, content)?;
        }
        let gen_time = start.elapsed();
        println!("  Generated in {:.2}s", gen_time.as_secs_f64());
        
        // Index files
        let stats = search.index_directory(dir.path()).await?;
        
        println!("  Files indexed: {}", stats.files_indexed);
        println!("  Chunks created: {}", stats.chunks_indexed);
        println!("  Time: {:.2}s", stats.indexing_time.as_secs_f64());
        println!("  Speed: {:.0} files/sec {}", 
            stats.files_per_second,
            if stats.files_per_second >= 1000.0 { "✅" } else { "❌" }
        );
        
        // Test search latency
        let start = Instant::now();
        let results = search.search("function", 10).await?;
        let latency = start.elapsed();
        println!("  Search latency: {:.2}ms {}", 
            latency.as_millis() as f64,
            if latency.as_millis() < 5 { "✅" } else { "❌" }
        );
        println!("  Results found: {}", results.len());
    }
    
    // Final assessment
    println!("\n{}", "=".repeat(80));
    println!("PERFORMANCE TARGETS");
    println!("{}", "=".repeat(80));
    
    println!("\n📋 Requirements:");
    println!("  Target: 1000+ files/sec indexing");
    println!("  Target: <5ms search latency");
    println!("  Target: <10MB memory usage");
    
    Ok(())
}

fn generate_code(i: usize) -> String {
    match i % 4 {
        0 => format!("fn function_{}() {{ println!(\"test\"); }}", i),
        1 => format!("async fn handler_{}() {{ Ok(()) }}", i),
        2 => format!("struct Data_{} {{ value: i32 }}", i),
        _ => format!("impl Trait for Type_{} {{ }}", i),
    }
}
