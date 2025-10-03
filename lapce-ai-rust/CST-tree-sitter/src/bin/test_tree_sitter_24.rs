use std::time::Instant;
use tree_sitter::{Parser, Language};

fn main() {
    println!("🚀 Tree-Sitter 0.24 Quick Test");
    println!("================================");
    
    // Test JavaScript
    let js_lang = unsafe { tree_sitter_javascript::LANGUAGE };
    test_language("JavaScript", js_lang.into(), "function test() { return 42; }");
    
    // Test Rust  
    let rust_lang = unsafe { tree_sitter_rust::LANGUAGE };
    test_language("Rust", rust_lang.into(), "fn main() { println!(\"Hello\"); }");
    
    // Test Python
    let py_lang = unsafe { tree_sitter_python::LANGUAGE };
    test_language("Python", py_lang.into(), "def test():\n    return 42");
}

fn test_language(name: &str, language: Language, code: &str) {
    let mut parser = Parser::new();
    
    match parser.set_language(&language) {
        Ok(_) => {
            let start = Instant::now();
            match parser.parse(code, None) {
                Some(tree) => {
                    let elapsed = start.elapsed();
                    println!("✅ {} - Parsed in {:.2}ms, nodes: {}", 
                             name, 
                             elapsed.as_secs_f64() * 1000.0,
                             tree.root_node().descendant_count());
                }
                None => println!("❌ {} - Failed to parse", name),
            }
        }
        Err(e) => println!("❌ {} - Failed to set language: {:?}", name, e),
    }
}
