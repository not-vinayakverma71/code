//! Validate a single file from the test codebase

use lapce_tree_sitter::compact::{CompactTreeBuilder, CompactNode};
use tree_sitter::{Parser, Node};
use std::fs;

fn main() {
    let path = "/home/verma/lapce/lapce-ai/massive_test_codebase/module_9/submodule_0/module_901.py";
    let source = fs::read(path).expect("Failed to read file");
    
    println!("Testing file: {}", path);
    println!("Source length: {} bytes", source.len());
    
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
    
    println!("\nRoot comparison:");
    println!("  TS: {} with {} children", ts_root.kind(), ts_root.child_count());
    println!("  Compact: {} with {} children", compact_root.kind(), compact_root.child_count());
    
    if ts_root.child_count() != compact_root.child_count() {
        println!("  ❌ Child count mismatch!");
        
        // Debug why
        println!("\nDebug info:");
        println!("  BP position: {}", compact_root.bp_position());
        println!("  Node index: {}", compact_root.index());
        
        // Check BP operations directly
        let bp_ops = compact_tree.bp_operations();
        println!("  Direct BP child_count(0): {}", bp_ops.child_count(0));
        println!("  Direct BP first_child(0): {:?}", bp_ops.first_child(0));
    } else {
        println!("  ✅ Child counts match!");
        
        // Check all children
        let mut all_match = true;
        let mut ts_cursor = ts_root.walk();
        let ts_children: Vec<Node> = ts_root.children(&mut ts_cursor).collect();
        let compact_children: Vec<CompactNode> = compact_root.children().collect();
        
        for (i, (ts_child, compact_child)) in ts_children.iter().zip(compact_children.iter()).enumerate() {
            if ts_child.kind() != compact_child.kind() ||
               ts_child.start_byte() != compact_child.start_byte() ||
               ts_child.end_byte() != compact_child.end_byte() ||
               ts_child.child_count() != compact_child.child_count() {
                println!("  Child {} mismatch:", i);
                println!("    TS: {} ({}..{}) with {} children", 
                         ts_child.kind(), ts_child.start_byte(), ts_child.end_byte(), ts_child.child_count());
                println!("    Compact: {} ({}..{}) with {} children",
                         compact_child.kind(), compact_child.start_byte(), compact_child.end_byte(), compact_child.child_count());
                all_match = false;
            }
        }
        
        if all_match {
            println!("\n🎉 All {} children match perfectly!", ts_children.len());
        } else {
            println!("\n❌ Some children don't match");
        }
    }
    
    // Memory stats
    println!("\nMemory:");
    println!("  Nodes: {}", compact_tree.node_count());
    println!("  Bytes/node: {:.2}", compact_tree.bytes_per_node());
    println!("  Total: {} bytes", compact_tree.memory_bytes());
}
