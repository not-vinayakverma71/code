//! Comprehensive test suite for all 17 working languages

use lapce_tree_sitter::native_parser_manager::NativeParserManager;
use lapce_tree_sitter::symbol_extraction::SymbolExtractor;
use lapce_tree_sitter::native_parser_manager::FileType;
use std::sync::Arc;

#[tokio::test]
async fn test_all_17_languages_parse_and_highlight() {
    let manager = Arc::new(NativeParserManager::new().unwrap());
    let extractor = SymbolExtractor::new();
    
    // Test cases for all 17 languages with real code
    let test_cases = vec![
        (FileType::Rust, include_str!("../src/lib.rs"), "lib.rs"),
        (FileType::JavaScript, "const x = 42;\nfunction test() { return x; }", "test.js"),
        (FileType::TypeScript, "interface User { name: string; age: number; }", "test.ts"),
        (FileType::Python, "def factorial(n):\n    return 1 if n <= 1 else n * factorial(n-1)", "test.py"),
        (FileType::Go, "package main\nimport \"fmt\"\nfunc main() { fmt.Println(\"Hello\") }", "test.go"),
        (FileType::C, "#include <stdio.h>\nint main() { printf(\"Hello\\n\"); return 0; }", "test.c"),
        (FileType::Cpp, "#include <iostream>\nint main() { std::cout << \"Hello\" << std::endl; }", "test.cpp"),
        (FileType::Java, "public class Main {\n    public static void main(String[] args) {\n        System.out.println(\"Hello\");\n    }\n}", "Main.java"),
        (FileType::Json, "{\"name\": \"test\", \"version\": \"1.0.0\", \"dependencies\": {}}", "package.json"),
        (FileType::Html, "<!DOCTYPE html>\n<html><head><title>Test</title></head><body><h1>Hello</h1></body></html>", "index.html"),
        (FileType::Css, "body { margin: 0; padding: 0; }\n.container { width: 100%; max-width: 1200px; }", "style.css"),
        (FileType::Bash, "#!/bin/bash\necho \"Hello World\"\nfor i in {1..5}; do echo $i; done", "script.sh"),
        (FileType::Ruby, "class Person\n  def initialize(name)\n    @name = name\n  end\nend", "person.rb"),
        (FileType::Php, "<?php\nclass User {\n    public function __construct($name) {\n        $this->name = $name;\n    }\n}\n?>", "user.php"),
        (FileType::CSharp, "using System;\nclass Program {\n    static void Main() {\n        Console.WriteLine(\"Hello\");\n    }\n}", "Program.cs"),
        (FileType::Toml, "[package]\nname = \"test\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1.0\"", "Cargo.toml"),
        (FileType::Markdown, "# Title\n\n## Subtitle\n\n- Item 1\n- Item 2\n\n```rust\nfn main() {}\n```", "README.md"),
    ];
    
    println!("\n🧪 Testing All 17 Languages\n");
    println!("Language        | Parse | Symbols | Highlights | Status");
    println!("----------------|-------|---------|------------|-------");
    
    let mut passed = 0;
    let mut failed = 0;
    
    for (file_type, code, filename) in test_cases {
        let mut status = vec![];
        
        // Test parsing
        let parse_result = manager.parse_string(code.as_bytes(), file_type).await;
        let parse_ok = parse_result.is_ok();
        status.push(if parse_ok { "✅" } else { "❌" });
        
        // Test symbol extraction
        let symbols_ok = if let Ok(ref pr) = parse_result {
            extractor.extract_from_parse_result(pr).is_ok()
        } else {
            false
        };
        status.push(if symbols_ok { "✅" } else { "❌" });
        
        // Test highlighting (check if queries compile)
        let queries_ok = if parse_ok {
            manager.get_queries(file_type).is_ok()
        } else {
            false
        };
        status.push(if queries_ok { "✅" } else { "❌" });
        
        let all_ok = parse_ok && symbols_ok && queries_ok;
        if all_ok {
            passed += 1;
        } else {
            failed += 1;
        }
        
        println!("{:15} | {:5} | {:7} | {:10} | {}",
            format!("{:?}", file_type),
            status[0], status[1], status[2],
            if all_ok { "✅ PASS" } else { "❌ FAIL" }
        );
    }
    
    println!("\n📊 Results: {} passed, {} failed", passed, failed);
    assert_eq!(failed, 0, "All 17 languages should work");
}

#[test]
fn test_language_detection() {
    let test_cases = vec![
        ("test.rs", Some(FileType::Rust)),
        ("test.js", Some(FileType::JavaScript)),
        ("test.jsx", Some(FileType::JavaScriptReact)),
        ("test.ts", Some(FileType::TypeScript)),
        ("test.tsx", Some(FileType::TypeScriptReact)),
        ("test.py", Some(FileType::Python)),
        ("test.go", Some(FileType::Go)),
        ("test.c", Some(FileType::C)),
        ("test.cpp", Some(FileType::Cpp)),
        ("test.cc", Some(FileType::Cpp)),
        ("test.java", Some(FileType::Java)),
        ("test.json", Some(FileType::Json)),
        ("test.html", Some(FileType::Html)),
        ("test.css", Some(FileType::Css)),
        ("test.sh", Some(FileType::Bash)),
        ("test.rb", Some(FileType::Ruby)),
        ("test.php", Some(FileType::Php)),
        ("test.cs", Some(FileType::CSharp)),
        ("Cargo.toml", Some(FileType::Toml)),
        ("README.md", Some(FileType::Markdown)),
    ];
    
    for (filename, expected) in test_cases {
        let detected = filename.split('.').last()
            .and_then(FileType::from_extension);
        assert_eq!(detected, expected, "Failed to detect language for {}", filename);
    }
}

#[tokio::test]
async fn test_error_recovery() {
    let manager = Arc::new(NativeParserManager::new().unwrap());
    
    // Test with malformed code
    let malformed_cases = vec![
        (FileType::Rust, "fn main( { }", "Unclosed parenthesis"),
        (FileType::JavaScript, "function() {{{{", "Too many braces"),
        (FileType::Python, "def test(\n    pass", "Invalid indentation"),
    ];
    
    for (file_type, code, description) in malformed_cases {
        let result = manager.parse_string(code.as_bytes(), file_type).await;
        // Should parse even with errors (tree-sitter is error-tolerant)
        assert!(result.is_ok(), "{} should still parse: {:?}", description, file_type);
    }
}

#[tokio::test]
async fn test_large_file_performance() {
    let manager = Arc::new(NativeParserManager::new().unwrap());
    
    // Generate a large Rust file (10,000 lines)
    let mut large_code = String::new();
    for i in 0..2000 {
        large_code.push_str(&format!(r#"
fn function_{}() -> i32 {{
    let x = {};
    let y = x * 2;
    return y + 1;
}}
"#, i, i));
    }
    
    let start = std::time::Instant::now();
    let result = manager.parse_string(large_code.as_bytes(), FileType::Rust).await;
    let elapsed = start.elapsed();
    
    assert!(result.is_ok());
    assert!(elapsed.as_secs() < 2, "Large file parsing took too long: {:?}", elapsed);
    
    let lines = large_code.lines().count();
    let lines_per_sec = lines as f64 / elapsed.as_secs_f64();
    println!("Large file performance: {} lines/sec", lines_per_sec as u64);
    assert!(lines_per_sec > 10_000.0, "Performance below 10K lines/sec");
}
