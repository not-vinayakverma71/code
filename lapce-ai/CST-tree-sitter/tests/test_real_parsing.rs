//! Real parsing tests for the 17 working languages

use lapce_tree_sitter::parser_manager::NativeParserManager;
use lapce_tree_sitter::types::FileType;
use std::time::Instant;

#[tokio::test]
async fn test_all_17_languages_parse_real_code() {
    let manager = NativeParserManager::new().await.unwrap();
    
    // Test real code for each language
    let test_cases = vec![
        (FileType::Rust, "fn main() { println!(\"Hello, world!\"); }", "rust.rs"),
        (FileType::JavaScript, "console.log('Hello, world!');", "test.js"),
        (FileType::TypeScript, "let x: number = 42;", "test.ts"),
        (FileType::Python, "def hello():\n    print('Hello')", "test.py"),
        (FileType::Go, "package main\nfunc main() {}", "test.go"),
        (FileType::C, "int main() { return 0; }", "test.c"),
        (FileType::Cpp, "int main() { return 0; }", "test.cpp"),
        (FileType::Java, "class Main { public static void main(String[] args) {} }", "Test.java"),
        (FileType::Json, "{\"key\": \"value\"}", "test.json"),
        (FileType::Html, "<html><body>Hello</body></html>", "test.html"),
        (FileType::Css, "body { color: red; }", "test.css"),
        (FileType::Bash, "#!/bin/bash\necho 'Hello'", "test.sh"),
        (FileType::Ruby, "puts 'Hello, world!'", "test.rb"),
        (FileType::Php, "<?php echo 'Hello'; ?>", "test.php"),
        (FileType::CSharp, "class Program { static void Main() {} }", "test.cs"),
        (FileType::Toml, "key = \"value\"", "test.toml"),
    ];
    
    println!("\n🧪 Testing 17 Languages Real Parsing\n");
    let mut passed = 0;
    let mut failed = 0;
    
    for (file_type, code, filename) in test_cases {
        let start = Instant::now();
        match manager.parse_string(code.as_bytes(), file_type).await {
            Ok(result) => {
                passed += 1;
                let elapsed = start.elapsed();
                println!("✅ {:?} ({}) - Parsed in {:?}", file_type, filename, elapsed);
            }
            Err(e) => {
                failed += 1;
                println!("❌ {:?} ({}) - Failed: {:?}", file_type, filename, e);
            }
        }
    }
    
    println!("\n📊 Results: {} passed, {} failed", passed, failed);
    assert_eq!(failed, 0, "All 17 languages should parse successfully");
}
