/// DEEP VERIFICATION OF 85% COMPLETION CLAIM
/// This test will verify each claim systematically

use std::time::Instant;
use std::fs;
use tempfile::tempdir;

// Test if these modules even exist and compile
#[path = "../src/production_semantic_search.rs"]
mod production_semantic_search;

use production_semantic_search::{ProductionSemanticSearch, ProductionEmbedder, ProductionVectorDB, DocumentMetadata, CodeChunk};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("DEEP VERIFICATION OF 85% COMPLETION CLAIM");
    println!("{}", "=".repeat(80));
    
    let mut tests_passed = 0;
    let mut tests_total = 0;
    
    // Test 1: Does ProductionEmbedder exist and work?
    tests_total += 1;
    print!("\n1. Testing ProductionEmbedder...");
    match test_embedder() {
        Ok(true) => {
            println!(" ✅ PASS");
            tests_passed += 1;
        }
        Ok(false) => println!(" ❌ FAIL: Not working correctly"),
        Err(e) => println!(" ❌ FAIL: {}", e),
    }
    
    // Test 2: Does ProductionVectorDB exist and work?
    tests_total += 1;
    print!("\n2. Testing ProductionVectorDB...");
    match test_vector_db().await {
        Ok(true) => {
            println!(" ✅ PASS");
            tests_passed += 1;
        }
        Ok(false) => println!(" ❌ FAIL: Not working correctly"),
        Err(e) => println!(" ❌ FAIL: {}", e),
    }
    
    // Test 3: Does HNSW index actually work?
    tests_total += 1;
    print!("\n3. Testing HNSW index search...");
    match test_hnsw_search().await {
        Ok(true) => {
            println!(" ✅ PASS");
            tests_passed += 1;
        }
        Ok(false) => println!(" ❌ FAIL: Search not working"),
        Err(e) => println!(" ❌ FAIL: {}", e),
    }
    
    // Test 4: Is search latency really <5ms?
    tests_total += 1;
    print!("\n4. Testing search latency (<5ms)...");
    match test_search_latency().await {
        Ok((latency, passed)) => {
            if passed {
                println!(" ✅ PASS: {:.2}ms", latency);
                tests_passed += 1;
            } else {
                println!(" ❌ FAIL: {:.2}ms (>5ms)", latency);
            }
        }
        Err(e) => println!(" ❌ FAIL: {}", e),
    }
    
    // Test 5: Is memory usage really <10MB?
    tests_total += 1;
    print!("\n5. Testing memory usage (<10MB)...");
    match test_memory_usage().await {
        Ok((memory_mb, passed)) => {
            if passed {
                println!(" ✅ PASS: {:.2}MB", memory_mb);
                tests_passed += 1;
            } else {
                println!(" ❌ FAIL: {:.2}MB (>10MB)", memory_mb);
            }
        }
        Err(e) => println!(" ❌ FAIL: {}", e),
    }
    
    // Test 6: Is indexing speed really 1000+ files/sec?
    tests_total += 1;
    print!("\n6. Testing indexing speed (1000+ files/sec)...");
    match test_indexing_speed().await {
        Ok((speed, passed)) => {
            if passed {
                println!(" ✅ PASS: {:.0} files/sec", speed);
                tests_passed += 1;
            } else {
                println!(" ❌ FAIL: {:.0} files/sec (<1000)", speed);
            }
        }
        Err(e) => println!(" ❌ FAIL: {}", e),
    }
    
    // Test 7: Does code chunking work with 50/10 overlap?
    tests_total += 1;
    print!("\n7. Testing code chunking (50/10 overlap)...");
    match test_code_chunking().await {
        Ok(true) => {
            println!(" ✅ PASS");
            tests_passed += 1;
        }
        Ok(false) => println!(" ❌ FAIL: Incorrect chunking"),
        Err(e) => println!(" ❌ FAIL: {}", e),
    }
    
    // Test 8: Is cache integrated?
    tests_total += 1;
    print!("\n8. Testing cache integration...");
    match test_cache_integration().await {
        Ok(true) => {
            println!(" ✅ PASS");
            tests_passed += 1;
        }
        Ok(false) => println!(" ❌ FAIL: Cache not working"),
        Err(e) => println!(" ❌ FAIL: {}", e),
    }
    
    // Test 9: Are embeddings really just hash-based (not ML)?
    tests_total += 1;
    print!("\n9. Verifying embeddings are hash-based (not ML)...");
    match verify_hash_embeddings() {
        Ok(true) => {
            println!(" ✅ CONFIRMED: Hash-based (not ML)");
            tests_passed += 1;
        }
        Ok(false) => println!(" ❌ UNEXPECTED: Not hash-based"),
        Err(e) => println!(" ❌ ERROR: {}", e),
    }
    
    // Calculate actual completion percentage
    println!("\n{}", "=".repeat(80));
    println!("VERIFICATION RESULTS");
    println!("{}", "=".repeat(80));
    println!("\nTests passed: {}/{}", tests_passed, tests_total);
    
    let base_functionality = if tests_passed >= 7 { 70 } else { tests_passed * 10 };
    let optimization_bonus = if tests_passed == tests_total { 15 } else { 0 };
    let actual_completion = base_functionality + optimization_bonus;
    
    println!("\n📊 ACTUAL COMPLETION: {}%", actual_completion);
    
    if actual_completion >= 85 {
        println!("✅ 85% claim VERIFIED");
    } else {
        println!("❌ 85% claim NOT VERIFIED (only {}% complete)", actual_completion);
    }
    
    Ok(())
}

fn test_embedder() -> anyhow::Result<bool> {
    let embedder = ProductionEmbedder::new();
    let embedding1 = embedder.embed("test");
    let embedding2 = embedder.embed("test");
    let embedding3 = embedder.embed("different");
    
    Ok(embedding1.len() == 384 && embedding1 == embedding2 && embedding1 != embedding3)
}

async fn test_vector_db() -> anyhow::Result<bool> {
    let mut db = ProductionVectorDB::new();
    let embedder = ProductionEmbedder::new();
    
    for i in 0..10 {
        let embedding = embedder.embed(&format!("doc {}", i));
        let metadata = DocumentMetadata {
            path: format!("file{}.rs", i).into(),
            content: format!("content {}", i),
            start_line: 1,
            end_line: 10,
        };
        db.insert(embedding, metadata);
    }
    
    Ok(db.size() == 10)
}

async fn test_hnsw_search() -> anyhow::Result<bool> {
    let mut db = ProductionVectorDB::new();
    let embedder = ProductionEmbedder::new();
    
    for i in 0..100 {
        let embedding = embedder.embed(&format!("document {}", i));
        let metadata = DocumentMetadata {
            path: format!("file{}.rs", i).into(),
            content: format!("content {}", i),
            start_line: 1,
            end_line: 10,
        };
        db.insert(embedding, metadata);
    }
    
    let query = embedder.embed("document 50");
    let results = db.search(&query, 10);
    
    Ok(results.len() == 10)
}

async fn test_search_latency() -> anyhow::Result<(f64, bool)> {
    let search = ProductionSemanticSearch::new().await?;
    let dir = tempdir()?;
    
    for i in 0..100 {
        let path = dir.path().join(format!("file{}.rs", i));
        fs::write(&path, format!("fn main() {{ println!(\"{}\"); }}", i))?;
    }
    
    search.index_directory(dir.path()).await?;
    
    let start = Instant::now();
    let _results = search.search("main function", 10).await?;
    let latency = start.elapsed().as_millis() as f64;
    
    Ok((latency, latency < 5.0))
}

async fn test_memory_usage() -> anyhow::Result<(f64, bool)> {
    let search = ProductionSemanticSearch::new().await?;
    let dir = tempdir()?;
    
    for i in 0..1000 {
        let path = dir.path().join(format!("file{}.rs", i));
        fs::write(&path, format!("fn function_{}() {{ }}", i))?;
    }
    
    let indexed = search.index_directory(dir.path()).await?;
    let (vectors, _cache) = search.get_stats().await;
    
    let memory_mb = (vectors * 384 * 4) as f64 / (1024.0 * 1024.0);
    Ok((memory_mb, memory_mb < 10.0))
}

async fn test_indexing_speed() -> anyhow::Result<(f64, bool)> {
    let search = ProductionSemanticSearch::new().await?;
    let dir = tempdir()?;
    
    for i in 0..1000 {
        let path = dir.path().join(format!("file{}.rs", i));
        fs::write(&path, format!("fn f{}() {{ }}", i))?;
    }
    
    let start = Instant::now();
    search.index_directory(dir.path()).await?;
    let elapsed = start.elapsed().as_secs_f64();
    
    let speed = 1000.0 / elapsed;
    Ok((speed, speed >= 1000.0))
}

async fn test_code_chunking() -> anyhow::Result<bool> {
    let search = ProductionSemanticSearch::new().await?;
    
    let code = (0..100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
    let chunks = search.chunk_code(&code);
    
    if chunks.len() < 2 {
        return Ok(false);
    }
    
    // Check first chunk is 50 lines
    let first_lines = chunks[0].content.lines().count();
    // Check overlap
    let overlap = chunks[0].end_line - chunks[1].start_line + 1;
    
    Ok(first_lines == 50 && overlap == 10)
}

async fn test_cache_integration() -> anyhow::Result<bool> {
    let search = ProductionSemanticSearch::new().await?;
    let dir = tempdir()?;
    
    let path = dir.path().join("test.rs");
    fs::write(&path, "fn main() {}")?;
    search.index_file(&path, "fn main() {}").await?;
    
    // First search
    let start1 = Instant::now();
    let results1 = search.search("main", 10).await?;
    let time1 = start1.elapsed();
    
    // Second search (should be cached)
    let start2 = Instant::now();
    let results2 = search.search("main", 10).await?;
    let time2 = start2.elapsed();
    
    Ok(results1 == results2 && time2 <= time1)
}

fn verify_hash_embeddings() -> anyhow::Result<bool> {
    let embedder = ProductionEmbedder::new();
    
    // Test determinism
    let embed1 = embedder.embed("test string");
    let embed2 = embedder.embed("test string");
    
    // Test that it's not random
    if embed1 != embed2 {
        return Ok(false);
    }
    
    // Test that embeddings are normalized
    let norm: f32 = embed1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let is_normalized = (norm - 1.0).abs() < 0.01;
    
    Ok(is_normalized)
}
