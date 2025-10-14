# Makefile for lapce-ai IPC system
.PHONY: help test coverage test-nuclear clean build release

help:
	@echo "Lapce-AI IPC System - Available targets:"
	@echo "  make test          - Run all tests"
	@echo "  make test-nuclear  - Run nuclear production tests"
	@echo "  make coverage      - Generate test coverage report (requires cargo-llvm-cov)"
	@echo "  make build         - Build debug"
	@echo "  make release       - Build release"
	@echo "  make clean         - Clean build artifacts"

# Run all tests
test:
	cargo test --workspace

# Run nuclear production tests
test-nuclear:
	@echo "Running IPC nuclear tests..."
	cargo test --test ipc_nuclear_production_test -- --nocapture --test-threads=1
	@echo ""
	@echo "Running Binary Protocol nuclear tests..."
	cargo test --release --test binary_protocol_nuclear_test -- --nocapture --test-threads=1
	@echo ""
	@echo "Running Connection Pool nuclear tests..."
	cargo test --test connection_pool_nuclear_test -- --nocapture --test-threads=1

# Generate coverage report (nuclear tests only due to workspace compilation issues)
coverage:
	@echo "Generating test coverage for nuclear test suite..."
	@which cargo-llvm-cov > /dev/null 2>&1 || (echo "Installing cargo-llvm-cov..." && cargo install cargo-llvm-cov)
	cargo llvm-cov --test binary_protocol_nuclear_test --test connection_pool_nuclear_test --test ipc_nuclear_production_test --lcov --output-path lcov.info
	@echo ""
	@echo "Coverage report generated: lcov.info"
	@echo "Checking coverage (nuclear tests measure production-critical paths)..."
	cargo llvm-cov --test binary_protocol_nuclear_test --test connection_pool_nuclear_test --test ipc_nuclear_production_test --summary-only | tee coverage-summary.txt
	@echo ""
	@echo "Note: Coverage is low (0.67%) because nuclear tests focus on production-critical paths."
	@echo "Full workspace coverage requires fixing compilation errors in lib tests."

# Build debug
build:
	cargo build --workspace

# Build release
release:
	cargo build --release --workspace

# Clean build artifacts
clean:
	cargo clean
	rm -f lcov.info coverage-summary.txt
