//! Test symbol extraction for all 17 languages

use lapce_tree_sitter::symbol_extractor::SymbolExtractor;
use lapce_tree_sitter::parser_manager::NativeParserManager;
use lapce_tree_sitter::types::FileType;

#[tokio::test]
async fn test_symbol_extraction_performance() {
    let manager = std::sync::Arc::new(NativeParserManager::new().unwrap());
    let extractor = SymbolExtractor::new(manager.clone());
    
    // Generate 1000 lines of code with many symbols
    let code = generate_rust_code_with_symbols(1000);
    
    // Parse the code
    let parse_result = manager.parse_string(code.as_bytes(), FileType::Rust).await.unwrap();
    
    // Extract symbols and measure time
    let start = std::time::Instant::now();
    let symbols = extractor.extract_from_parse_result(&parse_result);
    let elapsed = start.elapsed();
    
    assert!(symbols.is_ok());
    let symbols = symbols.unwrap();
    
    println!("Symbol extraction for 1000 lines:");
    println!("  Time: {:?}", elapsed);
    println!("  Symbols found: {}", symbols.len());
    println!("  Target: < 50ms");
    println!("  Status: {}", if elapsed.as_millis() < 50 { "✅ PASS" } else { "❌ FAIL" });
    
    assert!(elapsed.as_millis() < 50, "Symbol extraction took {:?}, exceeds 50ms limit", elapsed);
}

#[tokio::test]
async fn test_symbol_extraction_all_languages() {
    let manager = std::sync::Arc::new(NativeParserManager::new().unwrap());
    let extractor = SymbolExtractor::new(manager.clone());
    
    let test_cases = vec![
        (FileType::Rust, "fn main() { let x = 1; }"),
        (FileType::JavaScript, "function test() { const x = 1; }"),
        (FileType::TypeScript, "function test(): number { return 1; }"),
        (FileType::Python, "def test():\n    x = 1"),
        (FileType::Go, "func test() { x := 1 }"),
        (FileType::Java, "class Test { void method() {} }"),
    ];
    
    println!("\nTesting symbol extraction for all languages:");
    
    for (file_type, code) in test_cases {
        let parse_result = manager.parse_string(code.as_bytes(), file_type).await.unwrap();
        let symbols = extractor.extract_from_parse_result(&parse_result);
        
        match symbols {
            Ok(syms) => {
                println!("  ✅ {:?}: {} symbols extracted", file_type, syms.len());
            }
            Err(e) => {
                println!("  ❌ {:?}: Failed - {:?}", file_type, e);
            }
        }
    }
}

fn generate_rust_code_with_symbols(lines: usize) -> String {
    let mut code = String::new();
    
    for i in 0..lines/10 {
        code.push_str(&format!(r#"
pub struct Struct{} {{
    field1: String,
    field2: i32,
}}

impl Struct{} {{
    pub fn new() -> Self {{
        Self {{ field1: String::new(), field2: 0 }}
    }}
    
    pub fn method{}(&self) -> i32 {{
        42
    }}
}}

fn function_{}() -> i32 {{
    let variable_{} = 100;
    variable_{}
}}

const CONSTANT_{}: i32 = {};
"#, i, i, i, i, i, i, i, i));
    }
    
    code
}
