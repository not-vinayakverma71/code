#!/bin/bash

# Fix all cc version conflicts to use cc = "1.2"

find /home/verma/lapce/lapce-ai/CST-tree-sitter -name "Cargo.toml" -type f | while read -r file; do
    # Replace all cc versions with 1.2
    sed -i 's/^cc = .*/cc = "1.2"/' "$file"
done

echo "Fixed all cc versions to 1.2"
