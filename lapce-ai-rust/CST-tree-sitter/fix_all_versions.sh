#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/external-grammars

echo "Fixing ALL tree-sitter versions to exactly 0.24..."
echo "=================================================="

for dir in tree-sitter-*/; do
    if [ -f "$dir/Cargo.toml" ]; then
        echo "Fixing $dir"
        # Replace all tree-sitter versions with exactly 0.24
        sed -i 's/tree-sitter = ".*"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
        sed -i 's/tree-sitter = "^.*"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
        sed -i 's/tree-sitter = "~.*"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
        sed -i 's/tree-sitter = ">=.*"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
        sed -i 's/tree-sitter-language = ".*"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
    fi
done

echo "Done! All parsers now use tree-sitter 0.24"
