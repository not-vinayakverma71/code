fn main() {
    println!("Debug Test - Checking each parser");
    
    // Try to create each parser directly
    use tree_sitter::Parser;
    
    // Test 1: JavaScript
    print!("JavaScript parser: ");
    let mut parser = Parser::new();
    match parser.set_language(&tree_sitter_javascript::language().into()) {
        Ok(_) => println!("✅"),
        Err(e) => println!("❌ {}", e),
    }
    
    // Test 2: TypeScript
    print!("TypeScript parser: ");
    let mut parser = Parser::new();
    match parser.set_language(&tree_sitter_typescript::language_typescript().into()) {
        Ok(_) => println!("✅"),
        Err(e) => println!("❌ {}", e),
    }
    
    // Test 3: Python
    print!("Python parser: ");
    let mut parser = Parser::new();
    match parser.set_language(&tree_sitter_python::LANGUAGE.into()) {
        Ok(_) => println!("✅"),
        Err(e) => println!("❌ {}", e),
    }
    
    // Test 4: Rust
    print!("Rust parser: ");
    let mut parser = Parser::new();
    match parser.set_language(&tree_sitter_rust::LANGUAGE.into()) {
        Ok(_) => println!("✅"),
        Err(e) => println!("❌ {}", e),
    }
    
    // Test 5: Go
    print!("Go parser: ");
    let mut parser = Parser::new();
    match parser.set_language(&tree_sitter_go::LANGUAGE.into()) {
        Ok(_) => println!("✅"),
        Err(e) => println!("❌ {}", e),
    }
    
    println!("\nNow testing parse function:");
    
    // Test the actual parse function
    use lapce_tree_sitter::codex_exact_format::parse_source_code_definitions_for_file;
    
    let js_code = "function test() { return 42; }";
    println!("\nTesting JS code: {}", js_code);
    
    match parse_source_code_definitions_for_file("test.js", js_code) {
        Some(result) => {
            println!("Result: {}", result);
        }
        None => {
            println!("Returned None");
        }
    }
}
