#!/bin/bash
# Fix all external grammars to use tree-sitter 0.24.7

cd external-grammars

for dir in tree-sitter-*; do
    if [ -f "$dir/Cargo.toml" ]; then
        echo "Fixing $dir/Cargo.toml..."
        # Replace any tree-sitter version with 0.24.7
        sed -i 's/tree-sitter = .*/tree-sitter = "0.24.7"/' "$dir/Cargo.toml"
        sed -i 's/tree-sitter-language = .*/tree-sitter-language = "0.1.0"/' "$dir/Cargo.toml"
    fi
done

echo "✅ All tree-sitter versions fixed to 0.24.7"
