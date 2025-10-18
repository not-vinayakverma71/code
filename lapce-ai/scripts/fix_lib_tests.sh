#!/bin/bash
# Systematic fix for lib test compilation errors
# Focus: Fix async/await issues in tests

set -e

echo "Fixing lib test compilation errors..."
echo ""

# Get list of files with async/await errors
echo "1. Finding files with missing .await..."
cargo test --lib 2>&1 | grep "no method named \`unwrap\` found for opaque type.*Future" -B 2 | grep "src/" | cut -d':' -f1 | sort | uniq > /tmp/await_files.txt

FILE_COUNT=$(wc -l < /tmp/await_files.txt)
echo "   Found $FILE_COUNT files with async/await issues"

# For now, just show the files that need fixing
echo ""
echo "Files needing .await fixes:"
cat /tmp/await_files.txt

echo ""
echo "Run: cargo test --lib 2>&1 | grep 'error\[E' | sort | uniq -c"
