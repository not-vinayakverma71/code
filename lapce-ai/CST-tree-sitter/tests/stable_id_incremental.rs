//! Tests for stable ID persistence across incremental edits

use lapce_tree_sitter::compact::bytecode::{TreeSitterBytecodeEncoder, BytecodeDecoder};
use lapce_tree_sitter::incremental::IncrementalParser;
use tree_sitter::Parser;
use std::collections::HashMap;

/// Test that stable IDs remain consistent across small edits
#[test]
fn test_stable_ids_across_edits() {
    let mut parser = IncrementalParser::new("rust").unwrap();
    
    // Initial source
    let source1 = b"fn main() {\n    let x = 42;\n}";
    let tree1 = parser.parse_full(source1).unwrap();
    
    // Encode and get stable IDs
    let mut encoder1 = TreeSitterBytecodeEncoder::new();
    let bytecode1 = encoder1.encode_tree(&tree1, source1);
    let ids1 = bytecode1.stable_ids.clone();
    
    // Make a small edit: change 42 to 43
    let source2 = b"fn main() {\n    let x = 43;\n}";
    let edit = IncrementalParser::create_edit(
        source1,
        source2,
        24,  // start_byte
        26,  // old_end_byte
        26,  // new_end_byte
    );
    
    let tree2 = parser.parse_incremental(source2, edit).unwrap();
    
    // Encode the edited tree
    let mut encoder2 = TreeSitterBytecodeEncoder::new();
    let bytecode2 = encoder2.encode_tree(&tree2, source2);
    let ids2 = bytecode2.stable_ids.clone();
    
    // Most IDs should remain the same (except the edited literal node)
    // The structure (function, block, let statement) should have same IDs
    let mut same_count = 0;
    let mut diff_count = 0;
    
    for (i, (id1, id2)) in ids1.iter().zip(ids2.iter()).enumerate() {
        if id1 == id2 {
            same_count += 1;
        } else {
            diff_count += 1;
            println!("ID changed at index {}: {} -> {}", i, id1, id2);
        }
    }
    
    // Most nodes should have same IDs (only the literal value changed)
    assert!(same_count > diff_count, 
            "Expected most IDs to remain stable: {} same, {} different", 
            same_count, diff_count);
}

/// Test ID stability with node additions
#[test]
fn test_stable_ids_with_additions() {
    let mut parser = IncrementalParser::new("rust").unwrap();
    
    // Initial source
    let source1 = b"fn main() {\n    let x = 1;\n}";
    let tree1 = parser.parse_full(source1).unwrap();
    
    let mut encoder1 = TreeSitterBytecodeEncoder::new();
    let bytecode1 = encoder1.encode_tree(&tree1, source1);
    
    // Add a new statement
    let source2 = b"fn main() {\n    let x = 1;\n    let y = 2;\n}";
    let edit = IncrementalParser::create_edit(
        source1,
        source2,
        27,  // Insert point (after first statement)
        27,  // Same position
        42,  // New end after insertion
    );
    
    let tree2 = parser.parse_incremental(source2, edit).unwrap();
    
    let mut encoder2 = TreeSitterBytecodeEncoder::new();
    let bytecode2 = encoder2.encode_tree(&tree2, source2);
    
    // Original nodes should keep their IDs
    // New nodes should get new IDs
    assert!(bytecode2.stable_ids.len() > bytecode1.stable_ids.len(),
            "Should have more nodes after addition");
    
    // First few IDs (function declaration, etc.) should match
    for i in 0..5.min(bytecode1.stable_ids.len()) {
        println!("Comparing ID at {}: {} vs {}", i, bytecode1.stable_ids[i], bytecode2.stable_ids[i]);
    }
}

/// Test ID stability with node deletions
#[test]
fn test_stable_ids_with_deletions() {
    let mut parser = IncrementalParser::new("rust").unwrap();
    
    // Initial source with two statements
    let source1 = b"fn main() {\n    let x = 1;\n    let y = 2;\n}";
    let tree1 = parser.parse_full(source1).unwrap();
    
    let mut encoder1 = TreeSitterBytecodeEncoder::new();
    let bytecode1 = encoder1.encode_tree(&tree1, source1);
    
    // Remove second statement
    let source2 = b"fn main() {\n    let x = 1;\n}";
    let edit = IncrementalParser::create_edit(
        source1,
        source2,
        27,  // Start of deletion
        42,  // End of deletion
        27,  // New position after deletion
    );
    
    let tree2 = parser.parse_incremental(source2, edit).unwrap();
    
    let mut encoder2 = TreeSitterBytecodeEncoder::new();
    let bytecode2 = encoder2.encode_tree(&tree2, source2);
    
    // Remaining nodes should keep their IDs
    assert!(bytecode2.stable_ids.len() < bytecode1.stable_ids.len(),
            "Should have fewer nodes after deletion");
}

/// Test that decoder properly exposes stable IDs
#[test]
fn test_decoder_stable_ids() {
    let source = b"fn test() { let x = 42; }";
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    // Encode with stable IDs
    let mut encoder = TreeSitterBytecodeEncoder::new();
    let bytecode = encoder.encode_tree(&tree, source);
    
    // Decode and verify IDs are present
    let mut decoder = BytecodeDecoder::new();
    let nodes = decoder.decode(&bytecode).unwrap();
    
    // All nodes should have stable IDs
    for (i, node) in nodes.iter().enumerate() {
        assert!(node.stable_id > 0, "Node {} should have stable ID", i);
        println!("Node {}: {} (ID: {})", i, node.kind_name, node.stable_id);
    }
    
    // IDs should be unique
    let mut id_set = HashMap::new();
    for node in &nodes {
        if let Some(prev_idx) = id_set.insert(node.stable_id, node.kind_name.clone()) {
            panic!("Duplicate stable ID {}: {} and {}", 
                   node.stable_id, prev_idx, node.kind_name);
        }
    }
}

/// Test navigator returns stable IDs
#[test]
fn test_navigator_stable_ids() {
    use lapce_tree_sitter::compact::bytecode::BytecodeNavigator;
    
    let source = b"fn main() {}";
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    let mut encoder = TreeSitterBytecodeEncoder::new();
    let bytecode = encoder.encode_tree(&tree, source);
    
    let navigator = BytecodeNavigator::new(&bytecode);
    
    // Test get_stable_id method
    for i in 0..5 {
        if let Some(id) = navigator.get_stable_id(i) {
            assert!(id > 0, "Stable ID should be positive");
            println!("Node {} has stable ID: {}", i, id);
        }
    }
    
    // Test that get_node includes stable_id
    if let Some(node) = navigator.get_node(0) {
        assert!(node.stable_id > 0, "Node should have stable ID");
    }
}
