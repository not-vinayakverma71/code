# Upstream Validation Tests Specification (CST-UP04)

**Target:** CST-tree-sitter bytecode encoding/decoding  
**Purpose:** Validate encode/decode correctness with property-based and fuzz testing  
**Priority:** Medium

## Overview

This document specifies comprehensive validation tests for CST-tree-sitter's bytecode encoding and decoding to ensure:
1. Round-trip correctness (encode → decode → original)
2. Resilience to malformed inputs
3. Deterministic encoding behavior
4. Performance under adversarial inputs

## Property-Based Tests

### Required Dependencies

```toml
[dev-dependencies]
proptest = "1.4"
quickcheck = "1.0"
```

### Test Suite Structure

```rust
// tests/property_validation.rs

use proptest::prelude::*;
use lapce_tree_sitter::{TreeSitterBytecodeEncoder, BytecodeDecoder};

/// Property 1: Round-trip correctness
/// For any valid tree T, decode(encode(T)) == T
#[test]
fn prop_encode_decode_roundtrip() {
    proptest!(|(tree in arb_tree())| {
        let encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, tree.source());
        
        let decoder = BytecodeDecoder::new();
        let decoded_tree = decoder.decode(&bytecode).unwrap();
        
        // Verify structural equivalence
        prop_assert_eq!(decoded_tree.root_node().kind(), tree.root_node().kind());
        prop_assert_eq!(decoded_tree.root_node().child_count(), tree.root_node().child_count());
        
        // Verify deep equality
        prop_assert!(trees_equal(&tree, &decoded_tree));
    });
}

/// Property 2: Idempotence
/// encode(decode(encode(T))) == encode(T)
#[test]
fn prop_encode_idempotent() {
    proptest!(|(tree in arb_tree())| {
        let encoder = TreeSitterBytecodeEncoder::new();
        let bytecode1 = encoder.encode_tree(&tree, tree.source());
        
        let decoder = BytecodeDecoder::new();
        let decoded = decoder.decode(&bytecode1).unwrap();
        
        let bytecode2 = encoder.encode_tree(&decoded, decoded.source());
        
        // Bytecode should be identical
        prop_assert_eq!(bytecode1.bytes, bytecode2.bytes);
    });
}

/// Property 3: Size bounds
/// encoded_size(T) <= original_size(T) * compression_ratio
#[test]
fn prop_size_bounds() {
    proptest!(|(tree in arb_tree())| {
        let encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, tree.source());
        
        let original_size = tree.source().len();
        let encoded_size = bytecode.bytes.len();
        
        // Bytecode should not be pathologically large
        prop_assert!(encoded_size < original_size * 10);
    });
}

/// Property 4: Deterministic encoding
/// encode(T) at t1 == encode(T) at t2
#[test]
fn prop_deterministic_encoding() {
    proptest!(|(tree in arb_tree())| {
        let encoder1 = TreeSitterBytecodeEncoder::new();
        let encoder2 = TreeSitterBytecodeEncoder::new();
        
        let bytecode1 = encoder1.encode_tree(&tree, tree.source());
        let bytecode2 = encoder2.encode_tree(&tree, tree.source());
        
        prop_assert_eq!(bytecode1.bytes, bytecode2.bytes);
    });
}

/// Property 5: Stable ID preservation
/// For all nodes n in T, stable_id(decode(encode(T)).node(n)) == stable_id(T.node(n))
#[test]
fn prop_stable_id_preservation() {
    proptest!(|(tree in arb_tree_with_ids())| {
        let encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree_with_ids(&tree, tree.source());
        
        let decoder = BytecodeDecoder::new();
        let decoded = decoder.decode_with_ids(&bytecode).unwrap();
        
        // Check all stable IDs are preserved
        for (original_id, decoded_id) in tree.stable_ids().zip(decoded.stable_ids()) {
            prop_assert_eq!(original_id, decoded_id);
        }
    });
}

/// Property 6: Metadata preservation
/// For all nodes n, metadata(decode(encode(T)).node(n)) == metadata(T.node(n))
#[test]
fn prop_metadata_preservation() {
    proptest!(|(tree in arb_tree())| {
        let encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, tree.source());
        
        let decoder = BytecodeDecoder::new();
        let decoded = decoder.decode(&bytecode).unwrap();
        
        // Check all node metadata
        let original_nodes: Vec<_> = tree.walk().collect();
        let decoded_nodes: Vec<_> = decoded.walk().collect();
        
        prop_assert_eq!(original_nodes.len(), decoded_nodes.len());
        
        for (orig, dec) in original_nodes.iter().zip(decoded_nodes.iter()) {
            prop_assert_eq!(orig.kind(), dec.kind());
            prop_assert_eq!(orig.start_byte(), dec.start_byte());
            prop_assert_eq!(orig.end_byte(), dec.end_byte());
            prop_assert_eq!(orig.is_named(), dec.is_named());
            prop_assert_eq!(orig.is_missing(), dec.is_missing());
        }
    });
}
```

### Arbitrary Tree Generators

```rust
use proptest::prelude::*;
use tree_sitter::{Tree, Parser, Language};

/// Generate arbitrary valid syntax trees
fn arb_tree() -> impl Strategy<Value = Tree> {
    prop_oneof![
        arb_rust_tree(),
        arb_javascript_tree(),
        arb_python_tree(),
    ]
}

fn arb_rust_tree() -> impl Strategy<Value = Tree> {
    prop::string::string_regex("[a-z_][a-z0-9_]*")
        .unwrap()
        .prop_flat_map(|name| {
            (Just(name.clone()), prop::collection::vec(0..10u32, 0..5))
                .prop_map(move |(n, params)| {
                    let source = format!(
                        "fn {}({}) {{ {} }}",
                        n,
                        params.iter().map(|p| format!("p{}: i32", p)).collect::<Vec<_>>().join(", "),
                        "let x = 42;"
                    );
                    
                    let mut parser = Parser::new();
                    parser.set_language(tree_sitter_rust::language()).unwrap();
                    parser.parse(&source, None).unwrap()
                })
        })
}

fn arb_javascript_tree() -> impl Strategy<Value = Tree> {
    prop::string::string_regex("[a-z_][a-z0-9_]*")
        .unwrap()
        .prop_map(|name| {
            let source = format!("function {}() {{ return 42; }}", name);
            
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_javascript::language()).unwrap();
            parser.parse(&source, None).unwrap()
        })
}

fn arb_tree_with_ids() -> impl Strategy<Value = TreeWithIds> {
    arb_tree().prop_map(|tree| {
        // Assign stable IDs to all nodes
        TreeWithIds::from_tree(tree)
    })
}
```

## Fuzz Tests

### cargo-fuzz Integration

```toml
# Cargo.toml
[dev-dependencies]
arbitrary = { version = "1.3", features = ["derive"] }

[workspace]
members = [".", "fuzz"]
```

### Fuzz Target 1: Decode Malformed Bytecode

```rust
// fuzz/fuzz_targets/decode_malformed.rs

#![no_main]
use libfuzzer_sys::fuzz_target;
use lapce_tree_sitter::BytecodeDecoder;

fuzz_target!(|data: &[u8]| {
    let decoder = BytecodeDecoder::new();
    
    // Should never panic, even on malformed input
    let _ = decoder.decode(data);
    
    // If decode succeeds, verify basic invariants
    if let Ok(tree) = decoder.decode(data) {
        // Root node should exist
        assert!(tree.root_node().is_some());
        
        // Node count should be finite
        let node_count = tree.walk().count();
        assert!(node_count < 1_000_000); // Reasonable upper bound
        
        // All nodes should have valid byte ranges
        for node in tree.walk() {
            assert!(node.start_byte() <= node.end_byte());
            assert!(node.end_byte() <= tree.source().len());
        }
    }
});
```

### Fuzz Target 2: Encode-Decode Round Trip

```rust
// fuzz/fuzz_targets/roundtrip.rs

#![no_main]
use libfuzzer_sys::fuzz_target;
use lapce_tree_sitter::{TreeSitterBytecodeEncoder, BytecodeDecoder};
use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
struct SyntheticSource {
    language: Language,
    source: String,
}

#[derive(Arbitrary, Debug, Clone, Copy)]
enum Language {
    Rust,
    JavaScript,
    Python,
}

fuzz_target!(|input: SyntheticSource| {
    let mut parser = Parser::new();
    
    let lang = match input.language {
        Language::Rust => tree_sitter_rust::language(),
        Language::JavaScript => tree_sitter_javascript::language(),
        Language::Python => tree_sitter_python::language(),
    };
    
    parser.set_language(lang).unwrap();
    
    // Parse the synthetic source
    if let Some(tree) = parser.parse(&input.source, None) {
        let encoder = TreeSitterBytecodeEncoder::new();
        let bytecode = encoder.encode_tree(&tree, input.source.as_bytes());
        
        let decoder = BytecodeDecoder::new();
        if let Ok(decoded) = decoder.decode(&bytecode.bytes) {
            // Verify structural equality
            assert_eq!(decoded.root_node().kind(), tree.root_node().kind());
            assert_eq!(decoded.root_node().child_count(), tree.root_node().child_count());
        }
    }
});
```

### Fuzz Target 3: Segmented Bytecode

```rust
// fuzz/fuzz_targets/segmented_decode.rs

#![no_main]
use libfuzzer_sys::fuzz_target;
use lapce_tree_sitter::{SegmentedBytecodeStream, BytecodeNavigator};

fuzz_target!(|data: &[u8]| {
    // Try to create segmented stream from arbitrary data
    if let Ok(stream) = SegmentedBytecodeStream::from_bytes(data) {
        let navigator = BytecodeNavigator::new(&stream);
        
        // Navigate should not panic
        let _ = navigator.navigate_to_root();
        let _ = navigator.child_count();
        
        // Segment boundaries should be valid
        for segment in stream.segments() {
            assert!(segment.start_offset <= segment.end_offset);
            assert!(segment.end_offset <= data.len());
        }
    }
});
```

### Fuzz Target 4: Stable ID Assignment

```rust
// fuzz/fuzz_targets/stable_id_consistency.rs

#![no_main]
use libfuzzer_sys::fuzz_target;
use lapce_tree_sitter::CstApi;

fuzz_target!(|source: &str| {
    // Parse the same source twice
    let api1 = CstApi::parse_rust(source);
    let api2 = CstApi::parse_rust(source);
    
    if let (Some(api1), Some(api2)) = (api1, api2) {
        // Stable IDs should be identical for same source
        let ids1: Vec<_> = api1.stable_ids().collect();
        let ids2: Vec<_> = api2.stable_ids().collect();
        
        assert_eq!(ids1, ids2, "Stable IDs not deterministic");
        
        // No duplicate IDs
        let mut seen = std::collections::HashSet::new();
        for id in ids1 {
            assert!(seen.insert(id), "Duplicate stable ID: {}", id);
        }
    }
});
```

## Test Execution

### Running Property Tests

```bash
# Run all property tests
cargo test --test property_validation

# Run with more iterations (default: 256)
PROPTEST_CASES=10000 cargo test --test property_validation

# Run with specific seed for reproducibility
PROPTEST_MAX_SHRINK_ITERS=100000 cargo test --test property_validation
```

### Running Fuzz Tests

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Run decode_malformed fuzz target
cargo fuzz run decode_malformed

# Run with custom timeout and iterations
cargo fuzz run decode_malformed -- -max_total_time=3600 -max_len=65536

# Run all fuzz targets
for target in fuzz/fuzz_targets/*.rs; do
    cargo fuzz run $(basename $target .rs) -- -max_total_time=600
done
```

### Continuous Fuzzing

```bash
# Run overnight fuzzing
cargo fuzz run decode_malformed -- -max_total_time=28800 > fuzz.log 2>&1 &

# Check corpus size
ls -lh fuzz/corpus/decode_malformed/

# Minimize corpus
cargo fuzz cmin decode_malformed
```

## Crash Triage

### Reproduce Crashes

```bash
# Reproduce a specific crash
cargo fuzz run decode_malformed fuzz/artifacts/decode_malformed/crash-abc123

# Debug with gdb
rust-gdb --args target/x86_64-unknown-linux-gnu/release/decode_malformed fuzz/artifacts/decode_malformed/crash-abc123
```

### Crash Analysis Checklist

1. ☐ Extract minimal reproducer
2. ☐ Verify crash is not a fuzzer artifact
3. ☐ Check if crash is security-relevant
4. ☐ File upstream issue with reproducer
5. ☐ Add regression test
6. ☐ Implement fix
7. ☐ Verify fix with fuzzer

## Integration with CI

### GitHub Actions Workflow

```yaml
# .github/workflows/fuzz.yml
name: Fuzz Testing

on:
  schedule:
    - cron: '0 2 * * *'  # Run daily at 2 AM
  workflow_dispatch:

jobs:
  fuzz:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - decode_malformed
          - roundtrip
          - segmented_decode
          - stable_id_consistency
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust nightly
        uses: dtolnay/rust-toolchain@nightly
      
      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz
      
      - name: Run fuzzer
        run: |
          cargo fuzz run ${{ matrix.target }} -- \
            -max_total_time=1800 \
            -max_len=65536 \
            -rss_limit_mb=2048
      
      - name: Upload artifacts
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: fuzz-artifacts-${{ matrix.target }}
          path: fuzz/artifacts/${{ matrix.target }}/
```

## Success Metrics

| Metric | Target | Critical |
|--------|--------|----------|
| Property test pass rate | 100% | 99% |
| Fuzz crashes per 1M iterations | 0 | <5 |
| Code coverage (property tests) | >90% | >80% |
| Corpus size growth | Steady | N/A |
| Shrinking success rate | >80% | >60% |

## Known Issues to Test

Based on common bytecode encoding pitfalls:

1. **Off-by-one errors** in byte offsets
2. **Integer overflows** in size calculations
3. **Stack overflows** on deeply nested trees
4. **Out-of-bounds reads** in segment access
5. **Use-after-free** in cached pointers
6. **Compression artifacts** losing data
7. **Endianness issues** in multi-byte values
8. **Alignment faults** on unaligned access

## Timeline

- **Property test implementation:** 2 days
- **Fuzz target creation:** 2 days
- **CI integration:** 1 day
- **Initial corpus generation:** 3 days (continuous)
- **Crash triage & fixes:** Ongoing
- **Total:** ~1 week initial setup + continuous

## Dependencies

- **CST-tree-sitter:** Bytecode encoder/decoder exposure
- **proptest:** v1.4+
- **cargo-fuzz:** Latest
- **libfuzzer-sys:** v0.4+

---

*Last updated: 2025-10-11*  
*Status: Specification complete, awaiting upstream bytecode API*
