//! Debug position storage and retrieval

use lapce_tree_sitter::compact::{CompactTreeBuilder};
use tree_sitter::Parser;

fn main() {
    // Simple Python
    let source = b"# comment\nimport os";
    
    // Parse with Tree-sitter
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    // Collect all node positions in preorder
    let mut positions = Vec::new();
    collect_positions(tree.root_node(), &mut positions);
    
    println!("Tree-sitter nodes in preorder:");
    for (i, (kind, start, end)) in positions.iter().enumerate() {
        println!("  Node {}: {} ({}..{})", i, kind, start, end);
    }
    
    // Build compact tree
    let builder = CompactTreeBuilder::new();
    let compact_tree = builder.build(&tree, source);
    
    // Test position access
    println!("\nCompact tree position access:");
    for i in 0..positions.len().min(10) {
        let start = compact_tree.start_byte(i);
        let end = compact_tree.end_byte(i);
        let expected_start = positions[i].1;
        let expected_end = positions[i].2;
        
        println!("  Node {}: {}..{} (expected {}..{})", 
                 i, start, end, expected_start, expected_end);
        
        if start != expected_start || end != expected_end {
            println!("    ❌ MISMATCH!");
        }
    }
    
    // Test via CompactNode
    println!("\nVia CompactNode:");
    let root = compact_tree.root();
    println!("  Root: {} at {}..{}", root.kind(), root.start_byte(), root.end_byte());
    
    for (i, child) in root.children().enumerate() {
        println!("  Child {}: {} at {}..{} (index {})", 
                 i, child.kind(), child.start_byte(), child.end_byte(), child.index());
    }
}

fn collect_positions(node: tree_sitter::Node, positions: &mut Vec<(String, usize, usize)>) {
    positions.push((node.kind().to_string(), node.start_byte(), node.end_byte()));
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_positions(child, positions);
    }
}
