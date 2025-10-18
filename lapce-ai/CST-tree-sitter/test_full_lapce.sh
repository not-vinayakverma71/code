#!/bin/bash

echo "=== Full Lapce Codebase Test ==="
echo ""
echo "Counting all files in /home/verma/lapce..."
TOTAL_FILES=$(find /home/verma/lapce -type f | wc -l)
echo "Total files: $TOTAL_FILES"

echo ""
echo "Counting by extension..."
find /home/verma/lapce -type f -name "*.*" | sed 's/.*\.//' | sort | uniq -c | sort -rn | head -20

echo ""
echo "Running full benchmark with increased initial capacity..."
cd /home/verma/lapce/lapce-ai/CST-tree-sitter

# Set environment for better debugging
export RUST_LOG=debug
export RUST_BACKTRACE=1

# Run with time to track resource usage
/usr/bin/time -v ./target/release/benchmark_lapce_full 2>&1 | tee full_benchmark_output.log

echo ""
echo "Benchmark complete. Check full_benchmark_output.log for details."
