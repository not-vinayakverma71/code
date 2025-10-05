#!/bin/bash

echo "ANALYZING TREE CLONING BEHAVIOR"
echo "================================"
echo ""

cat > /tmp/test_tree_clone.rs << 'RUST'
use tree_sitter::{Parser, Tree};

fn get_rss_kb() -> u64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse().unwrap_or(0);
                }
            }
        }
    }
    0
}

fn main() {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    
    let code = "fn test() { println!(\"hello\"); }".repeat(100);
    
    println!("Baseline: {} KB", get_rss_kb());
    
    // Parse once
    let tree1 = parser.parse(&code, None).unwrap();
    let after_one = get_rss_kb();
    println!("After 1 tree: {} KB", after_one);
    
    // Clone the tree
    let tree2 = tree1.clone();
    let after_clone = get_rss_kb();
    println!("After clone: {} KB (delta: {})", after_clone, after_clone - after_one);
    
    // Clone again
    let tree3 = tree1.clone();
    let after_clone2 = get_rss_kb();
    println!("After 2nd clone: {} KB (delta: {})", after_clone2, after_clone2 - after_clone);
    
    // Make 100 clones
    let mut clones = Vec::new();
    for _ in 0..100 {
        clones.push(tree1.clone());
    }
    let after_100 = get_rss_kb();
    println!("After 100 clones: {} KB (delta: {})", after_100, after_100 - after_clone2);
    
    println!();
    if after_100 - after_one > 10000 {
        println!("❌ CLONING DUPLICATES MEMORY!");
        println!("   Each clone uses ~{} KB", (after_100 - after_one) / 100);
    } else {
        println!("✅ Cloning is cheap (reference counted)");
    }
}
RUST

echo "Compiling and running test..."
rustc /tmp/test_tree_clone.rs \
  -L ../target/release/deps \
  --extern tree_sitter=../target/release/deps/libtree_sitter.rlib \
  --extern tree_sitter_rust=../target/release/deps/libtree_sitter_rust.rlib \
  -o /tmp/test_tree_clone 2>&1 | head -10

if [ -f /tmp/test_tree_clone ]; then
    /tmp/test_tree_clone
else
    echo "Compilation failed"
fi

echo ""
echo "================================"
echo "CHECKING CODEBASE FOR TREE CLONES"
echo "================================"
echo ""

echo "Places where Tree is cloned:"
grep -rn "\.clone()" src/ | grep -i "tree" | head -20

echo ""
echo "CachedTree with #[derive(Clone)]:"
grep -rn "#\[derive(Clone)\]" src/ -A 3 | grep -B 1 "CachedTree"
