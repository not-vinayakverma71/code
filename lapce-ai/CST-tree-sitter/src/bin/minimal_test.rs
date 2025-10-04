fn main() {
    println!("Testing basic functionality...");
    
    // Test 1: Can we import the module?
    use lapce_tree_sitter::codex_exact_format;
    println!("✅ Module imported");
    
    // Test 2: Simple JavaScript code
    let js_code = "function test() { return 42; }";
    println!("Testing: {}", js_code);
    
    // Test 3: Call the function
    match codex_exact_format::parse_source_code_definitions_for_file("test.js", js_code) {
        Some(output) => {
            println!("✅ SUCCESS!");
            println!("Output:\n{}", output);
        }
        None => {
            println!("❌ Returned None");
        }
    }
}
