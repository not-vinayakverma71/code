//! Test with 1 million line files

use lapce_tree_sitter::parser_manager::NativeParserManager;
use std::time::Instant;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_parse_1m_lines() {
    let manager = NativeParserManager::new().expect("Failed to create manager");
    
    // Generate 1M line file
    let test_file = PathBuf::from("/tmp/test_1m_lines.rs");
    let mut content = String::new();
    
    for i in 0..200_000 {
        content.push_str(&format!(
            "fn function_{}() {{\n    let x = {};\n    let y = x * 2;\n    println!(\"Result: {{}}\", y);\n}}\n",
            i, i
        ));
    }
    
    fs::write(&test_file, &content).expect("Failed to write test file");
    
    let start = Instant::now();
    let result = manager.parse_file_sync(&test_file);
    let elapsed = start.elapsed();
    
    println!("Parsed 1M lines in {:?}", elapsed);
    println!("Lines per second: {}", 1_000_000.0 / elapsed.as_secs_f64());
    
    assert!(result.is_ok(), "Should parse successfully");
    assert!(elapsed.as_secs() < 100, "Should parse 1M lines in under 100 seconds");
    
    // Clean up
    fs::remove_file(test_file).ok();
}
