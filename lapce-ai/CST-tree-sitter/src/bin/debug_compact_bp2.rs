//! Debug BP sequence with more detail

use lapce_tree_sitter::compact::{CompactTreeBuilder};
use tree_sitter::Parser;

fn main() {
    // Very simple code
    let source = b"x = 1";
    
    // Parse with Tree-sitter
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    // Build compact tree
    let builder = CompactTreeBuilder::new();
    let compact_tree = builder.build(&tree, source);
    
    let bp = compact_tree.bp_bitvec();
    let bp_ops = compact_tree.bp_operations();
    
    // Print BP sequence with positions
    println!("BP sequence analysis:");
    for i in 0..bp.len() {
        println!("  Position {}: {} ", i, if bp.get(i) { '(' } else { ')' });
    }
    
    // Test find_close
    println!("\nTesting find_close:");
    for i in 0..bp.len() {
        if bp.get(i) {
            println!("  find_close({}): {:?}", i, bp_ops.find_close(i));
        }
    }
    
    // Manually check what should be children of root
    println!("\nManual inspection of root's children:");
    let root_close = bp_ops.find_close(0).unwrap();
    println!("  Root opens at 0, closes at {}", root_close);
    
    let mut pos = 1;
    let mut depth = 0;
    let mut child_num = 0;
    
    while pos < root_close {
        if bp.get(pos) {
            if depth == 0 {
                child_num += 1;
                println!("  Child {} found at position {}", child_num, pos);
            }
            depth += 1;
        } else {
            depth -= 1;
        }
        pos += 1;
    }
}
