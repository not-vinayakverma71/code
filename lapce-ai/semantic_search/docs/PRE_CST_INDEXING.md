# Pre-CST Semantic Search Pipeline

## Overview

This document describes the production-ready semantic search indexing pipeline before CST (Concrete Syntax Tree) integration. The system uses fallback line-based chunking and is fully functional for indexing and searching code.

## Architecture

```
┌─────────────┐     ┌──────────────┐     ┌───────────────┐
│ File Scanner├────>│ Code Parser  ├────>│ Embedding Gen │
└─────────────┘     └──────────────┘     └───────────────┘
                            │                      │
                            v                      v
                    ┌──────────────┐     ┌───────────────┐
                    │ Code Chunks  │     │ Embeddings    │
                    └──────────────┘     └───────────────┘
                            │                      │
                            └──────────┬───────────┘
                                      v
                            ┌───────────────────┐
                            │ LanceDB Storage   │
                            └───────────────────┘
```

## Components

### 1. File Discovery (`processors/scanner.rs`)
- Uses `walkdir` to recursively find files
- Respects ignore patterns (node_modules, .git, target, etc.)
- Limits: 50,000 files max per scan

### 2. Code Parsing (`processors/parser.rs`)
- Fallback line-based chunking (4KB chunks)
- Smart re-balancing to avoid splitting logical units
- Minimum chunk size: 100 chars
- Maximum chunk size: 6KB (with 1.5x tolerance)

### 3. Embedding Generation
- AWS Titan via Bedrock (production)
- Batch size: 100 chunks
- Parallel embedding creation

### 4. Storage
- **Vector Store**: LanceDB for persistence
- **Cache**: Hierarchical 3-tier cache
  - L1: Hot (1MB, uncompressed)
  - L2: Warm (3MB, zstd compressed)
  - L3: Cold (memory-mapped)
- **File Hash Cache**: Persistent JSON for incremental updates

### 5. Concurrency Settings
- Parsing: 10 concurrent files
- Batch processing: 5 concurrent batches
- Max pending batches: 3

## Configuration

### Environment Variables
```bash
# AWS Credentials for embeddings
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
export AWS_REGION=us-east-1

# Optional: Custom cache directory
export LAPCE_CACHE_DIR=/path/to/cache
```

### Tuning Parameters

In `processors/scanner.rs`:
```rust
const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024;  // 10MB
const BATCH_SEGMENT_THRESHOLD: usize = 100;         // Chunks per batch
const PARSING_CONCURRENCY: usize = 10;              // Parallel file parsing
const BATCH_PROCESSING_CONCURRENCY: usize = 5;      // Parallel batches
```

In `processors/parser.rs`:
```rust
const MAX_BLOCK_CHARS: usize = 4000;               // ~4KB chunks
const MIN_BLOCK_CHARS: usize = 100;                // Minimum chunk
const MIN_CHUNK_REMAINDER_CHARS: usize = 500;      // Avoid tiny remainders
```

## Running Tests

### Unit Tests
```bash
cd semantic_search
cargo test --lib
```

### Integration Tests
```bash
cargo test --test integration_test
```

### End-to-End Test
```bash
# Create a test workspace
mkdir -p /tmp/test_workspace
echo "fn main() { println!(\"Hello\"); }" > /tmp/test_workspace/main.rs

# Run indexing (requires AWS credentials)
cargo run --example index_workspace /tmp/test_workspace

# Search
cargo run --example search_code "println"
```

## Performance Metrics

With production settings on a typical codebase:

- **Throughput**: ~1,000 files/second
- **Embedding Latency**: 50-200ms per batch (100 chunks)
- **Memory Usage**: 
  - Base: ~50MB
  - With full caches: ~100MB
  - Under load: <200MB
- **Cache Hit Rate**: 80-95% after warm-up

## Monitoring

Key metrics logged:
- Files processed/skipped
- Blocks generated per file
- Embedding creation time
- Vector upsert time
- Cache hit/miss rates

Enable debug logging:
```bash
RUST_LOG=debug cargo run
```

## Troubleshooting

### Common Issues

1. **High memory usage**: Reduce batch sizes and cache limits
2. **Slow indexing**: Check AWS latency, reduce concurrency
3. **Files not indexed**: Check ignore patterns, file size limits
4. **Cache misses**: Increase L1/L2 cache sizes

### Debug Commands

```bash
# Check cache contents
ls -la ~/.lapce_cache/

# Monitor memory during indexing
watch -n 1 'ps aux | grep semantic_search'

# Profile performance
cargo build --release
perf record -g ./target/release/semantic_search
perf report
```

## Next Steps

This pipeline is ready for CST integration:
1. Replace fallback chunking with CST-based semantic chunks
2. Add language-specific transformers
3. Integrate with `CST-tree-sitter` for parse tree generation
4. Enhance chunk metadata with AST node information

See `CST_INTEGRATION.md` for the roadmap.
