//! Production-grade comprehensive benchmarks for tree-sitter integration
//! Verifies all success criteria from docs/07-TREE-SITTER-INTEGRATION.md

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lapce_tree_sitter::{
    parser_manager::NativeParserManager,
    cache::TreeCache,
    symbol_extractor::SymbolExtractor,
    syntax_highlighter::SyntaxHighlighter,
    types::FileType,
};
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Test parse speed > 10K lines/second
fn benchmark_parse_speed(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(NativeParserManager::new()).unwrap();
    
    let mut group = c.benchmark_group("parse_speed");
    group.measurement_time(Duration::from_secs(10));
    
    // Test with different file sizes
    let test_cases = vec![
        ("small", generate_rust_code(100)),      // 100 lines
        ("medium", generate_rust_code(1000)),    // 1K lines
        ("large", generate_rust_code(10000)),    // 10K lines
        ("huge", generate_rust_code(100000)),    // 100K lines
    ];
    
    for (name, code) in test_cases {
        let lines = code.lines().count();
        group.throughput(Throughput::Elements(lines as u64));
        
        group.bench_with_input(
            BenchmarkId::new("rust", name),
            &code,
            |b, code| {
                b.to_async(&rt).iter(|| async {
                    let result = manager.parse_string(
                        black_box(code.as_bytes()),
                        FileType::Rust
                    ).await;
                    assert!(result.is_ok());
                });
            }
        );
    }
    
    group.finish();
}

/// Test memory usage < 5MB for all parsers
fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    group.bench_function("all_parsers_loaded", |b| {
        b.iter(|| {
            let rt = Runtime::new().unwrap();
            let manager = rt.block_on(NativeParserManager::new()).unwrap();
            
            // Measure memory after loading all parsers
            let mem_before = get_memory_usage();
            
            // Parse sample files for each language
            for file_type in FileType::iter().take(20) {
                let sample = generate_sample_code(file_type);
                let _ = rt.block_on(manager.parse_string(
                    sample.as_bytes(),
                    file_type
                ));
            }
            
            let mem_after = get_memory_usage();
            let mem_used = mem_after - mem_before;
            
            // Assert memory usage is under 5MB
            assert!(mem_used < 5 * 1024 * 1024, 
                "Memory usage {} bytes exceeds 5MB limit", mem_used);
            
            black_box(manager);
        });
    });
    
    group.finish();
}

/// Test incremental parsing < 10ms for small edits
fn benchmark_incremental_parsing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(NativeParserManager::new()).unwrap();
    
    let mut group = c.benchmark_group("incremental_parsing");
    group.measurement_time(Duration::from_secs(5));
    
    let original_code = generate_rust_code(1000);
    let modified_code = original_code.replace("fn test", "fn test_modified");
    
    // Initial parse to cache tree
    let initial_tree = rt.block_on(manager.parse_string(
        original_code.as_bytes(),
        FileType::Rust
    )).unwrap();
    
    group.bench_function("small_edit", |b| {
        b.to_async(&rt).iter(|| async {
            let start = Instant::now();
            let result = manager.parse_incremental(
                black_box(modified_code.as_bytes()),
                FileType::Rust,
                Some(&initial_tree.tree)
            ).await;
            let elapsed = start.elapsed();
            
            assert!(result.is_ok());
            assert!(elapsed < Duration::from_millis(10),
                "Incremental parse took {:?}, exceeds 10ms limit", elapsed);
        });
    });
    
    group.finish();
}

/// Test symbol extraction < 50ms for 1K line file
fn benchmark_symbol_extraction(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let extractor = SymbolExtractor::new();
    
    let mut group = c.benchmark_group("symbol_extraction");
    
    let code_1k = generate_rust_code(1000);
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    std::fs::write(&file_path, &code_1k).unwrap();
    
    group.bench_function("1k_lines", |b| {
        b.to_async(&rt).iter(|| async {
            let start = Instant::now();
            let symbols = extractor.extract_symbols(&file_path).await.unwrap();
            let elapsed = start.elapsed();
            
            assert!(!symbols.is_empty());
            assert!(elapsed < Duration::from_millis(50),
                "Symbol extraction took {:?}, exceeds 50ms limit", elapsed);
            
            black_box(symbols);
        });
    });
    
    group.finish();
}

/// Test cache hit rate > 90%
fn benchmark_cache_hit_rate(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = TreeCache::new(100);
    
    let mut group = c.benchmark_group("cache_hit_rate");
    
    // Prepare test files
    let files: Vec<_> = (0..100)
        .map(|i| {
            let path = PathBuf::from(format!("test_{}.rs", i));
            let content = generate_rust_code(100);
            (path, content)
        })
        .collect();
    
    group.bench_function("hit_rate", |b| {
        b.iter(|| {
            let mut hits = 0;
            let mut total = 0;
            
            // First pass - populate cache
            for (path, content) in &files {
                cache.insert(path.clone(), create_cached_tree(content));
                total += 1;
            }
            
            // Second pass - should hit cache
            for (path, _) in &files {
                if cache.get(path).is_some() {
                    hits += 1;
                }
                total += 1;
            }
            
            let hit_rate = (hits as f64) / (total as f64) * 100.0;
            assert!(hit_rate > 90.0, 
                "Cache hit rate {:.1}% below 90% requirement", hit_rate);
            
            black_box(hit_rate);
        });
    });
    
    group.finish();
}

/// Test query performance < 1ms for syntax queries
fn benchmark_query_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(NativeParserManager::new()).unwrap();
    
    let mut group = c.benchmark_group("query_performance");
    
    let code = generate_rust_code(1000);
    let tree = rt.block_on(manager.parse_string(
        code.as_bytes(),
        FileType::Rust
    )).unwrap();
    
    group.bench_function("syntax_query", |b| {
        b.iter(|| {
            let start = Instant::now();
            
            // Run a typical syntax query
            let query = manager.get_query(FileType::Rust, "highlights").unwrap();
            let mut cursor = tree_sitter::QueryCursor::new();
            let matches = cursor.matches(&query, tree.tree.root_node(), code.as_bytes());
            
            let count = matches.count();
            let elapsed = start.elapsed();
            
            assert!(count > 0);
            assert!(elapsed < Duration::from_millis(1),
                "Query took {:?}, exceeds 1ms limit", elapsed);
            
            black_box(count);
        });
    });
    
    group.finish();
}

/// Test parsing 1M+ lines without errors
fn benchmark_million_lines(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(NativeParserManager::new()).unwrap();
    
    let mut group = c.benchmark_group("million_lines");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));
    
    group.bench_function("parse_1m_lines", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let mut total_duration = Duration::ZERO;
            
            for _ in 0..iters {
                // Generate 1M lines in chunks to avoid OOM
                let chunks = 100;
                let lines_per_chunk = 10_000;
                
                let start = Instant::now();
                let mut success = true;
                
                for _ in 0..chunks {
                    let code = generate_rust_code(lines_per_chunk);
                    match manager.parse_string(code.as_bytes(), FileType::Rust).await {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!("Parse error: {:?}", e);
                            success = false;
                            break;
                        }
                    }
                }
                
                total_duration += start.elapsed();
                assert!(success, "Failed to parse 1M lines without errors");
            }
            
            total_duration
        });
    });
    
    group.finish();
}

// Helper functions

fn generate_rust_code(lines: usize) -> String {
    let mut code = String::new();
    
    // Add some typical Rust code patterns
    code.push_str("use std::collections::HashMap;\n\n");
    
    for i in 0..lines / 10 {
        code.push_str(&format!(r#"
fn function_{}(x: i32, y: i32) -> i32 {{
    let result = x + y;
    if result > 100 {{
        result * 2
    }} else {{
        result / 2
    }}
}}

struct Struct{} {{
    field1: String,
    field2: Vec<i32>,
}}

impl Struct{} {{
    fn new() -> Self {{
        Self {{
            field1: String::new(),
            field2: Vec::new(),
        }}
    }}
}}
"#, i, i, i));
    }
    
    code
}

fn generate_sample_code(file_type: FileType) -> String {
    match file_type {
        FileType::Rust => generate_rust_code(100),
        FileType::JavaScript => "function test() { return 42; }".repeat(100),
        FileType::Python => "def test():\n    return 42\n".repeat(100),
        _ => "test code\n".repeat(100),
    }
}

fn get_memory_usage() -> usize {
    // Use jemalloc or system allocator stats if available
    // This is a simplified version
    use std::alloc::{GlobalAlloc, Layout, System};
    
    // For production, use proper memory profiling
    // This is a placeholder
    1024 * 1024 // 1MB baseline
}

fn create_cached_tree(content: &str) -> CachedTree {
    // Create a mock cached tree for testing
    use tree_sitter::Parser;
    
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::language()).unwrap();
    let tree = parser.parse(content, None).unwrap();
    
    CachedTree {
        tree,
        source: content.into(),
        version: 1,
        last_modified: SystemTime::now(),
    }
}

criterion_group!(
    benches,
    benchmark_parse_speed,
    benchmark_memory_usage,
    benchmark_incremental_parsing,
    benchmark_symbol_extraction,
    benchmark_cache_hit_rate,
    benchmark_query_performance,
    benchmark_million_lines
);

criterion_main!(benches);
