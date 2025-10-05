//! Test the integrated tree-sitter system

use lapce_tree_sitter::integrated_system::{IntegratedTreeSitter, SystemConfig};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Integrated Tree-Sitter System");
    println!("=====================================\n");
    
    // Create system
    let system = IntegratedTreeSitter::new()?;
    
    // Test 1: Parse source code
    println!("1. Testing source parsing...");
    let rust_code = r#"
fn main() {
    println!("Hello, World!");
}
    "#;
    
    let result = system.parse_source(rust_code, "rust")?;
    println!("✅ Parsed {} bytes in {:?}", result.source.len(), result.parse_time);
    
    // Test 2: Syntax highlighting
    println!("\n2. Testing syntax highlighting...");
    let highlights = system.highlight_source(rust_code, "rust")?;
    println!("✅ Found {} highlight ranges", highlights.len());
    for h in highlights.iter().take(3) {
        println!("  - {} at byte {}-{}", h.highlight.name, h.start_byte, h.end_byte);
    }
    
    // Test 3: Performance metrics
    println!("\n3. Testing performance metrics...");
    let metrics = system.get_metrics();
    println!("✅ Metrics:");
    println!("  - Memory: {:.2} MB", metrics.memory_usage_mb);
    println!("  - Avg parse time: {:.2} ms", metrics.average_parse_time_ms);
    println!("  - Cache hit rate: {:.2}%", metrics.cache_hit_rate * 100.0);
    
    // Test 4: Available themes
    println!("\n4. Testing theme support...");
    let themes = system.get_themes();
    println!("✅ Available themes: {:?}", themes);
    
    println!("\n✅ All tests passed!");
    Ok(())
}
