# Changelog

All notable changes to the Semantic Search System will be documented in this file.

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
