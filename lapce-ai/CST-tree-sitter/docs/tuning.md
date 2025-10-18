# Performance Tuning Guide

## Memory Tuning

### Cache Tier Ratios

Default allocation (100MB total):
- Hot: 10% (10MB) - Frequently accessed
- Warm: 20% (20MB) - Recently used
- Cold: 30% (30MB) - Occasional access
- Frozen: 40% (40MB) - Archived

Adjust based on workload:

```rust
// Read-heavy workload
let config = Phase4Config {
    hot_tier_ratio: 0.3,   // More hot cache
    warm_tier_ratio: 0.3,  // More warm cache
    cold_tier_ratio: 0.2,  // Less cold
    frozen_tier_ratio: 0.2, // Less frozen
    ..Default::default()
};

// Write-heavy workload
let config = Phase4Config {
    hot_tier_ratio: 0.1,   // Less hot (high churn)
    warm_tier_ratio: 0.2,  // Standard warm
    cold_tier_ratio: 0.3,  // Standard cold
    frozen_tier_ratio: 0.4, // More frozen (stable)
    ..Default::default()
};
```

### Segment Size

Default: 256KB per segment

```rust
// Small files (< 100KB average)
const SEGMENT_SIZE: usize = 64 * 1024;  // 64KB

// Large files (> 1MB average)
const SEGMENT_SIZE: usize = 512 * 1024; // 512KB
```

Trade-offs:
- Smaller segments = More granular loading, higher overhead
- Larger segments = Less overhead, more memory per load

### Parser Pool Size

```rust
// CPU-bound workload
let pool_size = num_cpus::get() * 2;

// I/O-bound workload
let pool_size = num_cpus::get() * 4;

// Memory-constrained environment
let pool_size = num_cpus::get();
```

## Latency Optimization

### Incremental Parsing

Enable for interactive editing:

```rust
let mut parser = IncrementalParser::new("rust").unwrap();

// First parse (full)
let tree = parser.parse_full(source).unwrap();

// Subsequent edits (incremental)
let edit = IncrementalParser::create_edit(
    old_source,
    new_source,
    start_byte,
    old_end_byte,
    new_end_byte,
);
let tree = parser.parse_incremental(new_source, edit).unwrap();
```

### Prefetching

Prewarm cache for predictable access:

```rust
// Prefetch related files
cache.prefetch(&[
    "src/main.rs",
    "src/lib.rs",
    "src/module.rs",
]);

// Prefetch by pattern
cache.prefetch_pattern("src/**/*.rs");
```

### Compression Settings

```rust
// Fast compression (lower ratio, faster)
use lz4::EncoderBuilder;
let encoder = EncoderBuilder::new()
    .level(1)  // Speed priority
    .build();

// High compression (higher ratio, slower)
use zstd::stream::Encoder;
let encoder = Encoder::new(writer, 9).unwrap(); // Max compression
```

## Throughput Optimization

### Batch Operations

```rust
// Inefficient: Individual operations
for file in files {
    cache.store(file.path, file.tree, file.source)?;
}

// Efficient: Batch operation
cache.store_batch(files.iter().map(|f| {
    (f.path.clone(), f.tree.clone(), f.source.clone())
}).collect())?;
```

### Parallel Processing

```rust
use rayon::prelude::*;

// Process files in parallel
files.par_iter()
    .map(|file| parse_file(file))
    .collect::<Vec<_>>();

// Control parallelism
rayon::ThreadPoolBuilder::new()
    .num_threads(4)
    .build_global()
    .unwrap();
```

### I/O Optimization

```rust
// Use memory-mapped files for large sources
use memmap2::MmapOptions;
let file = File::open(path)?;
let mmap = unsafe { MmapOptions::new().map(&file)? };

// Async I/O for non-blocking operations
use tokio::fs;
let contents = fs::read(path).await?;
```

## Language-Specific Tuning

### Rust

```toml
[languages.rust]
# Rust files are typically medium-sized
segment_size = 256_000
# Complex ASTs need more cache
hot_tier_ratio = 0.15
# Enable macro expansion caching
cache_expansions = true
```

### Python

```toml
[languages.python]
# Python files are typically smaller
segment_size = 128_000
# Simpler ASTs need less cache
hot_tier_ratio = 0.08
# Cache import resolutions
cache_imports = true
```

### JavaScript/TypeScript

```toml
[languages.javascript]
# JS can have large bundled files
segment_size = 512_000
# Dynamic nature needs more hot cache
hot_tier_ratio = 0.20
# Cache module resolutions
cache_modules = true
```

## Monitoring & Profiling

### Enable Detailed Metrics

```rust
// Enable all metrics
std::env::set_var("CST_METRICS_DETAILED", "true");

// Enable specific categories
std::env::set_var("CST_METRICS_CACHE", "true");
std::env::set_var("CST_METRICS_PARSE", "true");
std::env::set_var("CST_METRICS_IO", "true");
```

### Performance Profiling

```bash
# CPU profiling
perf record -g ./target/release/benchmark_performance
perf report

# Memory profiling
valgrind --tool=massif ./target/release/benchmark_performance
ms_print massif.out.*

# Cache misses
perf stat -e cache-misses,cache-references ./target/release/benchmark_performance
```

### Bottleneck Identification

Common bottlenecks and solutions:

| Symptom | Likely Cause | Solution |
|---------|--------------|----------|
| High latency spikes | Cache misses | Increase hot tier size |
| Steady memory growth | Memory leak | Check for Arc cycles |
| High CPU usage | Excessive parsing | Enable incremental parsing |
| High I/O wait | Segment thrashing | Increase segment cache |
| Low throughput | Lock contention | Use parking_lot::RwLock |

## Configuration Examples

### Development Environment

```toml
# Optimize for fast iteration
[dev]
memory_budget_mb = 50
hot_tier_ratio = 0.4
segment_size = 64_000
compression = false
debug_logging = true
```

### Production Server

```toml
# Optimize for efficiency
[prod]
memory_budget_mb = 500
hot_tier_ratio = 0.1
segment_size = 256_000
compression = true
metrics_enabled = true
```

### CI Environment

```toml
# Optimize for speed
[ci]
memory_budget_mb = 200
hot_tier_ratio = 0.3
segment_size = 128_000
compression = false
parallel_jobs = 4
```

## Benchmarking Commands

```bash
# Basic benchmark
cargo run --release --bin benchmark_performance

# With custom settings
CST_MEMORY_MB=200 CST_SEGMENT_SIZE=512000 \
  cargo run --release --bin benchmark_performance

# Compare configurations
./scripts/benchmark_compare.sh config1.toml config2.toml

# Stress test
cargo run --release --bin stress_test -- \
  --files 10000 \
  --threads 32 \
  --duration 3600
```
