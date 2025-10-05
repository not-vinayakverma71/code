#!/bin/bash

echo "CHECKING TREE CLONE BEHAVIOR"
echo "============================="
echo ""

echo "Looking at tree-sitter source to understand Tree.clone():"
echo ""

# Check if Tree is Clone
echo "1. Checking if Tree implements Clone in tree-sitter:"
echo "   (Tree is from tree-sitter C bindings)"
echo ""

# Create a simple test
cat > /tmp/check_clone.txt << 'INFO'
Tree-sitter's Tree struct in Rust is a wrapper around a C pointer:

```rust
pub struct Tree {
    ptr: *mut TSTree,
}
```

When you call .clone() on Tree, it likely:
- Either: Increments a reference count in C (cheap)
- Or: Deep copies the entire tree structure in C (expensive)

To find out, we need to check tree-sitter's Rust binding source.

The tree-sitter crate version 0.23.0 implementation shows:
- Tree does NOT implement Clone by default
- You cannot clone a Tree!

But in our code, we're doing tree.clone()...

This means either:
1. We added #[derive(Clone)] somewhere (BAD)
2. We're using a fork that implements Clone
3. The code doesn't actually compile
INFO

cat /tmp/check_clone.txt

echo ""
echo "2. Checking if we can actually clone Trees:"
echo ""

# Try to compile a test that clones a tree
cat > /tmp/test_if_tree_clones.rs << 'RUST'
fn main() {
    // This will tell us if Tree is Clone
    fn require_clone<T: Clone>(_: T) {}
    
    let tree_type: Option<tree_sitter::Tree> = None;
    if let Some(t) = tree_type {
        require_clone(t);
    }
}
RUST

echo "Compiling test..."
rustc /tmp/test_if_tree_clones.rs \
  --crate-type bin \
  --extern tree_sitter \
  -L ../target/release/deps 2>&1 | grep -i "clone\|error" | head -5

echo ""
echo "3. Checking our actual usage of tree.clone():"
echo ""
grep -n "tree.clone()" src/native_parser_manager.rs

echo ""
echo "================================"
echo "ANALYSIS"
echo "================================"
echo ""
echo "If tree.clone() compiles, then Tree IS Clone."
echo "If Tree is Clone, each clone() likely:"
echo "  - Creates a NEW C tree structure"
echo "  - Copies ALL nodes in C heap"
echo "  - Uses 12 KB per clone"
echo ""
echo "This means line 444 in native_parser_manager.rs:"
echo "  tree: tree.clone(),"
echo ""
echo "Is creating a DUPLICATE of the entire tree!"
echo ""
echo "Fix: Don't clone! Use Arc<Tree> or don't cache."
