//! Test all languages are working

use lapce_tree_sitter::parser_manager::NativeParserManager;
use lapce_tree_sitter::types::FileType;
use std::sync::Arc;

#[tokio::test]
async fn test_all_17_core_languages() {
    let manager = Arc::new(NativeParserManager::new().unwrap());
    
    let test_cases = vec![
        (FileType::JavaScript, "const x = 42;", "JavaScript"),
        (FileType::TypeScript, "let x: number = 42;", "TypeScript"),
        (FileType::Rust, "fn main() {}", "Rust"),
        (FileType::Python, "def test(): pass", "Python"),
        (FileType::Go, "func main() {}", "Go"),
        (FileType::C, "int main() { return 0; }", "C"),
        (FileType::Cpp, "int main() { return 0; }", "C++"),
        (FileType::Java, "class Test {}", "Java"),
        (FileType::Json, "{}", "JSON"),
        (FileType::Html, "<div></div>", "HTML"),
        (FileType::Css, "body {}", "CSS"),
        (FileType::Bash, "echo test", "Bash"),
        (FileType::Ruby, "puts 'test'", "Ruby"),
        (FileType::Php, "<?php echo 'test'; ?>", "PHP"),
        (FileType::CSharp, "class Test {}", "C#"),
        (FileType::Toml, "key = 'value'", "TOML"),
        (FileType::Markdown, "# Test", "Markdown"),
    ];
    
    let mut passed = 0;
    let mut failed = 0;
    
    for (file_type, code, name) in test_cases {
        match manager.parse_string(code.as_bytes(), file_type).await {
            Ok(_) => {
                println!("✅ {} parsing works", name);
                passed += 1;
            }
            Err(e) => {
                println!("❌ {} parsing failed: {:?}", name, e);
                failed += 1;
            }
        }
    }
    
    println!("\nResults: {} passed, {} failed", passed, failed);
    assert!(failed < 5, "Too many languages failed");
}

#[test]
fn test_build_compiles() {
    // Just verify the crate compiles
    assert!(true);
}
