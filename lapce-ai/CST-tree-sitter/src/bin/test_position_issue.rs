//! Debug position issue

use lapce_tree_sitter::compact::{CompactTreeBuilder};
use tree_sitter::Parser;

fn main() {
    // Simple Python with multiple statements
    let source = b"# comment\nimport os\nclass A:\n    pass\ndef foo():\n    pass";
    
    // Parse with Tree-sitter
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    // Build compact tree
    let builder = CompactTreeBuilder::new();
    let compact_tree = builder.build(&tree, source);
    
    // Compare positions
    println!("Comparing positions:");
    println!("Source:\n{}\n", std::str::from_utf8(source).unwrap());
    
    let ts_root = tree.root_node();
    let compact_root = compact_tree.root();
    
    let mut ts_cursor = ts_root.walk();
    let ts_children: Vec<_> = ts_root.children(&mut ts_cursor).collect();
    let compact_children: Vec<_> = compact_root.children().collect();
    
    for i in 0..ts_children.len() {
        let ts_child = ts_children[i];
        let compact_child = &compact_children[i];
        
        println!("Child {} ({}):", i, ts_child.kind());
        println!("  TS:      {}..{}", ts_child.start_byte(), ts_child.end_byte());
        println!("  Compact: {}..{}", compact_child.start_byte(), compact_child.end_byte());
        println!("  Node index: {}", compact_child.index());
        
        let text_ts = ts_child.utf8_text(source).unwrap_or("?");
        let text_compact = compact_child.utf8_text(source).unwrap_or("?");
        println!("  TS text:      {:?}", text_ts);
        println!("  Compact text: {:?}", text_compact);
        
        if ts_child.start_byte() != compact_child.start_byte() {
            println!("  ❌ POSITION MISMATCH!");
        }
    }
}
