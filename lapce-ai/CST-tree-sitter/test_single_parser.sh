#!/bin/bash

lang=$1
cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/external-grammars/$lang

echo "Testing $lang..."
echo "================="

# Check if Cargo.toml exists
if [ ! -f "Cargo.toml" ]; then
    echo "❌ No Cargo.toml found"
    exit 1
fi

# Show current tree-sitter version
echo "Tree-sitter version in Cargo.toml:"
grep "tree-sitter" Cargo.toml | grep -v "#"

echo ""
echo "Building..."
cargo build 2>&1 | tee /tmp/${lang}_build.log

if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo ""
    echo "✅ $lang builds successfully!"
else
    echo ""
    echo "❌ $lang build failed"
    echo ""
    echo "Error details:"
    grep -E "(error|failed)" /tmp/${lang}_build.log
fi
