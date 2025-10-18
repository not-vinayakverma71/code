# Migration Guide: v1.0.x → v1.1.0

**Release Date:** 2025-10-11  
**Breaking Changes:** None  
**Migration Effort:** Low (configuration adjustments only)

## Overview

Version 1.1.0 introduces incremental indexing with stable IDs, providing 5-100x performance improvements for typical file edits. This is a **backward-compatible** release with optional feature enablement.

## Quick Start

### For Users

```bash
# Update dependency
cargo update -p lancedb

# No code changes required - incremental indexing works automatically
# with default configuration
```

### For Operators

```bash
# 1. Backup existing data
./scripts/backup.sh

# 2. Update configuration (optional)
cp config.toml config.toml.backup
nano config.toml  # Add new settings below

# 3. Restart service
systemctl restart semantic-search

# 4. Verify
cargo test --features cst_ts
```

## What's New

### Major Features

1. **Stable ID-based Caching**
   - Automatic embedding reuse for unchanged nodes
   - 80-95% cache hit rates in production
   - Configurable cache size limits

2. **Incremental Change Detection**
   - Fast CST comparison by stable IDs
   - O(n) complexity, ~280k nodes/sec throughput
   - Identifies unchanged/modified/added/deleted nodes

3. **Async Indexing**
   - Concurrent processing with configurable parallelism
   - Queue-based work distribution
   - Graceful shutdown with timeout

4. **Feature Flags**
   - Runtime-configurable processing modes
   - Toggle features without recompilation

## Configuration Changes

### New Configuration Options

Add these to your `config.toml`:

```toml
[incremental_indexing]
# Enable incremental indexing (default: true)
enabled = true

# Cache configuration
[incremental_indexing.cache]
# Maximum cache size in MB (default: 100)
max_size_mb = 100

# Enable LRU eviction (default: true)
enable_lru = true

# Eviction threshold as percentage (default: 0.9)
eviction_threshold = 0.9

# Enable cache persistence (default: false)
persist_cache = false

# Cache directory (when persistence enabled)
cache_dir = "./cache/embeddings"

# Async indexing configuration
[incremental_indexing.async]
# Maximum concurrent indexing tasks (default: 4)
max_concurrent_tasks = 4

# Queue capacity (default: 1000)
queue_capacity = 1000

# Shutdown timeout in seconds (default: 30)
shutdown_timeout_secs = 30

# Feature flags
[incremental_indexing.features]
# Enable stable ID generation (default: true)
stable_ids = true

# Enable incremental change detection (default: true)
incremental_detection = true

# Enable cached embeddings (default: true)
cached_embeddings = true

# Metrics configuration
[incremental_indexing.metrics]
# Enable Prometheus metrics (default: true)
enable_metrics = true

# Metrics port (default: 9090)
metrics_port = 9090
```

### Environment Variables

New environment variables (override config file):

```bash
# Cache size
export INCREMENTAL_CACHE_SIZE_MB=200

# Concurrent tasks
export INCREMENTAL_MAX_TASKS=8

# Queue capacity
export INCREMENTAL_QUEUE_CAPACITY=2000

# Feature flags
export INCREMENTAL_ENABLE_STABLE_IDS=true
export INCREMENTAL_ENABLE_CACHE=true
```

## Upgrade Procedures

### Standard Upgrade (Zero Downtime)

```bash
# Step 1: Pre-flight checks
./scripts/preflight_check.sh

# Step 2: Backup
./scripts/backup.sh --output ./backups/v1.0-$(date +%Y%m%d)

# Step 3: Update binary
cargo build --release --features cst_ts
sudo systemctl stop semantic-search
sudo cp target/release/semantic-search /usr/local/bin/
sudo systemctl start semantic-search

# Step 4: Verify
curl http://localhost:9090/metrics | grep incremental_
./scripts/smoke_test.sh

# Step 5: Monitor
journalctl -u semantic-search -f
```

### Gradual Rollout (Production)

For large deployments, use a gradual rollout:

```bash
# Enable feature flag gradually
# Week 1: 10% of traffic
curl -X POST http://localhost:8080/api/config \
  -d '{"incremental_indexing.enabled": true, "rollout_percentage": 10}'

# Week 2: 50% of traffic
curl -X POST http://localhost:8080/api/config \
  -d '{"rollout_percentage": 50}'

# Week 3: 100% of traffic
curl -X POST http://localhost:8080/api/config \
  -d '{"rollout_percentage": 100}'
```

### Docker Upgrade

```bash
# Pull new image
docker pull lancedb/semantic-search:1.1.0

# Stop old container
docker stop semantic-search

# Start new container with same volumes
docker run -d \
  --name semantic-search \
  -v /data/embeddings:/data/embeddings \
  -v /data/cache:/data/cache \
  -p 8080:8080 -p 9090:9090 \
  lancedb/semantic-search:1.1.0

# Verify
docker logs semantic-search
```

## Code Changes

### No Breaking Changes

**Good news:** v1.1.0 is fully backward compatible. Existing code continues to work without modification.

### Optional: Use New Features

If you want to explicitly use incremental features:

```rust
use lancedb::indexing::{
    StableIdEmbeddingCache,
    IncrementalDetector,
    CachedEmbedder,
};

// Create cache
let cache = StableIdEmbeddingCache::new();

// Create embedder with caching
let model = Arc::new(YourEmbeddingModel::new());
let embedder = CachedEmbedder::new(model);

// Create detector for change tracking
let mut detector = IncrementalDetector::new();

// First indexing pass
let changeset = detector.detect_changes(&file_path, &cst_tree);

// Re-index only changed nodes
for stable_id in changeset.modified.iter().chain(changeset.added.iter()) {
    let node = find_node_by_id(&cst_tree, *stable_id);
    let embedding = embedder.embed_node(&node, &file_path)?;
    // Store embedding...
}
```

## Performance Tuning

### Small Projects (<1k files)

```toml
[incremental_indexing.cache]
max_size_mb = 50

[incremental_indexing.async]
max_concurrent_tasks = 4
queue_capacity = 500
```

**Expected:** <1s re-index for typical edits

### Medium Projects (1k-10k files)

```toml
[incremental_indexing.cache]
max_size_mb = 200

[incremental_indexing.async]
max_concurrent_tasks = 8
queue_capacity = 2000
```

**Expected:** 1-5s re-index for typical edits

### Large Projects (>10k files)

```toml
[incremental_indexing.cache]
max_size_mb = 500
enable_lru = true
eviction_threshold = 0.9

[incremental_indexing.async]
max_concurrent_tasks = 16
queue_capacity = 5000
```

**Expected:** 5-15s re-index for typical edits

## Monitoring

### Key Metrics to Watch

```promql
# Cache hit rate (target: >80%)
rate(incremental_cache_hits_total[5m]) / 
  (rate(incremental_cache_hits_total[5m]) + rate(incremental_cache_misses_total[5m]))

# Embedding reuse (target: >85%)
rate(incremental_embeddings_reused_total[5m]) / 
  (rate(incremental_embeddings_generated_total[5m]) + rate(incremental_embeddings_reused_total[5m]))

# Change detection latency (target: <1ms per 1k nodes)
histogram_quantile(0.95, rate(incremental_change_detection_duration_seconds_bucket[5m]))

# Incremental vs full indexing ratio (target: >90% incremental)
rate(incremental_indexing_total[5m]) / 
  (rate(incremental_indexing_total[5m]) + rate(full_indexing_total[5m]))
```

### Grafana Dashboard

Import the pre-built dashboard:

```bash
# Import dashboard
curl -X POST http://grafana:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @monitoring/grafana_dashboard.json
```

## Rollback Procedures

### Quick Rollback

If issues occur, rollback is simple:

```bash
# Step 1: Stop service
sudo systemctl stop semantic-search

# Step 2: Restore old binary
sudo cp /usr/local/bin/semantic-search.v1.0 /usr/local/bin/semantic-search

# Step 3: Restore old config (optional)
cp config.toml.backup config.toml

# Step 4: Start service
sudo systemctl start semantic-search

# Step 5: Verify
./scripts/smoke_test.sh
```

### Disable Incremental Indexing Only

To keep v1.1.0 but disable incremental features:

```toml
[incremental_indexing]
enabled = false
```

Or via environment variable:

```bash
export INCREMENTAL_INDEXING_ENABLED=false
systemctl restart semantic-search
```

### Data Rollback

v1.1.0 doesn't change data format, so no data migration needed. However, if cache corruption occurs:

```bash
# Clear cache (safe - will be rebuilt)
trash-put ./cache/embeddings/*

# Restart to rebuild cache
systemctl restart semantic-search
```

## Troubleshooting

### Cache Performance Issues

**Symptom:** Low cache hit rate (<50%)

**Solution:**
```toml
# Increase cache size
[incremental_indexing.cache]
max_size_mb = 500

# Enable persistence
persist_cache = true
```

### High Memory Usage

**Symptom:** RSS >500MB for small codebases

**Solution:**
```toml
# Reduce cache size
[incremental_indexing.cache]
max_size_mb = 50
eviction_threshold = 0.8

# Reduce concurrent tasks
[incremental_indexing.async]
max_concurrent_tasks = 2
```

### Slow Incremental Detection

**Symptom:** Change detection takes >10ms per 1k nodes

**Solution:**
- Check for very deep nesting (>100 levels)
- Verify stable IDs are being generated
- Profile with: `cargo bench --bench incremental_indexing_bench`

### Metrics Not Appearing

**Symptom:** No incremental_* metrics in Prometheus

**Solution:**
```toml
[incremental_indexing.metrics]
enable_metrics = true
metrics_port = 9090
```

Verify: `curl http://localhost:9090/metrics | grep incremental`

## Known Issues

### Limitations

1. **First Parse:** No benefit until file is indexed once
2. **Large Refactors:** <3x speedup when >50% of file changes
3. **Cold Start:** Cache rebuilt after restart (unless persistence enabled)

### Workarounds

```toml
# Enable cache persistence to survive restarts
[incremental_indexing.cache]
persist_cache = true
cache_dir = "./cache/embeddings"
```

## Testing

### Verify Incremental Indexing

```bash
# Run integration tests
cargo test --features cst_ts --test integration_tests

# Run large file tests
cargo test --features cst_ts --test large_file_tests

# Run benchmarks
cargo bench --bench incremental_indexing_bench
```

### Performance Validation

```bash
# Before upgrade
./scripts/benchmark.sh > before.txt

# After upgrade
./scripts/benchmark.sh > after.txt

# Compare
diff before.txt after.txt
```

Expected improvements:
- ✅ 5-15x faster re-indexing for typical edits
- ✅ 80-95% cache hit rate
- ✅ <1ms change detection per 1k nodes

## Support

### Getting Help

- **Documentation:** `docs/`
- **Performance Results:** `docs/performance_results.md`
- **Issue Tracker:** GitHub Issues
- **Chat:** Discord #semantic-search

### Reporting Issues

Include:
1. Version: `cargo --version`
2. Config: `cat config.toml`
3. Logs: `journalctl -u semantic-search -n 100`
4. Metrics: `curl http://localhost:9090/metrics`

## Next Steps

After successful migration:

1. ✅ Monitor metrics for 24-48 hours
2. ✅ Tune configuration based on workload
3. ✅ Enable cache persistence if beneficial
4. ✅ Review performance results in Grafana
5. ✅ Update documentation for your team

---

**Migration Complete!** 🎉

You're now running semantic_search v1.1.0 with incremental indexing enabled.

Questions? Check `docs/FAQ.md` or file an issue.
