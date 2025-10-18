#!/bin/bash

# Fix all tree-sitter version conflicts to use 0.23.0 consistently

find /home/verma/lapce/lapce-ai/CST-tree-sitter -name "Cargo.toml" -type f | while read -r file; do
    # Replace all tree-sitter versions with 0.23.0
    sed -i 's/^tree-sitter = .*/tree-sitter = "0.23.0"/' "$file"
    sed -i 's/^tree-sitter-language = .*/tree-sitter-language = "0.1"/' "$file"
done

echo "Fixed all tree-sitter versions to 0.23.0"
