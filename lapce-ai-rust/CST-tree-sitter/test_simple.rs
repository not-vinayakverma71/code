fn main() {
    use lapce_tree_sitter::codex_exact_format::parse_source_code_definitions_for_file;
    
    let code = r#"function test() {
    return 42;
}"#;
    
    match parse_source_code_definitions_for_file("test.js", code) {
        Some(result) => println!("✅ JavaScript works:\n{}", result),
        None => println!("❌ JavaScript failed"),
    }
}
