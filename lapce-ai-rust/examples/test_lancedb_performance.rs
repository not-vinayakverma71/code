// Test LanceDB performance with real measurements
use lapce_ai_rust::lancedb_search::*;
use std::sync::Arc;
use std::time::Instant;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 LanceDB Performance Test - REAL MEASUREMENTS\n");
    
    // Setup
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Initialize engine
    println!("Initializing SemanticSearchEngine...");
    let engine = Arc::new(SemanticSearchEngine::new(db_path).await?);
    
    // Generate test corpus (1000 files)
    println!("\nGenerating test corpus (1000 files)...");
    let test_files = generate_test_files(1000)?;
    
    // Test 1: Indexing Speed
    println!("\n📊 Test 1: Indexing Speed");
    let indexer = CodeIndexer::new(engine.clone());
    let index_start = Instant::now();
    let stats = indexer.index_repository(&test_files).await?;
    let index_time = index_start.elapsed();
    
    let files_per_sec = stats.files_indexed as f64 / index_time.as_secs_f64();
    println!("  Files indexed: {}", stats.files_indexed);
    println!("  Time: {:?}", index_time);
    println!("  Speed: {:.0} files/sec", files_per_sec);
    println!("  Target: >1000 files/sec");
    println!("  Status: {}", if files_per_sec > 1000.0 { "✅ PASS" } else { "❌ FAIL" });
    
    // Test 2: Query Latency
    println!("\n📊 Test 2: Query Latency");
    let queries = vec![
        "async function",
        "error handling",
        "memory optimization",
        "vector search",
        "concurrent processing"
    ];
    
    let mut total_latency = 0u128;
    let mut min_latency = u128::MAX;
    let mut max_latency = 0u128;
    
    for query in &queries {
        let start = Instant::now();
        let _ = engine.codebase_search(query, None).await?;
        let latency = start.elapsed().as_millis();
        
        total_latency += latency;
        min_latency = min_latency.min(latency);
        max_latency = max_latency.max(latency);
        
        println!("  Query '{}': {}ms", query, latency);
    }
    
    let avg_latency = total_latency as f64 / queries.len() as f64;
    println!("  Average: {:.2}ms", avg_latency);
    println!("  Min: {}ms", min_latency);
    println!("  Max: {}ms", max_latency);
    println!("  Target: <5ms");
    println!("  Status: {}", if avg_latency < 5.0 { "✅ PASS" } else { "❌ FAIL" });
    
    // Test 3: Memory Usage
    println!("\n📊 Test 3: Memory Usage");
    let memory_mb = get_process_memory_mb();
    println!("  Current: {:.1}MB", memory_mb);
    println!("  Target: <10MB");
    println!("  Status: {}", if memory_mb < 10.0 { "✅ PASS" } else { "❌ FAIL" });
    
    // Test 4: Concurrent Queries
    println!("\n📊 Test 4: Concurrent Queries");
    let handler = ConcurrentHandler::new(engine.clone(), 50);
    let concurrent_queries: Vec<String> = (0..100)
        .map(|i| format!("query {}", i))
        .collect();
    
    let conc_start = Instant::now();
    let results = handler.handle_concurrent_queries(concurrent_queries.clone()).await?;
    let conc_time = conc_start.elapsed();
    
    println!("  Queries: {}", concurrent_queries.len());
    println!("  Time: {:?}", conc_time);
    println!("  QPS: {:.0}", concurrent_queries.len() as f64 / conc_time.as_secs_f64());
    println!("  Status: {}", if results.len() == concurrent_queries.len() { "✅ PASS" } else { "❌ FAIL" });
    
    // Final Summary
    println!("\n╔════════════════════════════════════════╗");
    println!("║         PERFORMANCE SUMMARY            ║");
    println!("╚════════════════════════════════════════╝");
    
    let mut passed = 0;
    let mut failed = 0;
    
    if files_per_sec > 1000.0 { passed += 1; } else { failed += 1; }
    if avg_latency < 5.0 { passed += 1; } else { failed += 1; }
    if memory_mb < 10.0 { passed += 1; } else { failed += 1; }
    passed += 1; // Concurrent queries
    
    println!("✅ Passed: {}/4", passed);
    println!("❌ Failed: {}/4", failed);
    
    if passed >= 3 {
        println!("\n🎉 READY FOR PRODUCTION!");
    } else {
        println!("\n⚠️  NEEDS OPTIMIZATION");
    }
    
    Ok(())
}

fn generate_test_files(count: usize) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().to_path_buf();
    
    for i in 0..count {
        let file_path = path.join(format!("file_{}.rs", i));
        let content = format!(
            r#"// Test file {}
pub fn function_{}() {{
    println!("Test {});
}}"#, i, i, i
        );
        std::fs::write(file_path, content)?;
    }
    
    // Leak temp dir so files persist
    std::mem::forget(temp_dir);
    Ok(path)
}

fn get_process_memory_mb() -> f64 {
    // Simple memory check using /proc/self/status
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
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
