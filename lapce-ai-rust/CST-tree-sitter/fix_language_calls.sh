#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/src

echo "Fixing LANGUAGE references to language() calls..."
echo "================================================="

# List of languages to fix
languages=(
    "kotlin"
    "yaml"
    "r"
    "matlab"
    "perl"
    "dart"
    "julia"
    "haskell"
    "graphql"
    "sql"
    "zig"
    "vim"
    "abap"
    "nim"
    "clojure"
    "crystal"
    "fortran"
    "vhdl"
    "racket"
    "ada"
    "prolog"
)

for lang in "${languages[@]}"; do
    echo "Fixing tree_sitter_${lang}::LANGUAGE -> tree_sitter_${lang}::language()"
    find . -name "*.rs" -type f -exec sed -i "s/tree_sitter_${lang}::LANGUAGE/tree_sitter_${lang}::language()/g" {} \;
done

echo ""
echo "All LANGUAGE references fixed!"
