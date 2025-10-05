//! Simple validation test

use lapce_tree_sitter::compact::{CompactTreeBuilder, CompactNode};
use tree_sitter::{Parser, Node};

fn main() {
    // Simple Python
    let source = b"# comment\nimport os\nclass A:\n    pass";
    
    // Parse with Tree-sitter
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    // Build compact tree
    let builder = CompactTreeBuilder::new();
    let compact_tree = builder.build(&tree, source);
    
    // Compare
    let ts_root = tree.root_node();
    let compact_root = compact_tree.root();
    
    println!("Comparing trees:");
    compare_nodes(ts_root, compact_root, 0);
}

fn compare_nodes(ts_node: Node, compact_node: CompactNode, depth: usize) {
    let indent = "  ".repeat(depth);
    
    println!("{}Node: {}", indent, ts_node.kind());
    println!("{}  TS: children={}, range={}..{}", 
             indent, ts_node.child_count(), ts_node.start_byte(), ts_node.end_byte());
    println!("{}  Compact: children={}, range={}..{}", 
             indent, compact_node.child_count(), compact_node.start_byte(), compact_node.end_byte());
    
    if ts_node.child_count() != compact_node.child_count() {
        println!("{}  ❌ Child count mismatch!", indent);
        return;
    }
    
    if ts_node.start_byte() != compact_node.start_byte() || 
       ts_node.end_byte() != compact_node.end_byte() {
        println!("{}  ❌ Position mismatch!", indent);
        return;
    }
    
    // Compare children
    let mut ts_cursor = ts_node.walk();
    let ts_children: Vec<Node> = ts_node.children(&mut ts_cursor).collect();
    let compact_children: Vec<CompactNode> = compact_node.children().collect();
    
    if ts_children.len() != compact_children.len() {
        println!("{}  ❌ Different number of children collected!", indent);
        return;
    }
    
    for (ts_child, compact_child) in ts_children.iter().zip(compact_children.iter()) {
        compare_nodes(*ts_child, *compact_child, depth + 1);
    }
}
