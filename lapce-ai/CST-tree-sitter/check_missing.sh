#!/bin/bash

# Languages marked with ❌ that need to be checked
missing_languages=(
    "sql"
    "kotlin"
    "matlab"
    "r"
    "perl"
    "vb" # Visual Basic
    "fortran"
    "sas"
    "abap"
    "dart"
    "julia"
    "haskell"
    "clojure"
    "nim"
    "crystal"
    "zig"
    "ada"
    "prolog"
    "racket"
    "vhdl"
    "yaml"
    "xml"
    "graphql"
    "gradle"
    "vim"
)

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/external-grammars

echo "Checking missing languages..."
for lang in "${missing_languages[@]}"; do
    found=false
    for dir in tree-sitter-*/; do
        dirname=$(basename "$dir")
        if [[ "$dirname" == *"$lang"* ]]; then
            echo "✅ $lang -> found as $dirname"
            found=true
            break
        fi
    done
    if [ "$found" = false ]; then
        echo "❌ $lang -> MISSING"
    fi
done
