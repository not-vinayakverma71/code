# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2024-01-08

### Added
- Complete 6-phase optimization pipeline for 95% memory reduction
- Bytecode encoding/decoding with Tree-sitter integration
- Stable node IDs for semantic tracking across edits
- Canonical AST kind/field mappings for Rust, Python, JavaScript
- Edit delta/event API for incremental semantic updates
- Symbol extraction with Codex-compliant schema (<50ms/1K lines)
- Incremental parsing validation (<10ms for micro-edits)
- Prometheus metrics integration for observability
- CRC32 integrity checking for segmented bytecode
- Multi-tier cache system (hot/warm/cold/frozen)
- Memory-mapped source files support
- Cross-platform determinism tests
- Crash recovery and persistence tests
- Performance benchmarking with SLO enforcement
- Docker support with health checks
- Comprehensive CI/CD pipeline

### Performance
- Memory reduction: 95% vs baseline Tree-sitter
- Parse latency: <10ms for incremental edits
- Symbol extraction: <50ms for 1K lines
- Cache hit ratio: >90% for typical workloads
- Write throughput: >2000 ops/sec
- Read throughput: >5000 ops/sec

### Dependencies
- tree-sitter: 0.23.0
- All language grammars pinned at compatible versions
- See RELEASE.md for full version matrix

### Testing
- 68+ unit tests
- Cross-platform CI matrix (Linux/macOS/Windows)
- Fuzz testing for robustness
- Performance regression tests
- Memory stress tests

### Documentation
- Architecture overview
- API documentation
- Performance tuning guide
- Integration examples
