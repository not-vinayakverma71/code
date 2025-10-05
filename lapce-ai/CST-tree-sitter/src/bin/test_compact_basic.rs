//! Basic test for compact tree functionality

use lapce_tree_sitter::compact::{CompactTreeBuilder};
use tree_sitter::Parser;

fn main() {
    println!("Testing basic compact tree functionality...\n");
    
    // Simple Rust code
    let source = b"fn main() {\n    println!(\"Hello, world!\");\n}";
    
    // Parse with Tree-sitter
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    println!("Original Tree-sitter tree:");
    println!("  Root: {}", tree.root_node().kind());
    println!("  Children: {}", tree.root_node().child_count());
    println!("  Source size: {} bytes", source.len());
    
    // Build compact tree
    let builder = CompactTreeBuilder::new();
    let compact_tree = builder.build(&tree, source);
    
    println!("\nCompact tree:");
    println!("  Nodes: {}", compact_tree.node_count());
    println!("  Memory: {} bytes", compact_tree.memory_bytes());
    println!("  Bytes/node: {:.2}", compact_tree.bytes_per_node());
    
    // Validate structure
    match compact_tree.validate() {
        Ok(()) => println!("  ✅ Structure valid"),
        Err(e) => println!("  ❌ Structure invalid: {}", e),
    }
    
    // Test navigation
    let root = compact_tree.root();
    println!("\nRoot node:");
    println!("  Kind: {}", root.kind());
    println!("  Children: {}", root.child_count());
    println!("  Range: {}..{}", root.start_byte(), root.end_byte());
    
    // Walk children
    println!("\nChildren:");
    for (i, child) in root.children().enumerate() {
        println!("  {}: {} ({}..{})", 
                 i, child.kind(), child.start_byte(), child.end_byte());
    }
    
    // Memory comparison
    let ts_memory_estimate = tree.root_node().descendant_count() * 90; // ~90 bytes per TS node
    let compact_memory = compact_tree.memory_bytes();
    let compression = ts_memory_estimate as f64 / compact_memory as f64;
    
    println!("\nMemory comparison:");
    println!("  Tree-sitter (est): {} bytes", ts_memory_estimate);
    println!("  Compact: {} bytes", compact_memory);
    println!("  Compression: {:.2}x", compression);
    
    if compression > 5.0 {
        println!("\n✅ Success! Achieved {:.1}x compression!", compression);
    } else {
        println!("\n⚠️ Compression below target: {:.1}x (target: >5x)", compression);
    }
}
