// Query the actually indexed data using OUR SYSTEM
use lancedb::connect;
use lancedb::search::semantic_search_engine::{SemanticSearchEngine, SearchConfig, SearchFilters};
use lancedb::embeddings::aws_titan_production::{AwsTitanProduction, AwsTier};
use std::sync::Arc;
use std::time::Instant;
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║      QUERYING INDEXED DATA USING OUR SEMANTIC SEARCH SYSTEM       ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");
    
    // Check for existing indexed databases in temp directories
    let temp_dir_path = std::env::temp_dir();
    let temp_dir_str = temp_dir_path.to_str().unwrap();
    let temp_dirs = vec![
        "/tmp",
        "/var/folders",
        temp_dir_str,
    ];
    
    println!("🔍 Phase 1: Checking for indexed data");
    println!("═══════════════════════════════════════");
    
    let mut found_db = None;
    for dir in &temp_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap().to_str().unwrap();
                    if name.starts_with("tmp") || name.contains("lance") || name.contains("test") {
                        // Check if it's a valid LanceDB
                        let table_path = path.join("code_embeddings.lance");
                        if table_path.exists() {
                            found_db = Some(path.clone());
                            println!("  ✅ Found indexed database at: {}", path.display());
                            
                            // Calculate size
                            let size = get_dir_size(&path);
                            println!("  📊 Database size: {:.2} MB", size as f64 / 1_048_576.0);
                            
                            // Check table info
                            if let Ok(metadata) = fs::metadata(&table_path) {
                                println!("  📊 Table size: {:.2} MB", metadata.len() as f64 / 1_048_576.0);
                            }
                        }
                    }
                }
            }
        }
    }
    
    if found_db.is_none() {
        println!("  ❌ No indexed database found. Creating new one...");
        found_db = Some(std::env::temp_dir().join("semantic_search_test"));
    }
    
    let db_path = found_db.unwrap();
    
    println!("\n📊 Phase 2: Memory Analysis");
    println!("═══════════════════════════════════════");
    
    let mem_start = get_memory_mb();
    println!("  Memory before loading: {:.2} MB", mem_start);
    
    // Connect to database using OUR system
    let conn = Arc::new(connect(db_path.to_str().unwrap()).execute().await.unwrap());
    
    // Initialize OUR embedder (AWS Titan)
    let embedder = Arc::new(AwsTitanProduction::new(
        "us-east-1",
        AwsTier::Standard,
    ).await.expect("Failed to create AWS Titan"));
    
    // Initialize OUR search engine with OUR config
    let search_config = SearchConfig {
        db_path: db_path.to_str().unwrap().to_string(),
        cache_size: 1000,
        cache_ttl: 600,
        batch_size: 10,
        max_results: 10,
        min_score: 0.0,
        index_nprobes: Some(4),
        max_embedding_dim: Some(1536),
        optimal_batch_size: Some(10),
    };
    
    let search_engine = Arc::new(SemanticSearchEngine::new(
        search_config.clone(),
        embedder.clone()
    ).await.unwrap());
    
    let mem_after_load = get_memory_mb();
    println!("  Memory after loading system: {:.2} MB", mem_after_load);
    println!("  Memory used by system: {:.2} MB", mem_after_load - mem_start);
    
    // Check if table exists
    let tables = conn.table_names().execute().await.unwrap();
    println!("\n  Available tables: {:?}", tables);
    
    if tables.contains(&"code_embeddings".to_string()) {
        // Get table info
        let table = conn.open_table("code_embeddings").execute().await.unwrap();
        let count = table.count_rows(None).await.unwrap();
        println!("  ✅ Found {} indexed documents", count);
        
        // Calculate embeddings memory
        // Each embedding: 1536 floats * 4 bytes = 6KB
        // 100 files = 600KB just for embeddings
        let embedding_memory = count * 1536 * 4;
        println!("  📊 Embeddings memory: {:.2} KB ({} files × 1536 × 4 bytes)", 
            embedding_memory as f64 / 1024.0, count);
        
        println!("\n🔍 Phase 3: Query Performance Using OUR System");
        println!("═══════════════════════════════════════");
        
        let queries = vec![
            "implement semantic search with vector database",
            "async function error handling",
            "parse configuration file JSON",
            "cache optimization performance",
            "concurrent task execution",
        ];
        
        let mut query_times = Vec::new();
        
        for query in &queries {
            println!("\n  Query: '{}'", query);
            
            // Measure cold query
            let cold_start = Instant::now();
            let results = search_engine.search(
                query,
                10,
                Some(SearchFilters {
                    min_score: Some(0.0),
                    language: None,
                    path_pattern: None,
                })
            ).await.unwrap();
            let cold_time = cold_start.elapsed();
            
            println!("    Cold query time: {:?}", cold_time);
            println!("    Results found: {}", results.len());
            
            if !results.is_empty() {
                println!("    Top result score: {:.4}", results[0].score);
            }
            
            // Measure warm query (cached)
            let warm_start = Instant::now();
            let _ = search_engine.search(
                query,
                10,
                Some(SearchFilters {
                    min_score: Some(0.0),
                    language: None,
                    path_pattern: None,
                })
            ).await.unwrap();
            let warm_time = warm_start.elapsed();
            
            println!("    Warm query time: {:?}", warm_time);
            println!("    Cache speedup: {:.2}x", 
                cold_time.as_secs_f64() / warm_time.as_secs_f64());
            
            query_times.push(cold_time);
        }
        
        // Calculate statistics
        query_times.sort();
        let p50 = query_times[query_times.len() / 2];
        let p95 = query_times[(query_times.len() * 95 / 100).min(query_times.len() - 1)];
        let avg = query_times.iter().sum::<std::time::Duration>() / query_times.len() as u32;
        
        println!("\n📊 Phase 4: Final Performance Summary");
        println!("═══════════════════════════════════════");
        
        println!("\n  Query Latency (Using OUR System):");
        println!("  • Average: {:?}", avg);
        println!("  • P50: {:?}", p50);
        println!("  • P95: {:?}", p95);
        
        let mem_final = get_memory_mb();
        
        println!("\n  Memory Usage:");
        println!("  • Initial: {:.2} MB", mem_start);
        println!("  • After loading: {:.2} MB", mem_after_load);
        println!("  • Final: {:.2} MB", mem_final);
        println!("  • Total used: {:.2} MB", mem_final - mem_start);
        println!("  • Embeddings only: {:.2} MB", embedding_memory as f64 / 1_048_576.0);
        
        println!("\n  System Components:");
        println!("  • Search Engine: SemanticSearchEngine");
        println!("  • Embedder: AWS Titan Production");
        println!("  • Database: LanceDB");
        println!("  • Cache: Enabled");
        
        println!("\n╔══════════════════════════════════════════════════════════════════╗");
        println!("║                        RESULTS SUMMARY                            ║");
        println!("╚══════════════════════════════════════════════════════════════════╝");
        
        println!("\n  ✅ Files indexed: {}", count);
        println!("  ✅ Database size: {:.2} MB", get_dir_size(&db_path) as f64 / 1_048_576.0);
        println!("  ✅ Embeddings memory: {:.2} MB", embedding_memory as f64 / 1_048_576.0);
        println!("  ✅ Query latency (P95): {:?}", p95);
        println!("  ✅ Total memory used: {:.2} MB", mem_final - mem_start);
        
    } else {
        println!("  ❌ No indexed data found. Please run indexing first.");
    }
}

fn get_memory_mb() -> f64 {
    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
    }
    0.0
}

fn get_dir_size(path: &Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                size += get_dir_size(&path);
            } else if let Ok(metadata) = fs::metadata(&path) {
                size += metadata.len();
            }
        }
    }
    size
}
