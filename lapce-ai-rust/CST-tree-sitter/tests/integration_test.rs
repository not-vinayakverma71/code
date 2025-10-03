//! Integration test to verify parsing is working

use lapce_tree_sitter::NativeParserManager;
use std::sync::Arc;
use tempfile::NamedTempFile;
use std::io::Write;

#[tokio::test]
async fn test_parse_rust_file() {
    let rust_code = r#"
fn main() {
    println!("Hello, world!");
}
"#;

    let mut temp_file = NamedTempFile::with_suffix(".rs").unwrap();
    temp_file.write_all(rust_code.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    let result = parser_manager.parse_file(temp_file.path()).await;
    
    assert!(result.is_ok(), "Should parse Rust file successfully");
    let parse_result = result.unwrap();
    assert_eq!(parse_result.file_type, lapce_tree_sitter::FileType::Rust);
    assert!(parse_result.parse_time.as_millis() < 100, "Parse should be fast");
}

#[tokio::test]
async fn test_parse_javascript_file() {
    let js_code = r#"
function hello() {
    console.log("Hello, world!");
}
"#;

    let mut temp_file = NamedTempFile::with_suffix(".js").unwrap();
    temp_file.write_all(js_code.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    let result = parser_manager.parse_file(temp_file.path()).await;
    
    assert!(result.is_ok(), "Should parse JavaScript file successfully");
    let parse_result = result.unwrap();
    assert_eq!(parse_result.file_type, lapce_tree_sitter::FileType::JavaScript);
}

#[tokio::test]
async fn test_parse_python_file() {
    let python_code = r#"
def hello():
    print("Hello, world!")

if __name__ == "__main__":
    hello()
"#;

    let mut temp_file = NamedTempFile::with_suffix(".py").unwrap();
    temp_file.write_all(python_code.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    let result = parser_manager.parse_file(temp_file.path()).await;
    
    assert!(result.is_ok(), "Should parse Python file successfully");
    let parse_result = result.unwrap();
    assert_eq!(parse_result.file_type, lapce_tree_sitter::FileType::Python);
}

#[tokio::test]
async fn test_incremental_parsing() {
    // Test that cache works for incremental parsing
    let code = r#"
fn main() {
    let x = 42;
    println!("{}", x);
}
"#;

    let mut temp_file = NamedTempFile::with_suffix(".rs").unwrap();
    temp_file.write_all(code.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    
    // Parse once to populate cache
    let result1 = parser_manager.parse_file(temp_file.path()).await.unwrap();
    
    // Parse again - should hit cache
    let result2 = parser_manager.parse_file(temp_file.path()).await.unwrap();
    
    // Cache hit should be faster
    assert!(result2.parse_time < result1.parse_time || result2.parse_time.as_millis() == 0, 
            "Second parse should be faster due to cache");
}

#[tokio::test] 
async fn test_large_file_performance() {
    // Generate a large Rust file
    let mut code = String::new();
    for i in 0..1000 {
        code.push_str(&format!("fn function_{}() {{ println!(\"Function {}\"); }}\n", i, i));
    }
    
    let mut temp_file = NamedTempFile::with_suffix(".rs").unwrap();
    temp_file.write_all(code.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    let start = std::time::Instant::now();
    let result = parser_manager.parse_file(temp_file.path()).await;
    let elapsed = start.elapsed();
    
    assert!(result.is_ok(), "Should parse large file successfully");
    
    // Check performance - should parse > 10K lines/second
    let lines = code.lines().count();
    let lines_per_second = lines as f64 / elapsed.as_secs_f64();
    println!("Parsed {} lines in {:?} ({:.0} lines/second)", lines, elapsed, lines_per_second);
    assert!(lines_per_second > 10000.0, "Should parse > 10K lines/second");
}
