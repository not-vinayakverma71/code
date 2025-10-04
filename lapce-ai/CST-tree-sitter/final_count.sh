#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter

echo "=== COUNTING ALL 64 LANGUAGES ==="
echo ""

# Count crates.io deps (not path deps)
crates=$(grep -E '^tree-sitter-[a-z-]+ = "' Cargo.toml | wc -l)
echo "Crates.io dependencies: $crates"

# Count path deps
paths=$(grep -E '^tree-sitter-[a-z-]+ = \{ path' Cargo.toml | wc -l)
echo "Path dependencies: $paths"

# Total
total=$((crates + paths))
echo ""
echo "TOTAL ACTIVE LANGUAGES: $total"

echo ""
echo "=== ALL LANGUAGES ==="
echo "Crates.io:"
grep -E '^tree-sitter-[a-z-]+ = "' Cargo.toml | sed 's/ =.*//' | sed 's/tree-sitter-/  /' | nl

echo ""
echo "Path deps:"
grep -E '^tree-sitter-[a-z-]+ = \{ path' Cargo.toml | sed 's/ =.*//' | sed 's/tree-sitter-/  /' | nl

if [ "$total" -ge 62 ]; then
    echo ""
    echo "✅ SUCCESS: $total languages active!"
else
    echo ""
    echo "❌ Only $total languages, need 62+"
fi
