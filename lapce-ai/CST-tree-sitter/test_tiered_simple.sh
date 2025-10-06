#!/bin/bash

echo "=== Simple Tiered Storage Test ==="
echo ""

# Build
echo "Building..."
cd /home/verma/lapce/lapce-ai/CST-tree-sitter
cargo build --release --bin benchmark_tiered_storage 2>&1 | tail -3

# Run with shorter timeout
echo ""
echo "Running test (30 second timeout)..."
timeout 30 ./target/release/benchmark_tiered_storage 2>&1

# Check exit code
EXIT_CODE=$?
if [ $EXIT_CODE -eq 124 ]; then
    echo ""
    echo "❌ Test timed out after 30 seconds"
elif [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "✅ Test completed successfully"
else
    echo ""
    echo "⚠️ Test exited with code: $EXIT_CODE"
fi

# Show any results file
if [ -f "TIERED_STORAGE_RESULTS.json" ]; then
    echo ""
    echo "Results:"
    cat TIERED_STORAGE_RESULTS.json | jq '.'
fi
