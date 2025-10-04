#!/bin/bash

echo "=== VERIFYING ALL 62 LANGUAGES ==="
echo ""

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter

echo "Counting language dependencies in Cargo.toml..."

# Count crates.io dependencies (excluding tree-sitter itself and tree-sitter-highlight)
crates_count=$(cat Cargo.toml | grep '^tree-sitter-[a-z]' | grep ' = "' | wc -l)
echo "Crates.io parsers: $crates_count"

# Count path dependencies
path_count=$(cat Cargo.toml | grep '^tree-sitter-[a-z]' | grep ' = { path' | wc -l)
echo "Path dependencies: $path_count"

total=$((crates_count + path_count))
echo "TOTAL: $total languages"

echo ""
echo "=== LIST OF ALL LANGUAGES ==="

echo "CRATES.IO LANGUAGES:"
cat Cargo.toml | grep '^tree-sitter-[a-z]' | grep ' = "' | sed 's/ =.*//' | sed 's/tree-sitter-/  - /' | sort

echo ""
echo "PATH DEPENDENCIES:"
cat Cargo.toml | grep '^tree-sitter-[a-z]' | grep ' = { path' | sed 's/ =.*//' | sed 's/tree-sitter-/  - /' | sort

echo ""
if [ "$total" -ge 62 ]; then
    echo "✅ SUCCESS: Have $total languages (target: 62+)"
else
    echo "❌ MISSING: Have only $total languages, need 62+"
fi
