#!/bin/bash
echo "INVESTIGATING TREE-SITTER MEMORY MODEL"
echo "======================================="
echo ""

# Create a test to see how tree-sitter stores source
cat > /tmp/test_tree_source.rs << 'RUST'
use tree_sitter::{Parser, Tree};

fn main() {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    
    let source = "fn main() { println!(\"hello\"); }";
    let tree = parser.parse(source, None).unwrap();
    
    println!("Source size: {} bytes", source.len());
    println!("Tree size: {} bytes", std::mem::size_of::<Tree>());
    
    // Tree-sitter Tree is just a pointer to internal C structure
    // The source text is NOT stored in the Tree itself
    // Tree only has byte ranges that reference the source
    
    println!("\nTree-sitter does NOT store source in Tree!");
    println!("Tree only stores byte ranges (start, end)");
    println!("You need to keep source text separately!");
}
RUST

rustc /tmp/test_tree_source.rs -L ../target/release/deps --extern tree_sitter=../target/release/deps/libtree_sitter.rlib --extern tree_sitter_rust=../target/release/deps/libtree_sitter_rust.rlib 2>/dev/null

echo "Key insight:"
echo "============"
echo ""
echo "Tree-sitter's Tree struct does NOT contain the source text."
echo "It only contains:"
echo "  - Pointer to internal C tree structure"
echo "  - Byte ranges for each node"
echo "  - Parse state metadata"
echo ""
echo "The source text MUST be kept alive separately!"
echo ""
echo "Our StoredCST:"
echo "  tree: Tree          <- Pointers + metadata (~few KB)"
echo "  source: Vec<u8>     <- Full source text (avg 291 bytes)"
echo ""
echo "So we ARE duplicating source, but Tree doesn't have it either!"
echo "We NEED to store source for Tree to work."
echo ""
echo "Real question: Why is 12.4 KB per file so high?"
echo "  Source: ~291 bytes"
echo "  Tree metadata: ~few KB"
echo "  Total should be: ~2-3 KB, not 12.4 KB!"
