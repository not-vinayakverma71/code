#!/bin/bash
# Test determinism: Run pipeline twice and compare outputs

echo "=== DETERMINISM TEST ==="
echo "Running pipeline twice on identical input..."

# Test file
TEST_FILE="/home/verma/lapce/Codex/src/main.rs"

# Run 1
./target/release/benchmark_codex_complete > run1.txt 2>&1
cp CODEX_COMPLETE_PHASE4.json run1.json

# Run 2  
./target/release/benchmark_codex_complete > run2.txt 2>&1
cp CODEX_COMPLETE_PHASE4.json run2.json

# Compare outputs
echo ""
echo "Comparing outputs..."
if diff -q run1.json run2.json > /dev/null; then
    echo "✅ PASS: Outputs are identical (deterministic)"
else
    echo "❌ FAIL: Outputs differ (non-deterministic)"
    diff run1.json run2.json | head -20
fi

# Check metrics consistency
echo ""
echo "Run 1 metrics:"
grep "Lines per MB" run1.txt
grep "Files processed" run1.txt

echo ""
echo "Run 2 metrics:"
grep "Lines per MB" run2.txt
grep "Files processed" run2.txt

# Cleanup
rm -f run1.txt run2.txt run1.json run2.json
