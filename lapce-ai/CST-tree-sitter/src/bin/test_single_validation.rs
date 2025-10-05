//! Test validation on a single file

use lapce_tree_sitter::compact::{CompactTreeBuilder};
use tree_sitter::{Parser, Node};
use std::fs;

fn main() {
    // Read a test file
    let path = "/home/verma/lapce/lapce-ai/massive_test_codebase/module_9/submodule_0/module_901.py";
    let source = fs::read(path).expect("Failed to read file");
    
    // Parse with Tree-sitter
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into()).expect("Failed to set language");
    let tree = parser.parse(&source, None).expect("Failed to parse");
    
    // Build compact tree
    let builder = CompactTreeBuilder::new();
    let compact_tree = builder.build(&tree, &source);
    
    // Compare roots
    let ts_root = tree.root_node();
    let compact_root = compact_tree.root();
    
    println!("Tree-sitter root:");
    println!("  Kind: {}", ts_root.kind());
    println!("  Children: {}", ts_root.child_count());
    println!("  Range: {}..{}", ts_root.start_byte(), ts_root.end_byte());
    
    println!("\nCompact root:");
    println!("  Kind: {}", compact_root.kind());
    println!("  Children: {}", compact_root.child_count());
    println!("  Range: {}..{}", compact_root.start_byte(), compact_root.end_byte());
    
    // Compare first few children
    println!("\nComparing children:");
    let mut ts_cursor = ts_root.walk();
    let ts_children: Vec<Node> = ts_root.children(&mut ts_cursor).collect();
    let compact_children: Vec<_> = compact_root.children().collect();
    
    for i in 0..ts_children.len().min(5) {
        println!("  Child {}:", i);
        println!("    TS: {} ({}..{})", 
                 ts_children[i].kind(),
                 ts_children[i].start_byte(),
                 ts_children[i].end_byte());
        if i < compact_children.len() {
            println!("    Compact: {} ({}..{})",
                     compact_children[i].kind(),
                     compact_children[i].start_byte(),
                     compact_children[i].end_byte());
        } else {
            println!("    Compact: MISSING");
        }
    }
    
    // Memory stats
    println!("\nMemory:");
    println!("  Nodes: {}", compact_tree.node_count());
    println!("  Bytes/node: {:.2}", compact_tree.bytes_per_node());
    let ts_estimate = ts_root.descendant_count() * 90;
    let compact_memory = compact_tree.memory_bytes();
    println!("  TS estimate: {} KB", ts_estimate / 1024);
    println!("  Compact: {} KB", compact_memory / 1024);
    println!("  Compression: {:.1}x", ts_estimate as f64 / compact_memory as f64);
}
