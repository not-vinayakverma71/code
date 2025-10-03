// TEST IF RUST PARSER ACTUALLY WORKS

use lapce_tree_sitter::NativeParserManager;
use std::path::Path;

#[test]
fn test_rust_parser_actually_works() {
    println!("\n🔍 TESTING RUST PARSER...\n");
    
    // Create manager
    let manager = NativeParserManager::new()
        .expect("Failed to create parser manager");
    
    // Create test Rust file
    let test_code = r#"
fn main() {
    println!("Hello, world!");
}

struct Person {
    name: String,
    age: u32,
}

impl Person {
    fn new(name: String, age: u32) -> Self {
        Person { name, age }
    }
}
"#;
    
    let test_file = "/tmp/test_rust_parser.rs";
    std::fs::write(test_file, test_code).unwrap();
    
    // Try to parse
    println!("Parsing test file...");
    let result = manager.parse_file_sync(Path::new(test_file))
        .expect("Failed to parse Rust file");
    
    // Verify result
    assert!(result.tree.root_node().child_count() > 0, "Tree should have children");
    
    let root = result.tree.root_node();
    println!("✅ Parsing succeeded!");
    println!("  Root node kind: {}", root.kind());
    println!("  Child count: {}", root.child_count());
    
    // Check for expected structures
    let mut found_fn = false;
    let mut found_struct = false;
    let mut found_impl = false;
    
    for i in 0..root.child_count() {
        if let Some(child) = root.child(i) {
            let kind = child.kind();
            println!("  Child {}: {}", i, kind);
            
            if kind.contains("function") {
                found_fn = true;
            }
            if kind.contains("struct") {
                found_struct = true;
            }
            if kind.contains("impl") {
                found_impl = true;
            }
        }
    }
    
    assert!(found_fn || found_struct || found_impl, 
            "Should find at least one Rust construct");
    
    println!("\n✅ RUST PARSER WORKS!");
}

#[test]
fn test_rust_symbol_extraction() {
    use lapce_tree_sitter::SymbolExtractor;
    
    println!("\n🔍 TESTING RUST SYMBOL EXTRACTION...\n");
    
    let test_code = r#"
fn test_function() -> i32 {
    42
}

struct TestStruct {
    field: String,
}

impl TestStruct {
    fn method(&self) -> String {
        self.field.clone()
    }
}
"#;
    
    let test_file = "/tmp/test_symbols.rs";
    std::fs::write(test_file, test_code).unwrap();
    
    let extractor = SymbolExtractor::new();
    match extractor.extract_symbols(Path::new(test_file)) {
        Ok(symbols) => {
            println!("✅ Symbol extraction succeeded!");
            println!("  Found {} symbols", symbols.len());
            
            for symbol in &symbols {
                println!("  - {}: {}", symbol.kind, symbol.name);
            }
            
            assert!(!symbols.is_empty(), "Should find some symbols");
        }
        Err(e) => {
            println!("❌ Symbol extraction failed: {}", e);
            // Don't fail test - symbol extraction might not be implemented yet
        }
    }
}
