# Changelog

All notable changes to the Semantic Search System will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2025-10-11

### Added - Incremental Indexing System

- **Stable ID-based Caching**
  - `StableIdEmbeddingCache`: LRU cache with configurable size limits
  - Cache hit rates of 80-95% in production scenarios
  - Per-entry storage: ~400 bytes (embedding + metadata)
  - Thread-safe operations with RwLock

- **Incremental Change Detection**
  - `IncrementalDetector`: Compares CST trees by stable IDs
  - Identifies unchanged, modified, added, and deleted nodes
  - O(n) complexity for change detection
  - Throughput: ~280k nodes/sec

- **Cached Embedder**
  - `CachedEmbedder`: Intelligent caching wrapper for embedding models
  - Automatic cache lookup before generation
  - 15x speedup for cached embeddings
  - Statistics tracking (hits, misses, generation count)

- **Async Indexing**
  - `AsyncIndexer`: Concurrent indexing with configurable parallelism
  - Queue-based work distribution
  - Backpressure control with configurable queue capacity
  - Graceful shutdown with timeout

- **Feature Flags**
  - Runtime-configurable processing modes
  - Toggle between mapping-only and full CstApi
  - Stable ID generation strategies
  - Cache persistence modes
  - Thread-safe atomic updates

- **Metrics & Observability**
  - `IndexingMetrics`: Prometheus integration
  - Cache hit/miss counters
  - Embedding latency histograms
  - Incremental vs full indexing counters
  - Node processing rates

### Performance Improvements

- **Incremental Re-indexing**
  - 5-100x faster than full re-parsing for typical edits
  - <1ms per 1k nodes for change detection
  - 13.8x speedup for 1% file modification
  - 7.3x speedup for 5% file modification

- **Cache Performance**
  - Hot path: <50μs for cache hits
  - Memory efficient: ~400 bytes per cached entry
  - Scales linearly with codebase size
  - LRU eviction prevents unbounded growth

### Testing & Quality

- **Test Coverage**
  - 76/76 tests passing across all modules
  - Property-based tests for correctness
  - Large file tests (1k-50k nodes)
  - Integration tests for end-to-end flows
  - Stress tests for 50k+ node files

- **Benchmarks**
  - Criterion-based performance benchmarks
  - Cache operation microbenchmarks
  - Change detection benchmarks
  - Throughput comparison tests
  - Large file scenario benchmarks

- **CI/CD**
  - Code coverage workflows (target: 80%)
  - Large file benchmark job (opt-in)
  - Module-specific coverage tracking
  - Integration test coverage
  - Benchmark coverage in test mode

### Documentation

- **Performance Results**
  - Comprehensive performance analysis in `docs/performance_results.md`
  - Real-world scenario benchmarks
  - Configuration recommendations
  - ROI analysis and break-even points

- **Upstream Specifications**
  - Cache integration test specs (`docs/upstream_cache_integration.md`)
  - Tiered cache metrics spec (`docs/upstream_metrics_spec.md`)
  - Property/fuzz validation tests (`docs/upstream_validation_tests.md`)
  - Storage format versioning (`docs/upstream_storage_format.md`)

- **Security**
  - Safe cleanup script using `trash-put` (`scripts/cleanup.sh`)
  - No permanent file deletion in scripts
  - Recoverable cleanup operations

### Changed

- Moved from monolithic parsing to incremental approach
- Cache-first strategy for embedding generation
- Async-by-default for indexing operations
- Configurable feature flags for flexibility

### Upstream Requirements

**CST-tree-sitter Integration Points:**
- `Phase4Cache::load_api_from_cache()` - Required for cache integration
- Tiered cache hit/miss metrics - Required for observability
- Bytecode encode/decode validation - Required for correctness
- Storage format versioning - Required for migrations

### Migration Guide

See `docs/MIGRATION.md` for:
- Upgrading from 1.0.x to 1.1.0
- Configuration changes
- Feature flag usage
- Rollback procedures

---

## [1.0.0] - 2025-10-10

### Added
- **Core Features**
  - High-performance semantic search with AWS Titan embeddings
  - Filter-aware caching system achieving >80% hit rate
  - IVF_PQ index optimization with configurable thresholds
  - Periodic index compaction service with backpressure control
  - Multi-language CST-based parsing (Rust, TypeScript, Python, Go, Java, C++)

- **Observability**
  - Comprehensive Prometheus metrics (counters, histograms, gauges)
  - Correlation ID propagation for distributed tracing
  - Memory RSS monitoring with 10-second updates
  - Prometheus alerting rules for production monitoring
  - Tracing with PII redaction in logs

- **Security**
  - PII redaction for logs and metrics
  - Secure credential handling (no hardcoded secrets)
  - Rate limiting support
  - Fuzz-tested redaction patterns

- **Performance**
  - P50 latency: 45ms
  - P95 latency: 120ms
  - Throughput: 150 QPS
  - Cache hit latency: <5ms
  - Memory usage: 185MB baseline

- **CLI Tools**
  - `query_indexed_data`: Production-ready query tool with error handling
  - `final_benchmark`: Comprehensive performance benchmarking
  - `real_memory_benchmark`: Memory profiling tool

- **Testing**
  - 15+ comprehensive test suites
  - Cache effectiveness tests
  - Index compaction integration tests
  - PII redaction fuzz tests
  - Correlation ID propagation tests

### Fixed
- Double-counting of cache misses in metrics
- Memory leaks in long-running operations
- Error handling in CLI tools (replaced unwraps with proper errors)

### Changed
- Optimization threshold now configurable via `INDEX_OPTIMIZATION_THRESHOLD`
- Compaction interval configurable via `INDEX_COMPACTION_INTERVAL`
- Enhanced error messages with actionable context

### Documentation
- Complete CLI usage documentation
- Operator runbooks for deployment and operations
- Performance benchmarks in RESULTS.md
- Environment variable documentation

## [0.9.0] - 2025-10-01 (Pre-release)

### Added
- Initial semantic search implementation
- Basic caching system
- AWS Titan integration
- LanceDB vector storage

### Known Issues
- Arrow/DataFusion version incompatibilities (63 compilation errors)
- Requires LanceDB fork compatibility fixes

---

For detailed performance metrics, see [RESULTS.md](./RESULTS.md)
For deployment instructions, see [docs/operations/deployment.md](./docs/operations/deployment.md)
