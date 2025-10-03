#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/external-grammars

echo "Analyzing tree-sitter version requirements..."
echo "=============================================="
echo ""

for dir in tree-sitter-*/; do
    name="${dir%/}"
    lang="${name#tree-sitter-}"
    
    # Check Cargo.toml
    if [ -f "$dir/Cargo.toml" ]; then
        version=$(grep -E "tree-sitter.*=" "$dir/Cargo.toml" | grep -v "#" | head -1)
        echo "[$lang]"
        echo "  Cargo.toml: $version"
    elif [ -f "$dir/bindings/rust/Cargo.toml" ]; then
        version=$(grep -E "tree-sitter.*=" "$dir/bindings/rust/Cargo.toml" | grep -v "#" | head -1)
        echo "[$lang]"
        echo "  Rust binding: $version"
    else
        echo "[$lang]"
        echo "  No Cargo.toml found"
    fi
    
    # Check if parser.c exists
    if [ -f "$dir/src/parser.c" ]; then
        echo "  parser.c: ✅"
    else
        echo "  parser.c: ❌"
    fi
    
    # Check scanner
    if [ -f "$dir/src/scanner.c" ]; then
        echo "  scanner: C ✅"
    elif [ -f "$dir/src/scanner.cc" ]; then
        echo "  scanner: C++ ⚠️"
    else
        echo "  scanner: None"
    fi
    
    echo ""
done
