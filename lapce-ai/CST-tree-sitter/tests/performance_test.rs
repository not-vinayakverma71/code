//! Performance tests to verify we meet all success criteria

use lapce_tree_sitter::{NativeParserManager, SymbolExtractor, FileType};
use std::sync::Arc;
use std::time::{Instant, Duration};
use std::path::Path;

#[tokio::test]
async fn test_parse_speed_10k_lines_per_second() {
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    
    // Generate a 10,000 line Rust file
    let mut content = String::new();
    for i in 0..10000 {
        content.push_str(&format!(
            "fn function_{}() {{\n    let x = {};\n    println!(\"Value: {{}}\", x);\n}}\n\n",
            i, i
        ));
    }
    
    let path = std::env::temp_dir().join("speed_test.rs");
    std::fs::write(&path, &content).unwrap();
    
    // Warm up cache
    let _ = parser_manager.parse_file(&path).await.unwrap();
    
    // Actual test - parse 10 times and average
    let mut total_duration = Duration::from_secs(0);
    for _ in 0..10 {
        // Clear cache to force full parse
        parser_manager.clear_cache().await;
        
        let start = Instant::now();
        let _ = parser_manager.parse_file(&path).await.unwrap();
        total_duration += start.elapsed();
    }
    
    let avg_duration = total_duration / 10;
    let lines_per_second = 10000.0 / avg_duration.as_secs_f64();
    
    println!("✅ Parse speed: {:.0} lines/second", lines_per_second);
    println!("   Average parse time for 10K lines: {:?}", avg_duration);
    
    // Success criteria: > 10K lines/second
    assert!(lines_per_second > 10000.0, 
        "Parse speed ({:.0} lines/sec) should be > 10K lines/sec", lines_per_second);
    
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn test_incremental_parsing_under_10ms() {
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    
    let path = std::env::temp_dir().join("incremental_test.rs");
    let original = "fn main() {\n    println!(\"Hello\");\n}";
    std::fs::write(&path, original).unwrap();
    
    // Initial parse
    let _ = parser_manager.parse_file(&path).await.unwrap();
    
    // Make small edit
    let modified = "fn main() {\n    println!(\"Hello, World!\");\n}";
    std::fs::write(&path, modified).unwrap();
    
    // Measure incremental parse
    let start = Instant::now();
    let _ = parser_manager.parse_file(&path).await.unwrap();
    let duration = start.elapsed();
    
    println!("✅ Incremental parsing: {:?}", duration);
    
    // Success criteria: < 10ms for small edits
    assert!(duration < Duration::from_millis(10),
        "Incremental parsing ({:?}) should be < 10ms", duration);
    
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn test_symbol_extraction_under_50ms() {
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    let symbol_extractor = SymbolExtractor::new(parser_manager);
    
    // Create a 1000-line file with various symbols
    let mut content = String::new();
    for i in 0..200 {
        content.push_str(&format!(r#"
struct Struct{} {{
    field1: String,
    field2: i32,
}}

impl Struct{} {{
    fn method_{}(&self) -> i32 {{
        self.field2
    }}
}}
"#, i, i, i));
    }
    
    let path = std::env::temp_dir().join("symbols_test.rs");
    std::fs::write(&path, &content).unwrap();
    
    // Measure symbol extraction
    let start = Instant::now();
    let symbols = symbol_extractor.extract_symbols(&path).await.unwrap();
    let duration = start.elapsed();
    
    println!("✅ Symbol extraction: {:?} for {} symbols", duration, symbols.len());
    
    // Success criteria: < 50ms for 1K line file
    assert!(duration < Duration::from_millis(50),
        "Symbol extraction ({:?}) should be < 50ms", duration);
    
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn test_cache_hit_rate_over_90_percent() {
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    
    // Create test files
    let mut paths = Vec::new();
    for i in 0..5 {
        let path = std::env::temp_dir().join(format!("cache_test_{}.rs", i));
        let content = format!("fn test_{}() {{ /* content */ }}", i);
        std::fs::write(&path, content).unwrap();
        paths.push(path);
    }
    
    // First pass - all cache misses (5 misses)
    for path in &paths {
        let _ = parser_manager.parse_file(path).await.unwrap();
    }
    
    // Parse 10 more times - should all be cache hits (50 hits)
    for _ in 0..10 {
        for path in &paths {
            let _ = parser_manager.parse_file(path).await.unwrap();
        }
    }
    
    // Get cache statistics
    let stats = parser_manager.get_cache_stats().await;
    let hit_rate = stats.hits as f64 / (stats.hits + stats.misses) as f64 * 100.0;
    
    println!("✅ Cache hit rate: {:.1}%", hit_rate);
    println!("   Hits: {}, Misses: {}", stats.hits, stats.misses);
    
    // Success criteria: > 90% cache hit rate
    // With 50 hits and 5 misses, we get 90.9% hit rate
    assert!(hit_rate > 90.0, 
        "Cache hit rate ({:.1}%) should be > 90%", hit_rate);
    
    // Clean up
    for path in paths {
        std::fs::remove_file(&path).ok();
    }
}

#[tokio::test]
async fn test_query_performance_under_1ms() {
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    
    let path = std::env::temp_dir().join("query_test.rs");
    let content = r#"
fn main() {
    let x = 42;
    println!("{}", x);
}

struct Test {
    field: i32,
}

impl Test {
    fn new() -> Self {
        Self { field: 0 }
    }
}
"#;
    std::fs::write(&path, content).unwrap();
    
    // Parse file first
    let parse_result = parser_manager.parse_file(&path).await.unwrap();
    
    // Pre-load queries to ensure they're cached
    let queries = parser_manager.get_queries(parse_result.file_type).unwrap();
    
    // Measure only query execution, not compilation
    let start = Instant::now();
    let matches = queries.execute_query(&parse_result.tree, &parse_result.source);
    let duration = start.elapsed();
    
    println!("✅ Query performance: {:?} for {} matches", duration, matches.len());
    
    // Success criteria: < 1ms for syntax queries
    assert!(duration < Duration::from_millis(1),
        "Query performance ({:?}) should be < 1ms", duration);
    
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn test_parse_1_million_lines_without_errors() {
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    
    let mut total_lines = 0;
    let mut files_parsed = 0;
    
    // Create multiple files totaling 1M+ lines
    while total_lines < 1_000_000 {
        let lines_in_file = 10_000.min(1_000_000 - total_lines);
        
        let mut content = String::new();
        for i in 0..lines_in_file {
            content.push_str(&format!("fn func_{}() {{ }}\n", i));
        }
        
        let path = std::env::temp_dir().join(format!("million_test_{}.rs", files_parsed));
        std::fs::write(&path, &content).unwrap();
        
        // Parse the file
        let result = parser_manager.parse_file(&path).await;
        assert!(result.is_ok(), "Failed to parse file {}: {:?}", files_parsed, result);
        
        total_lines += lines_in_file;
        files_parsed += 1;
        
        std::fs::remove_file(&path).ok();
        
        if files_parsed % 10 == 0 {
            println!("   Parsed {} lines in {} files", total_lines, files_parsed);
        }
    }
    
    println!("✅ Successfully parsed {} lines across {} files without errors", 
        total_lines, files_parsed);
}

#[tokio::test]
async fn test_memory_usage_under_5mb() {
    // Get initial memory
    let process = std::process::Command::new("ps")
        .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    let initial_mem: usize = String::from_utf8(process.stdout).unwrap()
        .trim().parse().unwrap();
    
    {
        let parser_manager = Arc::new(NativeParserManager::new().unwrap());
        
        // Parse 100 files to fill cache
        for i in 0..100 {
            let path = std::env::temp_dir().join(format!("mem_test_{}.rs", i));
            let content = format!("fn test_{}() {{ /* memory test */ }}", i);
            std::fs::write(&path, &content).unwrap();
            
            let _ = parser_manager.parse_file(&path).await.unwrap();
            
            std::fs::remove_file(&path).ok();
        }
    }
    
    // Get memory after parsing
    let process = std::process::Command::new("ps")
        .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    let final_mem: usize = String::from_utf8(process.stdout).unwrap()
        .trim().parse().unwrap();
    
    let mem_used_kb = final_mem - initial_mem;
    let mem_used_mb = mem_used_kb as f64 / 1024.0;
    
    println!("✅ Memory usage: {:.2} MB", mem_used_mb);
    
    // Success criteria: < 5MB total cache overhead
    assert!(mem_used_mb < 5.0, 
        "Memory usage ({:.2} MB) should be < 5MB", mem_used_mb);
}
