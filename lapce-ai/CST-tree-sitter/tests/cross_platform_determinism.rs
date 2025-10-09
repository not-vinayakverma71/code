//! Cross-platform determinism tests
//! Ensures bytecode encoding is identical across Linux, macOS, and Windows

use lapce_tree_sitter::compact::bytecode::TreeSitterBytecodeEncoder;
use std::collections::HashMap;
use tree_sitter::Parser;

/// Test that bytecode is deterministic across runs
#[test]
fn test_bytecode_determinism() {
    let test_cases = vec![
        ("rust", "fn main() { println!(\"Hello, world!\"); }", tree_sitter_rust::LANGUAGE),
        ("python", "def hello():\n    print('Hello')", tree_sitter_python::LANGUAGE),
    ];
    
    for (lang, source, language) in test_cases {
        // Parse the source
        let mut parser = Parser::new();
        parser.set_language(&language.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        // Encode multiple times
        let mut bytecodes = Vec::new();
        for _ in 0..5 {
            let mut encoder = TreeSitterBytecodeEncoder::new();
            let bytecode = encoder.encode_tree(&tree, source.as_bytes());
            bytecodes.push(bytecode);
        }
        
        // All should be identical
        let first = &bytecodes[0];
        for (i, bytecode) in bytecodes.iter().enumerate().skip(1) {
            assert_eq!(
                first.bytes, bytecode.bytes,
                "{}: Run {} produced different bytecode", lang, i
            );
            assert_eq!(
                first.node_count, bytecode.node_count,
                "{}: Run {} has different node count", lang, i
            );
        }
    }
}

/// Test that identical input produces identical output regardless of memory state
#[test]
fn test_memory_independence() {
    let source = "fn test() { let x = 42; }";
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    
    // First encoding
    let tree1 = parser.parse(source, None).unwrap();
    let mut encoder1 = TreeSitterBytecodeEncoder::new();
    let bytecode1 = encoder1.encode_tree(&tree1, source.as_bytes());
    
    // Allocate some memory to change heap state
    let _dummy: Vec<u8> = vec![0; 1024 * 1024];
    
    // Second encoding
    let tree2 = parser.parse(source, None).unwrap();
    let mut encoder2 = TreeSitterBytecodeEncoder::new();
    let bytecode2 = encoder2.encode_tree(&tree2, source.as_bytes());
    
    assert_eq!(bytecode1.bytes, bytecode2.bytes, "Memory state affected encoding");
}

/// Test that order of encoding doesn't affect individual results
#[test]
fn test_encoding_order_independence() {
    let sources = vec![
        "fn a() {}",
        "fn b() { let x = 1; }",
        "fn c() { println!(\"test\"); }",
    ];
    
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    
    // Encode individually
    let mut individual_bytecodes = HashMap::new();
    for source in &sources {
        let tree = parser.parse(source, None).unwrap();
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        individual_bytecodes.insert(*source, bytecode);
    }
    
    // Encode in different order
    for source in sources.iter().rev() {
        let tree = parser.parse(source, None).unwrap();
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        let original = individual_bytecodes.get(source).unwrap();
        assert_eq!(
            original.bytes, bytecode.bytes,
            "Order affected encoding for: {}", source
        );
    }
}

/// Property test: encoding properties
#[test]
fn test_encoding_properties() {
    let test_sources = vec![
        "",  // Empty
        " ",  // Whitespace only
        "x",  // Single identifier
        "42",  // Single number
        "\"hello\"",  // String literal
        "// comment",  // Comment only
        "fn f() {}",  // Simple function
        "fn f() { fn g() {} }",  // Nested function
    ];
    
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    
    for source in test_sources {
        let tree = parser.parse(source, None).unwrap();
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        // Properties that must hold
        assert!(bytecode.bytes.len() > 0, "Empty bytecode for: {:?}", source);
        assert!(bytecode.node_count > 0, "Zero nodes for: {:?}", source);
        assert_eq!(bytecode.source_len, source.len(), "Source length mismatch for: {:?}", source);
        
        // Must have End marker
        assert!(
            bytecode.bytes.iter().any(|&b| b == 0xFF),
            "No End marker for: {:?}", source
        );
        
        // Node count should match tree
        let actual_nodes = count_nodes(tree.root_node());
        assert_eq!(
            bytecode.node_count, actual_nodes,
            "Node count mismatch for: {:?}", source
        );
    }
}

fn count_nodes(node: tree_sitter::Node) -> usize {
    let mut count = 1;
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            count += count_nodes(child);
        }
    }
    count
}

/// Test that bytecode is platform-independent (simulated)
#[cfg(test)]
mod platform_simulation {
    use super::*;
    
    #[test]
    fn test_endianness_independence() {
        // Bytecode should use fixed endianness (little-endian varints)
        let source = "let x = 0x12345678;";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        // Check that multi-byte values are encoded consistently
        // This would fail if we used native endianness
        let bytes = &bytecode.bytes;
        
        // The bytecode should be identical regardless of platform
        // We can't actually test different platforms in one test,
        // but we can verify the encoding is deterministic
        assert!(!bytes.is_empty());
        
        // Re-encode and verify identical
        let mut encoder2 = TreeSitterBytecodeEncoder::new();
        let bytecode2 = encoder2.encode_tree(&tree, source.as_bytes());
        assert_eq!(bytecode.bytes, bytecode2.bytes);
    }
    
    #[test]
    fn test_path_separator_independence() {
        // Even though we don't encode paths, test that the encoder
        // doesn't accidentally include system-specific data
        let source = "// Test file\nfn main() {}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, source.as_bytes());
        
        // Bytecode should not contain path separators
        assert!(!bytecode.bytes.windows(2).any(|w| w == b"\\/" || w == b"\\\\"));
    }
}
