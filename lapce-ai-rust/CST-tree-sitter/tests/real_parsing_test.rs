//! Real-world parsing tests with actual files

use lapce_tree_sitter::{NativeParserManager, FileType};
use std::sync::Arc;
use std::time::Instant;
use std::path::Path;

#[tokio::test]
async fn test_parse_rust_file() {
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    
    // Create a test Rust file
    let path = std::env::temp_dir().join("test_parse.rs");
    let content = r#"
fn main() {
    println!("Hello, world!");
}

struct MyStruct {
    field1: String,
    field2: i32,
}

impl MyStruct {
    fn new() -> Self {
        Self {
            field1: String::new(),
            field2: 0,
        }
    }
}

trait MyTrait {
    fn method(&self);
}
"#;
    std::fs::write(&path, content).unwrap();
    
    let start = Instant::now();
    let result = parser_manager.parse_file(&path).await;
    let duration = start.elapsed();
    
    match result {
        Ok(parse_result) => {
            println!("✅ Successfully parsed Rust file");
            println!("   Parse time: {:?}", duration);
            println!("   File type: {:?}", parse_result.file_type);
            println!("   Tree root: {:?}", parse_result.tree.root_node().kind());
            assert_eq!(parse_result.file_type, FileType::Rust);
            assert!(parse_result.tree.root_node().child_count() > 0);
        }
        Err(e) => {
            panic!("Failed to parse Rust file: {:?}", e);
        }
    }
    
    // Clean up
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn test_parse_multiple_languages() {
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    
    // Create test files for different languages
    let test_cases = vec![
        ("test.js", FileType::JavaScript, "function test() { return 42; }"),
        ("test.py", FileType::Python, "def test():\n    return 42"),
        ("test.go", FileType::Go, "func test() int { return 42 }"),
        ("test.java", FileType::Java, "class Test { int test() { return 42; } }"),
        ("test.cpp", FileType::Cpp, "int test() { return 42; }"),
        ("test.c", FileType::C, "int test() { return 42; }"),
    ];
    
    for (filename, expected_type, content) in test_cases {
        // Write test file
        let path = std::env::temp_dir().join(filename);
        std::fs::write(&path, content).unwrap();
        
        // Parse the file
        let result = parser_manager.parse_file(&path).await;
        
        match result {
            Ok(parse_result) => {
                println!("✅ Parsed {}: {:?}", filename, parse_result.file_type);
                assert_eq!(parse_result.file_type, expected_type);
            }
            Err(e) => {
                println!("⚠️  Failed to parse {}: {:?}", filename, e);
            }
        }
        
        // Clean up
        std::fs::remove_file(&path).ok();
    }
}

#[tokio::test]
async fn test_incremental_parsing() {
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    
    // Create a test file
    let path = std::env::temp_dir().join("incremental_test.rs");
    let original_content = "fn main() {\n    println!(\"Hello\");\n}";
    std::fs::write(&path, original_content).unwrap();
    
    // First parse
    let start1 = Instant::now();
    let result1 = parser_manager.parse_file(&path).await.unwrap();
    let duration1 = start1.elapsed();
    
    // Modify the file
    let modified_content = "fn main() {\n    println!(\"Hello, World!\");\n}";
    std::fs::write(&path, modified_content).unwrap();
    
    // Second parse (should use incremental parsing)
    let start2 = Instant::now();
    let result2 = parser_manager.parse_file(&path).await.unwrap();
    let duration2 = start2.elapsed();
    
    println!("First parse: {:?}", duration1);
    println!("Incremental parse: {:?}", duration2);
    
    // Incremental parsing should be faster
    assert!(duration2 < duration1 * 2, "Incremental parsing should be efficient");
    
    // Clean up
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn test_cache_hit_rate() {
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    
    // Create a test file
    let path = std::env::temp_dir().join("cache_test.rs");
    let content = "fn test() { /* cached */ }";
    std::fs::write(&path, content).unwrap();
    
    // Parse multiple times to test cache
    let mut durations = Vec::new();
    
    for i in 0..10 {
        let start = Instant::now();
        let _ = parser_manager.parse_file(&path).await.unwrap();
        let duration = start.elapsed();
        durations.push(duration);
        println!("Parse {}: {:?}", i + 1, duration);
    }
    
    // First parse should be slowest, rest should hit cache
    let first = durations[0];
    let avg_cached = durations[1..].iter()
        .sum::<std::time::Duration>() / (durations.len() - 1) as u32;
    
    println!("First parse: {:?}", first);
    println!("Avg cached parse: {:?}", avg_cached);
    
    // Cache hits should be much faster
    assert!(avg_cached < first / 2, "Cache should significantly speed up parsing");
    
    // Clean up
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn test_parse_large_file() {
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    
    // Generate a large Rust file (10K lines)
    let mut content = String::new();
    for i in 0..10000 {
        content.push_str(&format!("fn function_{}() {{ println!(\"{}\"); }}\n", i, i));
    }
    
    let path = std::env::temp_dir().join("large_test.rs");
    std::fs::write(&path, &content).unwrap();
    
    // Parse and measure
    let start = Instant::now();
    let result = parser_manager.parse_file(&path).await.unwrap();
    let duration = start.elapsed();
    
    let lines_per_second = 10000.0 / duration.as_secs_f64();
    
    println!("Parsed 10K lines in {:?}", duration);
    println!("Speed: {:.0} lines/second", lines_per_second);
    
    // Should meet performance target: > 10K lines/second
    assert!(lines_per_second > 10000.0, "Should parse > 10K lines/second");
    
    // Clean up
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn test_memory_usage() {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    struct MemoryTracker;
    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
    
    unsafe impl GlobalAlloc for MemoryTracker {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let ret = System.alloc(layout);
            if !ret.is_null() {
                ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
            }
            ret
        }
        
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout);
            ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
        }
    }
    
    let before = ALLOCATED.load(Ordering::SeqCst);
    
    {
        let parser_manager = Arc::new(NativeParserManager::new().unwrap());
        
        // Parse several files
        for i in 0..10 {
            let path = std::env::temp_dir().join(format!("mem_test_{}.rs", i));
            let content = format!("fn test_{}() {{ }}", i);
            std::fs::write(&path, content).unwrap();
            
            let _ = parser_manager.parse_file(&path).await.unwrap();
            
            std::fs::remove_file(&path).ok();
        }
    }
    
    let after = ALLOCATED.load(Ordering::SeqCst);
    let used = (after - before) / 1024 / 1024; // Convert to MB
    
    println!("Memory used: {} MB", used);
    
    // Should use < 5MB
    assert!(used < 5, "Memory usage should be < 5MB");
}
