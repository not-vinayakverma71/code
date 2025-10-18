# CST Integration Guide

## Overview

The semantic search system integrates with **lapce-tree-sitter** (CST-tree-sitter) to provide canonical AST construction across multiple programming languages. This integration enables consistent semantic analysis regardless of the source language.

## Architecture

### Phase A: Canonical Kind Mapping (Current)

**Status:** ✅ Complete

Phase A provides language-agnostic node type mapping using canonical kinds from `lapce-tree-sitter`.

```
Source Code (Rust/JS/Python/etc.)
    ↓
Tree-sitter Parser (language-specific)
    ↓
CST Node (concrete syntax tree)
    ↓
Canonical Kind Mapper (lapce-tree-sitter)
    ↓
AstNode with normalized types
```

**Benefits:**
- Consistent AST structure across languages
- Simplified cross-language semantic analysis
- Robust identifier and value extraction

### Phase B: Stable IDs & Incremental Indexing (Future)

Phase B will add:
- Stable node IDs for tracking nodes across file edits
- Incremental re-embedding of only changed code
- CstApi-based segmented storage
- Sub-linear indexing for large codebases

## Feature Flags

### `cst_ts` Feature

Enable canonical mapping support:

```toml
[dependencies]
semantic_search = { path = "...", features = ["cst_ts"] }
```

**What changes with `cst_ts` enabled:**
- Uses `CanonicalKind` → `AstNodeType` mapping
- Robust field-based identifier extraction
- Prometheus metrics for mapping quality
- All tests pass in both modes

**Default behavior (without feature):**
- Falls back to language-specific string matching
- Still functional but less robust
- No canonical mapping metrics

## Usage

### Basic Usage

```rust
use semantic_search::processors::cst_to_ast_pipeline::CstToAstPipeline;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = CstToAstPipeline::new();
    
    // Process a Rust file
    let result = pipeline.process_file(Path::new("src/main.rs")).await?;
    
    println!("Language: {}", result.language);
    println!("Parse time: {:.2}ms", result.parse_time_ms);
    println!("Transform time: {:.2}ms", result.transform_time_ms);
    println!("Root node type: {:?}", result.ast.node_type);
    
    Ok(())
}
```

### Querying AST

```rust
// Find all function declarations
let functions = pipeline.query_both(
    Path::new("src/lib.rs"),
    "FunctionDeclaration"
)?;

for func in functions.ast_matches {
    if let Some(name) = &func.identifier {
        println!("Function: {}", name);
        println!("  Lines: {}-{}", 
            func.metadata.start_line, 
            func.metadata.end_line);
        println!("  Complexity: {}", func.metadata.complexity);
    }
}
```

### Cross-Language Consistency

With `cst_ts` enabled, the same constructs map to identical AST types:

```rust
// Rust: fn add(x: i32, y: i32) -> i32 { x + y }
// JavaScript: function add(x, y) { return x + y; }
// Python: def add(x, y): return x + y
// Go: func add(x int, y int) int { return x + y }

// All produce:
// - node_type: AstNodeType::FunctionDeclaration
// - identifier: Some("add")
// - Consistent structure for parameters and body
```

## Supported Languages

| Language   | Extension | Canonical Mapping | Status |
|-----------|-----------|-------------------|--------|
| Rust      | `.rs`     | ✅ Full           | Stable |
| JavaScript| `.js`     | ✅ Full           | Stable |
| TypeScript| `.ts`     | ✅ Full           | Stable |
| Python    | `.py`     | ✅ Full           | Stable |
| Go        | `.go`     | ✅ Full           | Stable |
| Java      | `.java`   | ✅ Full           | Stable |

## Canonical Node Types

### Declarations
- `FunctionDeclaration` - Functions, methods, procedures
- `ClassDeclaration` - Classes, types
- `InterfaceDeclaration` - Interfaces, protocols, traits
- `StructDeclaration` - Structs, records
- `EnumDeclaration` - Enums, unions
- `VariableDeclaration` - Variables, constants
- `TypeAlias` - Type aliases, typedefs

### Control Flow
- `IfStatement` - Conditional branches
- `ForLoop` - For/foreach loops
- `WhileLoop` - While/until loops
- `SwitchStatement` - Switch/match expressions
- `ReturnStatement` - Return statements
- `BreakStatement` / `ContinueStatement`

### Expressions
- `BinaryExpression` - Binary operations (+, -, &&, etc.)
- `UnaryExpression` - Unary operations (!, -, *)
- `CallExpression` - Function calls
- `MemberExpression` - Property access (a.b)
- `ArrayExpression` - Array/list literals

### Types
- `TypeAnnotation` - Type hints, signatures
- `GenericType` - Generic/parameterized types
- `UnionType` - Union types (A | B)
- `IntersectionType` - Intersection types (A & B)

### Literals
- `StringLiteral`, `NumberLiteral`, `BooleanLiteral`
- `NullLiteral` - null, nil, None

## Metrics

With `cst_ts` enabled, Prometheus metrics track mapping quality:

```prometheus
# Successful canonical mappings by language
canonical_mapping_applied_total{language="rust"} 1234

# Unknown/fallback mappings by language  
canonical_mapping_unknown_total{language="python"} 5
```

Monitor these to identify:
- Languages with poor canonical coverage
- New language constructs needing mapping support
- Regression in mapping quality

**Target:** `unknown_total < 1%` of `applied_total` per language.

## Error Handling

All errors go through PII redaction before logging:

```rust
// Error messages automatically redact sensitive data
match pipeline.process_file(path).await {
    Ok(result) => { /* ... */ }
    Err(e) => {
        // Error logged with PII redacted
        eprintln!("Failed to process file: {}", e);
    }
}
```

## Performance

### Parse Times (typical)

| File Size | Lines | Parse + Transform | Memory |
|-----------|-------|------------------|--------|
| Small     | <500  | <5ms             | <1MB   |
| Medium    | 1-5K  | 10-50ms          | 2-5MB  |
| Large     | 10K+  | 100-500ms        | 10-50MB|

### Optimization Tips

1. **Cache AST results** - Pipeline maintains internal cache by path
2. **Batch processing** - Process multiple files concurrently
3. **Incremental updates** - Phase B will enable delta indexing

## Testing

### Run Tests

```bash
# Default features
cargo test --lib

# With cst_ts feature
cargo test --lib --no-default-features --features cst_ts

# CST canonical mapping tests
cargo test --lib --features cst_ts processors::cst_to_ast_pipeline::cst_to_ast_tests
```

### Test Coverage

- ✅ Canonical mapping for 6 languages
- ✅ Identifier extraction
- ✅ Cross-language consistency
- ✅ Error handling and fallbacks
- ✅ Metrics instrumentation

## CI/CD

The GitHub Actions workflow tests both feature configurations:

```yaml
# .github/workflows/semantic_search_ci.yml
- Default features with clippy -D warnings
- cst_ts feature with clippy -D warnings
- Both configurations must pass
```

## Migration from Legacy

**Before (language-specific):**
```rust
match cst.kind.as_str() {
    "function_declaration" => AstNodeType::FunctionDeclaration,
    "if_statement" => AstNodeType::IfStatement,
    // ... 50+ language-specific mappings
}
```

**After (canonical):**
```rust
#[cfg(feature = "cst_ts")]
let node_type = get_node_type_with_canonical(language, &cst.kind);
// Automatically maps across all languages
```

## Troubleshooting

### High `unknown_total` Metrics

**Cause:** Language constructs without canonical mapping

**Fix:** 
1. Check upstream `lapce-tree-sitter` for missing mappings
2. Add fallback mappings in language transformer
3. Report to CST-tree-sitter maintainers

### Compilation Errors with `cst_ts`

**Symptom:** Missing `lapce-tree-sitter` symbols

**Fix:** Ensure submodule is initialized:
```bash
git submodule update --init --recursive
```

### Performance Degradation

**Symptom:** Slow parse times for large files

**Solution (Phase B):** 
- Enable segmented storage
- Use incremental indexing
- Configure parse timeouts

## Future: Phase B

Coming in Phase B:

### Stable Node IDs
```rust
// Track nodes across file edits
let stable_id: u64 = node.metadata.stable_id?;
// Re-embed only changed nodes
```

### Incremental Indexing
```rust
// Only re-index changed functions
indexer.update_file(path, &edits).await?;
// 10-100x faster for large codebases
```

### CstApi Integration
```rust
// Segmented storage for massive files
let api = CstApiBuilder::new()
    .with_source(&source)
    .with_tree(&tree)
    .build()?;
```

## Resources

- [CST-tree-sitter Repository](../CST-tree-sitter/)
- [Canonical Kind Spec](../CST-tree-sitter/docs/canonical_kinds.md)
- [Prometheus Metrics](./metrics.md)
- [API Documentation](https://docs.rs/semantic_search)

## Contributing

To improve canonical mappings:

1. Identify missing mappings via `unknown_total` metrics
2. Add mappings to `lapce-tree-sitter`
3. Update `canonical_to_ast_node_type()` if needed
4. Add tests in `cst_to_ast_tests.rs`
5. Verify metrics improve

## License

SPDX-License-Identifier: Apache-2.0
