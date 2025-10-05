//! Direct BP method testing

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
    
    println!("Direct BP testing:");
    
    // Test child_count with manual implementation
    let root_pos = 0;
    let root_close = bp_ops.find_close(root_pos).unwrap();
    println!("Root at {}, closes at {}", root_pos, root_close);
    
    // Count children manually
    let mut pos = root_pos + 1;
    let mut depth = 0;
    let mut child_count = 0;
    
    println!("\nScanning for children:");
    while pos < root_close {
        println!("  pos={}, bit={}, depth={}", pos, if bp.get(pos) { '(' } else { ')' }, depth);
        if bp.get(pos) {
            if depth == 0 {
                child_count += 1;
                println!("    -> Found child #{} at position {}", child_count, pos);
            }
            depth += 1;
        } else {
            depth -= 1;
        }
        pos += 1;
    }
    
    println!("\nManual child count: {}", child_count);
    println!("BP ops child_count(0): {}", bp_ops.child_count(0));
    println!("BP ops first_child(0): {:?}", bp_ops.first_child(0));
    println!("BP ops kth_child(0, 1): {:?}", bp_ops.kth_child(0, 1));
}
