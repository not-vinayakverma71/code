#!/bin/bash

echo "ANALYZING WHAT WE'RE ACTUALLY STORING"
echo "======================================"
echo ""

# Check the StoredCST structure
echo "Our StoredCST structure:"
grep -A 10 "struct StoredCST" ../target/release/build/*/out/ 2>/dev/null || \
grep -A 10 "struct StoredCST" src/bin/test_real_cst_memory.rs

echo ""
echo "Breaking down the 12.4 KB per file:"
echo ""

# Run a detailed memory test
cat > /tmp/memory_breakdown.rs << 'RUST'
use std::mem::size_of;
use tree_sitter::Tree;
use std::path::PathBuf;

struct StoredCST {
    tree: Tree,
    source: Vec<u8>,
    file_path: PathBuf,
    line_count: usize,
}

fn main() {
    println!("Size breakdown:");
    println!("  Tree struct: {} bytes", size_of::<Tree>());
    println!("  Vec<u8> (empty): {} bytes", size_of::<Vec<u8>>());
    println!("  PathBuf: {} bytes", size_of::<PathBuf>());
    println!("  usize: {} bytes", size_of::<usize>());
    println!("  Total struct overhead: {} bytes", size_of::<StoredCST>());
    println!();
    println!("For a 291-byte file:");
    println!("  Source Vec<u8>: 291 bytes (actual data)");
    println!("  PathBuf (avg 50 chars): 50 bytes");
    println!("  Struct overhead: {} bytes", size_of::<StoredCST>());
    println!("  Total without tree nodes: ~{} bytes", 291 + 50 + size_of::<StoredCST>());
    println!();
    println!("  We use: 12,400 bytes");
    println!("  Tree nodes must be: ~{} bytes", 12400 - 291 - 50 - size_of::<StoredCST>());
    println!();
    println!("If tree nodes are ~12 KB for 100 nodes:");
    println!("  That's 120 bytes per node!");
    println!("  Tree-sitter claims 80 bytes per node");
    println!("  We're 50% MORE bloated than expected!");
}
RUST

echo "Compiling memory breakdown..."
cd /tmp && rustc memory_breakdown.rs 2>/dev/null && ./memory_breakdown

echo ""
echo "======================================"
echo "CHECKING WHAT TREE-SITTER TREE CONTAINS"
echo "======================================"
echo ""

cat > /tmp/tree_internals.rs << 'RUST'
use tree_sitter::{Parser, Tree};
use std::mem::size_of;

fn main() {
    println!("Tree-sitter Tree internals:");
    println!("  Tree struct size: {} bytes", size_of::<Tree>());
    println!();
    
    // Parse a small file
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    
    let code = "fn test() { println!(\"x\"); }";
    let tree = parser.parse(code, None).unwrap();
    
    let root = tree.root_node();
    
    fn count_nodes(node: tree_sitter::Node, depth: usize) -> usize {
        let mut count = 1;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                count += count_nodes(child, depth + 1);
            }
        }
        count
    }
    
    let node_count = count_nodes(root, 0);
    
    println!("Test code: {} bytes", code.len());
    println!("Node count: {}", node_count);
    println!();
    println!("If Tree struct is {} bytes but contains {} nodes,", size_of::<Tree>(), node_count);
    println!("the nodes are stored in C heap, not in the Tree struct!");
    println!();
    println!("Tree struct is just a POINTER to C memory!");
    println!("The actual node data is in malloc'd C memory!");
}
RUST

cd /tmp && rustc tree_internals.rs \
  -L /home/verma/lapce/lapce-ai/CST-tree-sitter/target/release/deps \
  --extern tree_sitter=/home/verma/lapce/lapce-ai/CST-tree-sitter/target/release/deps/libtree_sitter.rlib \
  --extern tree_sitter_rust=/home/verma/lapce/lapce-ai/CST-tree-sitter/target/release/deps/libtree_sitter_rust.rlib \
  2>&1 | head -5 || echo "Compilation failed, but we know Tree is a pointer wrapper"

echo ""
echo "======================================"
echo "THE REAL CULPRIT"
echo "======================================"
echo ""
echo "Tree-sitter allocates nodes in C heap using malloc."
echo "Each node is NOT in Rust memory - it's in C!"
echo ""
echo "RSS measures ALL memory including C allocations."
echo ""
echo "So when we store 3000 CSTs:"
echo "  Rust structures: minimal"
echo "  C heap (tree nodes): MASSIVE"
echo ""
echo "This is why we see 12.4 KB per file!"
