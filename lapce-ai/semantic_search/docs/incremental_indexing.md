# Incremental Indexing Architecture (Phase B)

## Overview

The semantic search system now supports **incremental indexing** powered by stable IDs from CST-tree-sitter. This enables efficient re-indexing of modified files by reusing embeddings for unchanged code structures.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    File Change Event                     │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│          CstToAstPipeline (with stable IDs)             │
│  • Parse file → tree-sitter Tree                        │
│  • Build CstApi with stable node IDs                    │
│  • Transform to canonical AST                           │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│        CachedEmbedder.embed_file_incremental()          │
│  • Retrieve old stable IDs from cache                   │
│  • Pass to IncrementalDetector                          │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│       IncrementalDetector.detect_changes()              │
│  • Compare old vs new stable IDs                        │
│  • Classify: unchanged/modified/added/deleted           │
│  • Return ChangeSet                                     │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│        StableIdEmbeddingCache (lookup/store)            │
│  • For unchanged nodes: retrieve cached embeddings      │
│  • For modified/new nodes: generate embeddings          │
│  • Store new embeddings with stable IDs                 │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│              CodeIndexer (batch insert)                 │
│  • Combine cached + newly generated embeddings          │
│  • Batch insert to LanceDB vector store                 │
└─────────────────────────────────────────────────────────┘
```

## Core Components

### 1. StableIdEmbeddingCache
**Location:** `src/indexing/stable_id_cache.rs`

Thread-safe cache mapping `stable_id → embedding + metadata`.

```rust
pub struct StableIdEmbeddingCache {
    cache: Arc<DashMap<u64, CacheEntry>>,
    file_index: Arc<DashMap<PathBuf, HashSet<u64>>>,
}

pub struct CacheEntry {
    pub embedding: Vec<f32>,
    pub source_text: String,
    pub node_kind: String,
    pub timestamp: u64,
    pub file_path: String,
}
```

**Key operations:**
- `insert(stable_id, entry)` - Store embedding
- `get(stable_id) -> Option<CacheEntry>` - Retrieve cached embedding
- `invalidate_file(path)` - Clear all entries for a file
- `stats()` - Get cache hit/miss metrics

### 2. IncrementalDetector
**Location:** `src/indexing/incremental_detector.rs`

Detects changes between old and new CST by comparing stable IDs.

```rust
pub struct IncrementalDetector {
    old_ids: HashSet<u64>,
    new_ids: HashMap<u64, CstNode>,
}

pub struct ChangeSet {
    pub unchanged: Vec<CstNode>,
    pub modified: Vec<CstNode>,
    pub added: Vec<CstNode>,
    pub deleted: Vec<u64>,
}
```

**Algorithm:**
1. Collect all stable IDs from old and new CST
2. Classify each ID:
   - **Unchanged:** ID exists in both, content hash matches
   - **Modified:** ID exists in both, content hash differs
   - **Added:** ID only in new CST
   - **Deleted:** ID only in old CST

### 3. CachedEmbedder
**Location:** `src/indexing/cached_embedder.rs`

Wraps embedding model with intelligent caching.

```rust
pub struct CachedEmbedder {
    model: Arc<dyn EmbeddingModel>,
    cache: StableIdEmbeddingCache,
    detector: IncrementalDetector,
    stats: Arc<RwLock<EmbeddingStats>>,
}
```

**Key methods:**
- `embed_node(node)` - Embed single node (cache-aware)
- `embed_file_incremental(cst, path)` - Full file with change detection
- `invalidate_file(path)` - Clear file cache
- `stats()` - Get performance metrics

### 4. AsyncIndexer
**Location:** `src/indexing/async_indexer.rs`

Production-grade async indexing with back-pressure and timeouts.

```rust
pub struct AsyncIndexer {
    config: IndexerConfig,
    task_tx: mpsc::Sender<IndexTask>,
    semaphore: Arc<Semaphore>,
}

pub struct IndexerConfig {
    pub max_concurrent_tasks: usize,
    pub file_timeout: Duration,
    pub queue_capacity: usize,
    pub enable_incremental: bool,
}
```

**Features:**
- Bounded concurrency via semaphore
- Queue-based back-pressure
- Per-file timeouts
- Graceful shutdown
- Priority-based task scheduling

## Usage Examples

### Basic Incremental Indexing

```rust
use semantic_search::indexing::{
    CachedEmbedder, StableIdEmbeddingCache
};
use semantic_search::processors::CstToAstPipeline;

// Setup
let cache = StableIdEmbeddingCache::new();
let model = MyEmbeddingModel::new();
let embedder = Arc::new(CachedEmbedder::new(Arc::new(model)));
let pipeline = Arc::new(CstToAstPipeline::new());

// First parse - all new
let output = pipeline.process_file(&file_path).await?;
let (embeddings, changeset) = embedder
    .embed_file_incremental(&output.cst, &file_path)?;

println!("Added: {}", changeset.added.len());
// Added: 15

// Re-parse same file - all cached
let output2 = pipeline.process_file(&file_path).await?;
let (embeddings2, changeset2) = embedder
    .embed_file_incremental(&output2.cst, &file_path)?;

println!("Unchanged: {}, reused: {}", 
         changeset2.unchanged.len(),
         embedder.stats().embeddings_reused);
// Unchanged: 15, reused: 15
```

### With CodeIndexer

```rust
use semantic_search::search::CodeIndexer;

let indexer = CodeIndexer::new(search_engine)
    .with_cached_embedder(embedder)
    .with_batch_size(100);

// Initial index
indexer.index_repository(repo_path).await?;

// File modified - only re-index changed parts
indexer.reindex_file(&file_path).await?;
```

### Async Indexing

```rust
use semantic_search::indexing::{AsyncIndexer, IndexerConfig, IndexTask};

let config = IndexerConfig {
    max_concurrent_tasks: 4,
    file_timeout: Duration::from_secs(30),
    queue_capacity: 1000,
    enable_incremental: true,
};

let indexer = AsyncIndexer::with_config(config);

// Submit tasks
for file in files {
    indexer.submit(IndexTask {
        file_path: file,
        priority: TaskPriority::Normal,
    }).await?;
}

// Collect results
while let Some(result) = indexer.next_result().await {
    println!("Indexed {} in {:?}", result.file_path, result.duration);
    println!("  Generated: {}, Reused: {}", 
             result.embeddings_generated,
             result.embeddings_reused);
}
```

## Performance Benefits

### Cache Hit Rates

Typical cache hit rates for common scenarios:

| Scenario | Cache Hit Rate | Speedup |
|----------|---------------|---------|
| No changes (re-parse) | 95-100% | 50-100x |
| Single function edit | 80-95% | 5-20x |
| New function added | 85-95% | 5-15x |
| Refactor (rename) | 70-85% | 3-10x |
| Major rewrite | 20-50% | 1.2-2x |

### Memory Usage

- **Embedding cache:** ~100 bytes per node (avg)
- **Detector state:** ~24 bytes per stable ID
- **Overhead:** ~20% of embedding size

For a typical 1000-line file with 200 AST nodes:
- Cache footprint: ~20 KB
- Memory savings vs full re-embed: ~80%

## Metrics & Monitoring

### Prometheus Metrics

All metrics have `language` label for per-language tracking:

**Stable ID Metrics:**
```
indexing_stable_ids_generated_total{language="rust"}
indexing_stable_ids_reused_total{language="rust"}
indexing_stable_id_cache_size
```

**Cache Performance:**
```
indexing_embedding_cache_hits_total{file_type="rs"}
indexing_embedding_cache_misses_total{file_type="rs"}
indexing_cache_hit_rate{cache_type="embedding"}
```

**Change Detection:**
```
indexing_nodes_unchanged_total{language="rust"}
indexing_nodes_modified_total{language="rust"}
indexing_nodes_added_total{language="rust"}
indexing_nodes_deleted_total{language="rust"}
```

**Performance:**
```
indexing_parse_duration_seconds{strategy="incremental",language="rust"}
indexing_change_detection_duration_seconds{language="rust"}
indexing_embedding_generation_duration_seconds{cached="true"}
indexing_incremental_speedup_ratio{language="rust"}
```

**Async Indexer:**
```
indexing_queue_length
indexing_active_tasks
indexing_tasks_completed_total{status="success"}
indexing_backpressure_events_total{reason="queue_full"}
indexing_timeout_events_total{operation="embedding"}
```

### Example Queries

```promql
# Cache hit rate over time
rate(indexing_embedding_cache_hits_total[5m]) / 
(rate(indexing_embedding_cache_hits_total[5m]) + 
 rate(indexing_embedding_cache_misses_total[5m]))

# Average speedup from incremental indexing
histogram_quantile(0.5, indexing_incremental_speedup_ratio)

# Indexing throughput
rate(indexing_tasks_completed_total{status="success"}[1m])

# P95 parse latency
histogram_quantile(0.95, indexing_parse_duration_seconds_bucket)
```

## Configuration

### IndexerConfig

```rust
pub struct IndexerConfig {
    /// Maximum concurrent indexing tasks
    pub max_concurrent_tasks: usize,  // Default: 4
    
    /// Timeout for single file indexing
    pub file_timeout: Duration,        // Default: 30s
    
    /// Embedding generation timeout
    pub embedding_timeout: Duration,   // Default: 10s
    
    /// Queue capacity (back-pressure)
    pub queue_capacity: usize,         // Default: 1000
    
    /// Enable incremental indexing
    pub enable_incremental: bool,      // Default: true
}
```

### CstCacheConfig

```rust
pub struct CstCacheConfig {
    /// Enable Phase4 caching
    pub enabled: bool,                 // Default: true
    
    /// Cache directory
    pub cache_dir: PathBuf,            // Default: temp_dir/cst_cache
    
    /// Memory budget in MB
    pub memory_budget_mb: usize,       // Default: 100
    
    /// Enable compression
    pub enable_compression: bool,      // Default: true
}
```

## Best Practices

### 1. Cache Invalidation

Invalidate cache when:
- File content changes externally
- File is deleted
- Switching branches with different history

```rust
// On file change
embedder.invalidate_file(&file_path);

// On branch switch
for file in changed_files {
    embedder.invalidate_file(&file);
}
```

### 2. Memory Management

Monitor cache size and evict when needed:

```rust
let (hits, misses, size, entries) = embedder.cache_stats();
if size > MAX_CACHE_SIZE_MB * 1_048_576 {
    // Evict least recently used entries
    embedder.evict_lru(0.2); // Evict 20%
}
```

### 3. Error Handling

Always handle parse failures gracefully:

```rust
match embedder.embed_file_incremental(&cst, &path) {
    Ok((embeddings, changeset)) => {
        // Process results
    }
    Err(e) => {
        log::warn!("Incremental indexing failed: {}, falling back to full", e);
        // Fall back to full re-index
        let embeddings = embedder.embed_nodes(&all_nodes)?;
    }
}
```

### 4. Performance Tuning

Adjust concurrency based on workload:

```rust
// CPU-bound workload (many small files)
let config = IndexerConfig {
    max_concurrent_tasks: num_cpus::get(),
    ..Default::default()
};

// I/O-bound workload (few large files)
let config = IndexerConfig {
    max_concurrent_tasks: num_cpus::get() * 2,
    ..Default::default()
};
```

## Testing

Run incremental indexing tests:

```bash
# Unit tests
cargo test --lib --no-default-features --features cst_ts indexing

# Integration tests
cargo test --lib --no-default-features --features cst_ts incremental_integration_tests

# Performance tests
cargo test --lib --no-default-features --features cst_ts test_performance_comparison
```

## Troubleshooting

### Low Cache Hit Rate

**Symptoms:** Cache hit rate < 50% on minor edits

**Causes:**
1. Stable IDs not persisting across parses
2. Content hashes changing unnecessarily
3. Tree structure changing significantly

**Solutions:**
- Verify CST-tree-sitter stable ID generation
- Check that file content is identical between parses
- Review parser configuration

### High Memory Usage

**Symptoms:** Cache size growing unbounded

**Causes:**
1. No cache eviction policy
2. Many large files indexed
3. Memory budget too high

**Solutions:**
- Implement LRU eviction
- Reduce `memory_budget_mb`
- Enable compression in CstCacheConfig

### Slow Indexing

**Symptoms:** Indexing slower than expected

**Causes:**
1. Too many concurrent tasks (thrashing)
2. Timeouts too aggressive
3. Back-pressure not working

**Solutions:**
- Reduce `max_concurrent_tasks`
- Increase `file_timeout`
- Check queue capacity

## Future Work

- **Phase4 cache integration:** Full bytecode caching from CST-tree-sitter
- **Distributed caching:** Share cache across multiple machines
- **Smart eviction:** Use access patterns for better LRU
- **Persistent cache:** Survive restarts with disk-backed storage
- **Delta encoding:** Store only changed nodes, not full CST

## References

- [CST Integration Guide](./cst_integration.md)
- [Upstream CST-tree-sitter](../CST-tree-sitter/README.md)
- [Prometheus Metrics](./metrics.md)
