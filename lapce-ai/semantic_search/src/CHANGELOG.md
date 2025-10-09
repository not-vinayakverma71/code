# Changelog

All notable changes to the Semantic Search system will be documented in this file.

## [1.0.0] - 2025-10-08

### Added
- **AWS Titan Integration**: Production-grade embeddings with amazon.titan-embed-text-v2:0 (1536 dimensions)
- **CST Pipeline**: Semantic-aware code chunking via tree-sitter for 7+ languages
- **Filter-Aware Caching**: Query results isolated by search filters preventing cache bleed
- **Hierarchical Cache**: 3-tier cache system (memory + mmap + disk) for sub-5ms cache hits
- **IVF_PQ Indexing**: Optimized vector search with quantization (~75% memory reduction)
- **Incremental Indexing**: Real-time file change processing with debouncing (<100ms per file)
- **Prometheus Metrics**: Comprehensive observability with histograms, counters, and gauges
- **PII Redaction**: Security utility for redacting sensitive information from logs
- **Rate Limiting**: AWS Titan embedder with configurable RPS and concurrent request limits
- **Multi-Language Support**: CST tests for Rust, TypeScript, JavaScript, Python, Go, Java, C++

### Changed
- Replaced local BERT embeddings with AWS Titan for production quality
- Migrated from simple chunking to CST-based semantic chunking
- Updated cache implementation to be filter-aware with SHA256 keys
- Enhanced error handling with structured Result types (no unwraps in hot paths)

### Fixed
- Cache isolation issues with filter-aware keys
- Compilation errors in incremental_indexer.rs
- Memory leaks in long-running indexing operations
- Rate limiting and retry logic for AWS API calls

### Security
- No hardcoded credentials - all configuration from environment
- PII redaction in logs and metrics
- Rate limiting enforcement (10 RPS default)
- Input validation for embeddings

### Performance
- Search latency: p50 <50ms, p95 <200ms, p99 <500ms (targets)
- Cache hit rate: >80% for repeated queries
- Incremental update: <100ms per file
- Memory baseline: <100MB RSS

### Testing
- Multi-language CST tests (15+ test cases)
- AWS configuration hardening tests
- Security and rate limiting stress tests
- Cache effectiveness validation tests
- Index maintenance integration tests
- Metrics export validation

### Documentation
- Complete production architecture guide in docs/06-SEMANTIC-SEARCH-LANCEDB.md
- Performance validation framework in RESULTS.md
- Production readiness checklist
- CI/CD pipeline with IPC-grade standards

## [0.9.0] - 2024-12-01

### Added
- Initial LanceDB integration
- Basic semantic search functionality
- Simple file-based caching

## [0.8.0] - 2024-11-01

### Added
- Prototype embeddings with Candle
- Basic code chunking
