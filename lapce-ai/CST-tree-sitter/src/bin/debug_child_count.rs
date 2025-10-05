//! Debug child count issue

use lapce_tree_sitter::compact::{CompactTreeBuilder};
use tree_sitter::Parser;

fn main() {
    // Simple Rust with multiple children
    let source = b"fn main() {\n    let x = 1;\n    let y = 2;\n}";
    
    // Parse with Tree-sitter
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    // Show Tree-sitter structure
    println!("Tree-sitter tree:");
    let root = tree.root_node();
    println!("  Root: {} (children: {})", root.kind(), root.child_count());
    let mut cursor = root.walk();
    for (i, child) in root.children(&mut cursor).enumerate() {
        println!("    Child {}: {} (children: {})", i, child.kind(), child.child_count());
    }
    
    // Build compact tree
    let builder = CompactTreeBuilder::new();
    let compact_tree = builder.build(&tree, source);
    
    // Show compact structure
    println!("\nCompact tree:");
    let compact_root = compact_tree.root();
    println!("  Root: {} (children: {})", compact_root.kind(), compact_root.child_count());
    
    // Direct BP test
    let bp_ops = compact_tree.bp_operations();
    println!("\nDirect BP operations:");
    println!("  child_count(0): {}", bp_ops.child_count(0));
    println!("  first_child(0): {:?}", bp_ops.first_child(0));
    
    // Test if CompactNode methods work
    println!("\nCompactNode methods:");
    println!("  root.bp_position(): {}", compact_root.bp_position());
    println!("  root.child_count(): {}", compact_root.child_count());
    println!("  root.first_child(): {:?}", compact_root.first_child().map(|c| c.kind().to_string()));
    
    // Iterate children
    println!("\nChildren via iterator:");
    for (i, child) in compact_root.children().enumerate() {
        println!("    Child {}: {}", i, child.kind());
    }
}
