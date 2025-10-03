// SIMPLE TEST - DOES RUST PARSING WORK AT ALL?

#[test]
fn test_rust_parser_basic() {
    use tree_sitter::Parser;
    
    println!("\n=== TESTING IF RUST PARSER WORKS ===\n");
    
    // Try to create a Rust parser directly
    let mut parser = Parser::new();
    let language = unsafe { tree_sitter_rust::language() };
    
    parser.set_language(language)
        .expect("Failed to set Rust language");
    
    // Parse simple Rust code
    let code = "fn main() { println!(\"hello\"); }";
    let tree = parser.parse(code, None)
        .expect("Failed to parse Rust code");
    
    // Check the result
    let root = tree.root_node();
    println!("✅ RUST PARSING WORKS!");
    println!("  Root kind: {}", root.kind());
    println!("  Children: {}", root.child_count());
    
    assert_eq!(root.kind(), "source_file");
    assert!(root.child_count() > 0);
    
    // Print tree structure
    fn print_tree(node: tree_sitter::Node, indent: usize) {
        println!("{}{} [{}..{}]", 
            "  ".repeat(indent), 
            node.kind(),
            node.start_byte(),
            node.end_byte());
        
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                print_tree(child, indent + 1);
            }
        }
    }
    
    println!("\nTree structure:");
    print_tree(root, 0);
    
    println!("\n✅ RUST PARSER VERIFIED WORKING!");
}
