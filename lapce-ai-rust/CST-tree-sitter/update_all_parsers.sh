#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/external-grammars

echo "Phase 1: Updating all Cargo.toml files to tree-sitter 0.24..."
echo "=============================================================="

for dir in tree-sitter-*/; do
    name="${dir%/}"
    lang="${name#tree-sitter-}"
    
    echo "Processing $lang..."
    
    # Update Cargo.toml if exists
    if [ -f "$dir/Cargo.toml" ]; then
        sed -i 's/tree-sitter-language = "0.1"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
        sed -i 's/tree-sitter = "0.17"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
        sed -i 's/tree-sitter = "~0.20"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
        sed -i 's/tree-sitter = ">=0.21"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
        sed -i 's/tree-sitter = ">=0.21.0"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
        sed -i 's/tree-sitter = "0.22.6"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
        sed -i 's/tree-sitter = ">=0.22.5"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
        sed -i 's/tree-sitter = "~0.25"/tree-sitter = "0.24"/g' "$dir/Cargo.toml"
        echo "  ✅ Updated Cargo.toml"
    elif [ -f "$dir/bindings/rust/Cargo.toml" ]; then
        sed -i 's/tree-sitter-language = "0.1"/tree-sitter = "0.24"/g' "$dir/bindings/rust/Cargo.toml"
        sed -i 's/tree-sitter = "0.17"/tree-sitter = "0.24"/g' "$dir/bindings/rust/Cargo.toml"
        echo "  ✅ Updated bindings/rust/Cargo.toml"
    else
        echo "  ⚠️ No Cargo.toml found"
    fi
done

echo ""
echo "Phase 1 complete!"
