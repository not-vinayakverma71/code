#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/external-grammars

echo "Checking Cargo.toml files for all parsers..."
echo "============================================"
echo ""

missing_count=0
has_cargo_count=0

for dir in tree-sitter-*/; do
    name="${dir%/}"
    if [ -f "$dir/Cargo.toml" ]; then
        echo "✅ $name"
        has_cargo_count=$((has_cargo_count + 1))
    else
        echo "❌ $name - MISSING Cargo.toml"
        missing_count=$((missing_count + 1))
        # Check for parser.c
        [ -f "$dir/src/parser.c" ] && echo "   Has parser.c" || echo "   No parser.c"
    fi
done

echo ""
echo "Summary:"
echo "  Has Cargo.toml: $has_cargo_count"
echo "  Missing Cargo.toml: $missing_count"
