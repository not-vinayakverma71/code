# Semantic Search CLI Documentation

## Overview
Command-line tools for interacting with the semantic search system.

## Available Commands

### query_indexed_data
Query existing indexed data using semantic search.

**Usage:**
```bash
# Basic usage
cargo run --release --bin query_indexed_data

# With environment variables
AWS_REGION=us-east-1 \
AWS_ACCESS_KEY_ID=your_key \
AWS_SECRET_ACCESS_KEY=your_secret \
cargo run --release --bin query_indexed_data
```

**Exit Codes:**
- 0: Success
- 1: Database connection failed
- 2: AWS credentials error
- 3: Search engine initialization failed
- 4: Query execution failed

**Example Output:**
```
╔══════════════════════════════════════════════════════════════════╗
║      QUERYING INDEXED DATA USING OUR SEMANTIC SEARCH SYSTEM       ║
╚══════════════════════════════════════════════════════════════════╝

🔍 Phase 1: Checking for indexed data
  ✅ Found indexed database at: /tmp/semantic_search_test
  📊 Database size: 45.23 MB
  
📊 Phase 2: Memory Analysis
  Memory before loading: 120.45 MB
  Memory after loading system: 185.67 MB
  Memory used by system: 65.22 MB
```

### final_benchmark
Run comprehensive benchmarks against real AWS Titan embeddings.

**Usage:**
```bash
cargo run --release --bin final_benchmark -- \
  --iterations 100 \
  --output results/benchmark_$(date +%Y%m%d).json
```

### real_memory_benchmark
Profile memory usage during search operations.

**Usage:**
```bash
cargo run --release --bin real_memory_benchmark
```

## Environment Variables

| Variable | Description | Required | Default |
|----------|-------------|----------|---------|
| AWS_REGION | AWS region for Titan | Yes | - |
| AWS_ACCESS_KEY_ID | AWS access key | Yes | - |
| AWS_SECRET_ACCESS_KEY | AWS secret key | Yes | - |
| INDEX_OPTIMIZATION_THRESHOLD | Batch size for optimization | No | 100 |
| INDEX_COMPACTION_INTERVAL | Seconds between compactions | No | 3600 |
| INDEX_COMPACTION_ENABLED | Enable periodic compaction | No | true |
| RUST_LOG | Logging level | No | info |

## Troubleshooting

### Database Connection Failed
```
Error: Failed to connect to database
```
**Solution:** Ensure the database path exists and has proper permissions.

### AWS Credentials Error
```
Error: Failed to create AWS Titan embedder - check AWS credentials
```
**Solution:** Verify AWS credentials are set correctly in environment variables.

### Memory Issues
```
Error: Out of memory
```
**Solution:** Increase system memory or reduce batch_size in configuration.

## Performance Tips

1. **Use Release Mode**: Always run with `--release` for production
2. **Cache Configuration**: Adjust cache_size based on available memory
3. **Batch Processing**: Use optimal_batch_size for better throughput
4. **Index Optimization**: Set INDEX_OPTIMIZATION_THRESHOLD based on workload

## Examples

### Search with Filters
```bash
# Search Rust files only
echo '{"language": "rust"}' | cargo run --release --bin query_indexed_data --filters
```

### Benchmark with Custom Settings
```bash
INDEX_OPTIMIZATION_THRESHOLD=50 \
cargo run --release --bin final_benchmark -- --warm-up 10 --iterations 1000
```

### Memory Profiling
```bash
# Profile with detailed output
RUST_LOG=debug cargo run --release --bin real_memory_benchmark
```
