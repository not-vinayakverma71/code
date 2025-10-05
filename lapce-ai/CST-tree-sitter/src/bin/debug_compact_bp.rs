//! Debug BP sequence generation

use lapce_tree_sitter::compact::{CompactTreeBuilder};
use tree_sitter::Parser;

fn main() {
    // Very simple code
    let source = b"x = 1";
    
    // Parse with Tree-sitter
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    println!("Tree-sitter tree:");
    print_tree(tree.root_node(), source, 0);
    
    // Build compact tree
    let builder = CompactTreeBuilder::new();
    let compact_tree = builder.build(&tree, source);
    
    println!("\nCompact tree:");
    println!("  Nodes: {}", compact_tree.node_count());
    
    let bp = compact_tree.bp_bitvec();
    println!("  BP length: {}", bp.len());
    
    // Print BP sequence
    print!("  BP sequence: ");
    for i in 0..bp.len().min(100) {
        print!("{}", if bp.get(i) { '(' } else { ')' });
    }
    println!();
    
    // Test root
    let root = compact_tree.root();
    println!("\nRoot node:");
    println!("  BP position: {}", root.bp_position());
    println!("  Index: {}", root.index());
    println!("  Kind: {}", root.kind());
    println!("  Child count: {}", root.child_count());
    
    // Test BP operations directly
    let bp_ops = compact_tree.bp_operations();
    println!("\nDirect BP operations:");
    println!("  first_child(0): {:?}", bp_ops.first_child(0));
    println!("  child_count(0): {}", bp_ops.child_count(0));
    
    // Check if BP has expected structure
    let opens = bp.count_ones();
    let closes = bp.count_zeros();
    println!("\nBP stats:");
    println!("  Opens: {}", opens);
    println!("  Closes: {}", closes);
    println!("  Balanced: {}", opens == closes);
}

fn print_tree(node: tree_sitter::Node, source: &[u8], depth: usize) {
    let indent = "  ".repeat(depth);
    let text = if node.child_count() == 0 {
        node.utf8_text(source).unwrap_or("?").to_string()
    } else {
        String::new()
    };
    
    println!("{}{} ({}) [{}..{}] {}", 
             indent, 
             node.kind(), 
             node.child_count(),
             node.start_byte(),
             node.end_byte(),
             text);
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_tree(child, source, depth + 1);
    }
}
