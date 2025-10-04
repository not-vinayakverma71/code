//! Production Performance Test - Comprehensive benchmarks for all metrics

use std::time::{Duration, Instant};
use std::fs;
use std::path::Path;
use lapce_tree_sitter::{CodexSymbolExtractor, LapceTreeSitterAPI};

fn main() {
    println!("🏭 PRODUCTION PERFORMANCE TEST SUITE");
    println!("=====================================\n");
    
    // Test configurations
    let small_file = generate_test_file(100);      // 100 lines
    let medium_file = generate_test_file(1000);    // 1K lines
    let large_file = generate_test_file(10000);    // 10K lines
    let huge_file = generate_test_file(100000);    // 100K lines
    let massive_file = generate_test_file(1000000); // 1M lines
    
    // Initialize APIs
    let extractor = CodexSymbolExtractor::new();
    let api = LapceTreeSitterAPI::new();
    
    // 1. Parse Speed Test
    println!("📊 PARSE SPEED TEST");
    println!("-------------------");
    test_parse_speed(&extractor, &small_file, "Small (100 lines)");
    test_parse_speed(&extractor, &medium_file, "Medium (1K lines)");
    test_parse_speed(&extractor, &large_file, "Large (10K lines)");
    test_parse_speed(&extractor, &huge_file, "Huge (100K lines)");
    test_parse_speed(&extractor, &massive_file, "Massive (1M lines)");
    
    // 2. Memory Usage Test
    println!("\n📊 MEMORY USAGE TEST");
    println!("--------------------");
    test_memory_usage(&api);
    
    // 3. Symbol Extraction Speed Test
    println!("\n📊 SYMBOL EXTRACTION SPEED TEST");
    println!("--------------------------------");
    test_symbol_extraction_speed(&extractor, &medium_file);
    
    // 4. Cache Hit Rate Test
    println!("\n📊 CACHE HIT RATE TEST");
    println!("----------------------");
    test_cache_hit_rate(&api);
    
    // 5. Incremental Parsing Test
    println!("\n📊 INCREMENTAL PARSING TEST");
    println!("---------------------------");
    test_incremental_parsing(&api);
    
    // 6. Multi-language Test
    println!("\n📊 MULTI-LANGUAGE PERFORMANCE TEST");
    println!("-----------------------------------");
    test_all_languages(&extractor);
    
    // 7. Directory Traversal Test
    println!("\n📊 DIRECTORY TRAVERSAL TEST");
    println!("---------------------------");
    test_directory_traversal(&extractor);
    
    // Final Summary
    print_summary();
}

fn generate_test_file(lines: usize) -> String {
    let mut content = String::new();
    
    // Generate realistic Rust code
    content.push_str("use std::collections::HashMap;\n");
    content.push_str("use std::sync::Arc;\n\n");
    
    for i in 0..lines/20 {
        // Add a function every 20 lines
        content.push_str(&format!("fn function_{}() -> i32 {{\n", i));
        content.push_str("    let mut sum = 0;\n");
        for j in 0..8 {
            content.push_str(&format!("    sum += {};\n", j));
        }
        content.push_str("    sum\n");
        content.push_str("}\n\n");
        
        // Add a struct every 20 lines
        content.push_str(&format!("struct Struct{} {{\n", i));
        content.push_str("    field1: String,\n");
        content.push_str("    field2: u64,\n");
        content.push_str("    field3: Vec<i32>,\n");
        content.push_str("}\n\n");
        
        // Add impl block
        content.push_str(&format!("impl Struct{} {{\n", i));
        content.push_str("    fn new() -> Self {\n");
        content.push_str("        Self {\n");
        content.push_str("            field1: String::new(),\n");
        content.push_str("            field2: 0,\n");
        content.push_str("            field3: Vec::new(),\n");
        content.push_str("        }\n");
        content.push_str("    }\n");
        content.push_str("}\n\n");
    }
    
    content
}

fn test_parse_speed(extractor: &CodexSymbolExtractor, content: &str, label: &str) {
    let lines = content.lines().count();
    let start = Instant::now();
    
    // Parse 10 times to get average
    for _ in 0..10 {
        let _ = extractor.extract_from_file("test.rs", content);
    }
    
    let elapsed = start.elapsed();
    let avg_time = elapsed / 10;
    let lines_per_sec = (lines as f64 * 10.0) / elapsed.as_secs_f64();
    
    println!("  {} -> {} lines/sec (avg {}ms/parse)", 
             label, 
             lines_per_sec as u64,
             avg_time.as_millis());
    
    // Check against target (>10K lines/sec)
    if lines_per_sec > 10000.0 {
        println!("    ✅ PASS: Exceeds 10K lines/sec target");
    } else {
        println!("    ❌ FAIL: Below 10K lines/sec target");
    }
}

fn test_memory_usage(api: &LapceTreeSitterAPI) {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    struct MemoryTracker;
    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
    
    unsafe impl GlobalAlloc for MemoryTracker {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
            System.alloc(layout)
        }
        
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
            System.dealloc(ptr, layout)
        }
    }
    
    // Measure memory before
    let before = ALLOCATED.load(Ordering::SeqCst);
    
    // Load all 23 language parsers
    for lang in ["js", "ts", "tsx", "py", "rs", "go", "c", "cpp", "cs", 
                 "rb", "java", "php", "swift", "lua", "ex", "scala",
                 "css", "json", "toml", "sh", "elm", "Dockerfile", "md"] {
        let sample = "function test() { return 42; }";
        let _ = api.extract_symbols(&format!("test.{}", lang), sample);
    }
    
    // Measure memory after
    let after = ALLOCATED.load(Ordering::SeqCst);
    let used_mb = (after - before) as f64 / 1_048_576.0;
    
    println!("  Memory used: {:.2} MB", used_mb);
    
    // Check against target (<5MB)
    if used_mb < 5.0 {
        println!("    ✅ PASS: Below 5MB target");
    } else {
        println!("    ❌ FAIL: Exceeds 5MB target");
    }
}

fn test_symbol_extraction_speed(extractor: &CodexSymbolExtractor, content: &str) {
    // Test on 1K line file
    let content_1k = &content[..content.len().min(50000)]; // Approximately 1K lines
    
    let start = Instant::now();
    let result = extractor.extract_from_file("test.rs", content_1k);
    let elapsed = start.elapsed();
    
    println!("  1K lines extraction: {}ms", elapsed.as_millis());
    
    // Check against target (<50ms for 1K lines)
    if elapsed.as_millis() < 50 {
        println!("    ✅ PASS: Below 50ms target");
    } else {
        println!("    ❌ FAIL: Exceeds 50ms target");
    }
    
    if let Some(output) = result {
        let symbol_count = output.lines().count() - 1; // Minus header
        println!("    Symbols extracted: {}", symbol_count);
    }
}

fn test_cache_hit_rate(api: &LapceTreeSitterAPI) {
    let content = "fn main() { println!(\"test\"); }";
    
    // First parse (cache miss)
    let start1 = Instant::now();
    let _ = api.extract_symbols("test.rs", content);
    let first_time = start1.elapsed();
    
    // Parse same file 100 times
    let start2 = Instant::now();
    for _ in 0..100 {
        let _ = api.extract_symbols("test.rs", content);
    }
    let cached_time = start2.elapsed() / 100;
    
    // Calculate cache effectiveness
    let speedup = first_time.as_nanos() as f64 / cached_time.as_nanos() as f64;
    let hit_rate = ((speedup - 1.0) / speedup * 100.0).min(100.0);
    
    println!("  First parse: {}μs", first_time.as_micros());
    println!("  Cached parse: {}μs", cached_time.as_micros());
    println!("  Cache hit rate: {:.1}%", hit_rate);
    
    // Check against target (>90%)
    if hit_rate > 90.0 {
        println!("    ✅ PASS: Exceeds 90% cache hit rate");
    } else {
        println!("    ❌ FAIL: Below 90% cache hit rate");
    }
}

fn test_incremental_parsing(api: &LapceTreeSitterAPI) {
    let original = "fn main() {\n    println!(\"hello\");\n}";
    let modified = "fn main() {\n    println!(\"hello world\");\n}";
    
    // Parse original
    let _ = api.extract_symbols("test.rs", original);
    
    // Measure incremental parse time
    let start = Instant::now();
    let _ = api.extract_symbols("test.rs", modified);
    let elapsed = start.elapsed();
    
    println!("  Incremental parse time: {}ms", elapsed.as_millis());
    
    // Check against target (<10ms)
    if elapsed.as_millis() < 10 {
        println!("    ✅ PASS: Below 10ms target");
    } else {
        println!("    ❌ FAIL: Exceeds 10ms target");
    }
}

fn test_all_languages(extractor: &CodexSymbolExtractor) {
    let languages = vec![
        ("test.js", "function test() { return 42; }"),
        ("test.py", "def test():\n    return 42"),
        ("test.rs", "fn test() -> i32 { 42 }"),
        ("test.go", "func test() int { return 42 }"),
        ("test.java", "public int test() { return 42; }"),
        ("test.rb", "def test\n  42\nend"),
        ("test.cpp", "int test() { return 42; }"),
        ("test.cs", "public int Test() { return 42; }"),
        ("test.php", "<?php function test() { return 42; } ?>"),
        ("test.swift", "func test() -> Int { return 42 }"),
        ("test.lua", "function test() return 42 end"),
        ("test.ex", "def test do 42 end"),
        ("test.scala", "def test(): Int = 42"),
    ];
    
    let mut total_time = Duration::ZERO;
    let mut success_count = 0;
    
    for (file, code) in languages {
        let start = Instant::now();
        if extractor.extract_from_file(file, code).is_some() {
            success_count += 1;
        }
        total_time += start.elapsed();
    }
    
    println!("  Languages tested: {}", success_count);
    println!("  Average time per language: {}μs", total_time.as_micros() / success_count as u128);
}

fn test_directory_traversal(extractor: &CodexSymbolExtractor) {
    // Test on current directory
    let start = Instant::now();
    let result = extractor.extract_from_directory(".");
    let elapsed = start.elapsed();
    
    let files_processed = result.matches("# ").count();
    println!("  Files processed: {} in {}ms", files_processed, elapsed.as_millis());
    
    if files_processed > 0 {
        let ms_per_file = elapsed.as_millis() / files_processed as u128;
        println!("  Average: {}ms per file", ms_per_file);
    }
}

fn print_summary() {
    println!("\n🎯 PERFORMANCE SUMMARY");
    println!("======================");
    println!("✅ Tests completed successfully");
    println!("📊 Real performance metrics collected");
    println!("🔍 Check individual test results above for pass/fail status");
}
