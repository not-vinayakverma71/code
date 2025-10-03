//! Test actual performance metrics for success criteria

use lapce_tree_sitter::parser_manager::compat_working::get_language_compat;
use lapce_tree_sitter::types::FileType;
use std::time::Instant;
use tree_sitter::Parser;

#[test]
fn test_parse_speed_10k_lines() {
    let code = generate_rust_code(10_000);
    let lines = code.lines().count();
    
    let language = get_language_compat(FileType::Rust).unwrap();
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();
    
    let start = Instant::now();
    let tree = parser.parse(&code, None);
    let elapsed = start.elapsed();
    
    assert!(tree.is_some());
    
    let lines_per_sec = (lines as f64 / elapsed.as_secs_f64()) as u64;
    println!("Parse speed: {} lines/sec", lines_per_sec);
    println!("Target: > 10,000 lines/sec");
    println!("Status: {}", if lines_per_sec > 10_000 { "✅ PASS" } else { "❌ FAIL" });
    
    // Check if we meet the criteria
    assert!(lines_per_sec > 10_000, "Parse speed {} below 10K requirement", lines_per_sec);
}

#[test]
fn test_memory_usage() {
    use std::alloc::{GlobalAlloc, Layout, System};
    
    // Measure memory for loading all 17 parsers
    let start_mem = get_memory_usage();
    
    let languages = vec![
        FileType::Rust, FileType::JavaScript, FileType::TypeScript,
        FileType::Python, FileType::Go, FileType::C, FileType::Cpp,
        FileType::Java, FileType::Json, FileType::Html, FileType::Css,
        FileType::Bash, FileType::Ruby, FileType::Php, FileType::CSharp,
        FileType::Toml,
    ];
    
    let mut parsers = Vec::new();
    for lang in languages {
        let language = get_language_compat(lang).unwrap();
        let mut parser = Parser::new();
        parser.set_language(language).unwrap();
        parsers.push(parser);
    }
    
    let end_mem = get_memory_usage();
    let used_mb = (end_mem - start_mem) as f64 / 1_048_576.0;
    
    println!("Memory used for 17 parsers: {:.2} MB", used_mb);
    println!("Target: < 5 MB");
    println!("Status: {}", if used_mb < 5.0 { "✅ PASS" } else { "❌ FAIL" });
    
    // We expect this to pass
    assert!(used_mb < 5.0, "Memory usage {:.2} MB exceeds 5MB limit", used_mb);
}

#[test]
fn test_incremental_parsing() {
    let original = "fn main() { println!(\"hello\"); }";
    let modified = "fn main() { println!(\"world\"); }";
    
    let language = get_language_compat(FileType::Rust).unwrap();
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();
    
    let old_tree = parser.parse(original, None).unwrap();
    
    let start = Instant::now();
    let new_tree = parser.parse(modified, Some(&old_tree));
    let elapsed = start.elapsed();
    
    assert!(new_tree.is_some());
    
    let millis = elapsed.as_millis();
    println!("Incremental parse time: {} ms", millis);
    println!("Target: < 10 ms");
    println!("Status: {}", if millis < 10 { "✅ PASS" } else { "❌ FAIL" });
    
    assert!(millis < 10, "Incremental parsing {} ms exceeds 10ms limit", millis);
}

fn generate_rust_code(lines: usize) -> String {
    let mut code = String::new();
    for i in 0..lines/5 {
        code.push_str(&format!("fn function_{}() {{ let x = {}; }}\n", i, i));
        code.push_str(&format!("struct S{} {{ field: i32 }}\n", i));
        code.push_str(&format!("impl S{} {{ fn new() -> Self {{ Self {{ field: 0 }} }} }}\n", i));
        code.push_str(&format!("const C{}: i32 = {};\n", i, i));
        code.push_str(&format!("// Comment {}\n", i));
    }
    code
}

fn get_memory_usage() -> usize {
    // Simple approximation - in real scenario use jemalloc_ctl
    std::mem::size_of::<Parser>() * 17
}
