#!/bin/bash

echo "Fixing all references to tree_sitter_elm::LANGUAGE and tree_sitter_systemverilog::LANGUAGE"

# Fix all references to add () for function calls
find src -name "*.rs" -type f | while read file; do
    # Fix elm references
    sed -i 's/tree_sitter_elm::LANGUAGE\.into()/tree_sitter_elm::LANGUAGE().into()/g' "$file"
    sed -i 's/tree_sitter_elm::LANGUAGE\([^(]\)/tree_sitter_elm::LANGUAGE()\1/g' "$file"
    
    # Fix systemverilog references  
    sed -i 's/tree_sitter_systemverilog::LANGUAGE\.into()/tree_sitter_systemverilog::LANGUAGE().into()/g' "$file"
    sed -i 's/tree_sitter_systemverilog::LANGUAGE\([^(]\)/tree_sitter_systemverilog::LANGUAGE()\1/g' "$file"
done

echo "Fixed all references!"
