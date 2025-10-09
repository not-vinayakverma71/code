# Changelog

All notable changes to the Lapce-AI IPC system will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Canonical 24-byte binary protocol header with CRC32 validation
- Shared memory IPC with ring buffer implementation
- Connection pool with health checks and adaptive scaling
- Comprehensive test suite (fuzz, chaos, scalability, compatibility)
- Prometheus metrics and observability dashboards
- Structured logging with PII redaction and sampling
- Production-grade systemd service configuration
- Memory footprint validation (≤3MB baseline target)

### Changed
- All protocol fields now use Little-Endian encoding
- Unified codec implementations (binary_codec, zero_copy_codec)
- Replaced mock metrics with real Prometheus exports
- Enhanced security with 0600 SHM permissions

### Fixed
- Shared memory handshake race conditions
- Protocol version negotiation
- Connection pool reuse rate >95%
- Memory leaks in buffer management

### Security
- PII redaction in logs
- Authentication token support
- IP allowlist configuration
- Rate limiting and DOS protection

## [1.0.0] - TBD

### Initial Production Release
- Complete IPC system with shared memory transport
- ≥1M msg/s throughput, ≤10µs p99 latency
- Production-ready with comprehensive testing
- Full observability and monitoring

## Version History

### 0.9.0 (Pre-release)
- Protocol stabilization
- Performance optimization
- Security hardening

### 0.8.0 (Beta)
- Shared memory implementation
- Connection pooling
- Basic metrics

### 0.7.0 (Alpha)
- Initial IPC framework
- Binary codec
- Basic server implementation
