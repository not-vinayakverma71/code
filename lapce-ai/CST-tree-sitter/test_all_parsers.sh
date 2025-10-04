#!/bin/bash

# Test compilation of all 21 languages individually
cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/external-grammars

languages=(
    "tree-sitter-kotlin"
    "tree-sitter-yaml"
    "tree-sitter-r"
    "tree-sitter-matlab"
    "tree-sitter-perl"
    "tree-sitter-dart"
    "tree-sitter-julia"
    "tree-sitter-haskell"
    "tree-sitter-graphql"
    "tree-sitter-sql"
    "tree-sitter-zig"
    "tree-sitter-vim"
    "tree-sitter-abap"
    "tree-sitter-nim"
    "tree-sitter-clojure"
    "tree-sitter-crystal"
    "tree-sitter-fortran"
    "tree-sitter-vhdl"
    "tree-sitter-racket"
    "tree-sitter-ada"
    "tree-sitter-prolog"
)

echo "Testing compilation of 21 languages..."
echo "======================================="
echo ""

success_count=0
fail_count=0
failed_langs=""

for lang in "${languages[@]}"; do
    echo "Testing $lang..."
    cd "$lang" 2>/dev/null
    if [ $? -ne 0 ]; then
        echo "  ❌ Directory not found"
        fail_count=$((fail_count + 1))
        failed_langs="$failed_langs $lang(dir)"
        continue
    fi
    
    # Try to build
    cargo build --release 2>&1 | tail -5 > /tmp/build_log.txt
    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        echo "  ✅ Build successful"
        success_count=$((success_count + 1))
    else
        echo "  ❌ Build failed"
        fail_count=$((fail_count + 1))
        failed_langs="$failed_langs $lang"
        cat /tmp/build_log.txt | sed 's/^/    /'
    fi
    
    cd ..
    echo ""
done

echo "======================================="
echo "SUMMARY:"
echo "  Success: $success_count/21"
echo "  Failed: $fail_count/21"
if [ -n "$failed_langs" ]; then
    echo "  Failed languages:$failed_langs"
fi
echo ""
