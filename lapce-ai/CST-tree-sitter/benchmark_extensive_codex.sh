#!/bin/bash
# Extensive benchmark of Codex with all validation tests

echo "=== EXTENSIVE CODEX BENCHMARK ==="
echo "Date: $(date)"
echo "Target: /home/verma/lapce/Codex"
echo ""

# Check initial system state
echo "=== SYSTEM STATE ==="
echo "Memory before:"
free -h | head -3
echo ""

# Count files
echo "=== CODEX STATISTICS ==="
echo "Total files: $(find /home/verma/lapce/Codex -type f | wc -l)"
echo "Source files: $(find /home/verma/lapce/Codex -type f \( -name "*.rs" -o -name "*.js" -o -name "*.ts" -o -name "*.py" \) | wc -l)"
echo "Total lines: $(find /home/verma/lapce/Codex -type f \( -name "*.rs" -o -name "*.js" -o -name "*.ts" -o -name "*.py" \) -exec wc -l {} \; | awk '{sum+=$1} END {print sum}')"
echo ""

# Run main benchmark
echo "=== RUNNING CODEX BENCHMARK ==="
time ./target/release/benchmark_codex_complete

# Check memory after
echo ""
echo "=== POST-BENCHMARK STATE ==="
echo "Memory after:"
free -h | head -3

# Run stress test
echo ""
echo "=== MEMORY STRESS TEST ==="
for i in {1..10}; do
    echo -n "Iteration $i: "
    ./target/release/benchmark_codex_complete 2>&1 | grep "RSS:" | tail -1
    sleep 1
done

# Test multi-tier if available
if [ -f "./target/release/test_multi_tier" ]; then
    echo ""
    echo "=== MULTI-TIER TEST ==="
    timeout 30 ./target/release/test_multi_tier 2>&1 | grep -E "(Hot|Warm|Cold|Frozen|SUCCESS)"
fi

# Parse final JSON report
echo ""
echo "=== FINAL METRICS FROM JSON ==="
if [ -f "CODEX_COMPLETE_PHASE4.json" ]; then
    echo "Lines per MB: $(jq '.lines_per_mb' CODEX_COMPLETE_PHASE4.json)"
    echo "Files processed: $(jq '.files_processed' CODEX_COMPLETE_PHASE4.json)"
    echo "Total lines: $(jq '.total_lines' CODEX_COMPLETE_PHASE4.json)"
    echo "Parse time: $(jq '.parse_time_seconds' CODEX_COMPLETE_PHASE4.json) seconds"
    echo "Memory efficiency: $(jq '.memory.efficiency' CODEX_COMPLETE_PHASE4.json)"
fi

echo ""
echo "=== BENCHMARK COMPLETE ===
