fn main() {
    println!("Starting simple test...");
    
    // Test if the function exists and is accessible
    use lapce_tree_sitter::codex_exact_format::parse_source_code_definitions_for_file;
    
    // Test with simplest JavaScript
    let result = parse_source_code_definitions_for_file(
        "test.js", 
        "function test() { return 1; }"
    );
    
    match result {
        Some(output) => {
            println!("Success! Output:");
            println!("{}", output);
        }
        None => {
            println!("Failed - returned None");
        }
    }
}
