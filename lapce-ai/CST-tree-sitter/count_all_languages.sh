#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter

echo "=== COMPLETE LANGUAGE COUNT ==="
echo ""

echo "1. CRATES.IO DEPENDENCIES:"
grep -E "^tree-sitter-[a-z-]+ = \"" Cargo.toml | sort | nl

echo ""
echo "2. PATH DEPENDENCIES:"
grep -E "^tree-sitter-[a-z-]+ = \{ path" Cargo.toml | sort | nl

echo ""
echo "=== SUMMARY ==="
crates_count=$(grep -E "^tree-sitter-[a-z-]+ = \"" Cargo.toml | wc -l)
path_count=$(grep -E "^tree-sitter-[a-z-]+ = \{ path" Cargo.toml | wc -l)
total=$((crates_count + path_count))

echo "Crates.io dependencies: $crates_count"
echo "Path dependencies: $path_count"
echo "TOTAL LANGUAGES: $total"

echo ""
echo "=== ALL LANGUAGES LIST ==="
(grep -E "^tree-sitter-[a-z-]+ = \"" Cargo.toml | sed 's/ =.*//' | sed 's/tree-sitter-//'; 
 grep -E "^tree-sitter-[a-z-]+ = \{ path" Cargo.toml | sed 's/ =.*//' | sed 's/tree-sitter-//') | sort -u | nl
