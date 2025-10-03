fn main() {
    println!("Step-by-step debugging...\n");
    
    // Step 1: Import and check parser
    println!("Step 1: Creating JavaScript parser");
    use tree_sitter::Parser;
    let mut parser = Parser::new();
    let lang = tree_sitter_javascript::LANGUAGE;
    parser.set_language(&lang.into()).expect("Failed to set language");
    println!("✅ Parser created\n");
    
    // Step 2: Parse simple code
    println!("Step 2: Parsing simple code");
    let code = "function test() { return 42; }";
    let tree = parser.parse(code, None).expect("Failed to parse");
    println!("✅ Tree created with {} nodes\n", tree.root_node().descendant_count());
    
    // Step 3: Check if we can access the parse function
    println!("Step 3: Testing parse_file_with_tree_sitter");
    use lapce_tree_sitter::codex_exact_format::parse_file_with_tree_sitter;
    
    match parse_file_with_tree_sitter(code, "javascript") {
        Some(result) => {
            println!("✅ parse_file_with_tree_sitter works!");
            println!("Result: {}", result);
        }
        None => {
            println!("❌ parse_file_with_tree_sitter returned None");
        }
    }
    
    println!("\nStep 4: Testing full function");
    use lapce_tree_sitter::codex_exact_format::parse_source_code_definitions_for_file;
    
    match parse_source_code_definitions_for_file("test.js", code) {
        Some(result) => {
            println!("✅ Full function works!");
            println!("Result: {}", result);
        }
        None => {
            println!("❌ Full function returned None");
        }
    }
}
