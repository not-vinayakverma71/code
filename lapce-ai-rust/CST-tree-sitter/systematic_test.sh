#!/bin/bash

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

echo "SYSTEMATIC COMPILATION TEST - 21 Languages"
echo "==========================================="
echo ""

success=0
failed=0
failed_list=""

for lang in "${languages[@]}"; do
    printf "%-25s: " "$lang"
    
    if [ ! -d "$lang" ]; then
        echo "❌ Directory not found"
        failed=$((failed + 1))
        failed_list="$failed_list\n  - $lang (missing)"
        continue
    fi
    
    cd "$lang"
    
    # Quick build test
    cargo build 2>&1 > /tmp/${lang}_build.log
    result=$?
    
    if [ $result -eq 0 ]; then
        echo "✅ SUCCESS"
        success=$((success + 1))
    else
        echo "❌ FAILED"
        failed=$((failed + 1))
        failed_list="$failed_list\n  - $lang"
        # Extract first error
        grep -m1 "error" /tmp/${lang}_build.log | sed 's/^/      /'
    fi
    
    cd ..
done

echo ""
echo "==========================================="
echo "RESULTS:"
echo "  ✅ Successful: $success/21"
echo "  ❌ Failed: $failed/21"

if [ $failed -gt 0 ]; then
    echo ""
    echo "Failed languages:"
    echo -e "$failed_list"
fi

echo ""
echo "Next step: Fix compilation errors in failed languages"
