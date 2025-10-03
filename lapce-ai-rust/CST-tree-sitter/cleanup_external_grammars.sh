#!/bin/bash

# Languages to KEEP (25 ❌ languages from document)
KEEP_LANGUAGES=(
    "tree-sitter-sql"
    "tree-sitter-kotlin"
    "tree-sitter-matlab"
    "tree-sitter-r"
    "tree-sitter-perl"
    "tree-sitter-vb"
    "tree-sitter-fortran"
    "tree-sitter-sas"
    "tree-sitter-abap"
    "tree-sitter-dart"
    "tree-sitter-vhdl"
    "tree-sitter-julia"
    "tree-sitter-haskell"
    "tree-sitter-clojure"
    "tree-sitter-nim"
    "tree-sitter-crystal"
    "tree-sitter-zig"
    "tree-sitter-ada"
    "tree-sitter-prolog"
    "tree-sitter-racket"
    "tree-sitter-yaml"
    "tree-sitter-xml"
    "tree-sitter-graphql"
    "tree-sitter-gradle"
    "tree-sitter-vim"
)

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/external-grammars

echo "Starting cleanup - keeping only 25 ❌ languages..."

# Get all directories
for dir in tree-sitter-*/; do
    dirname="${dir%/}"
    
    # Check if this directory should be kept
    keep=false
    for lang in "${KEEP_LANGUAGES[@]}"; do
        if [ "$dirname" = "$lang" ]; then
            keep=true
            break
        fi
    done
    
    # If not in keep list, trash it
    if [ "$keep" = false ]; then
        echo "Trashing: $dirname"
        trash-put "$dirname"
    else
        echo "Keeping: $dirname"
    fi
done

echo ""
echo "Cleanup complete!"
echo "Remaining directories:"
ls -d tree-sitter-*/ | wc -l
echo ""
