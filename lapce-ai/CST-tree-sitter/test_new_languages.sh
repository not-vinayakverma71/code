#!/bin/bash

echo "Testing available tree-sitter language packages..."
echo "================================================="

# Test which packages are available
languages=(
    "tree-sitter-kotlin"
    "tree-sitter-yaml" 
    "tree-sitter-sql"
    "tree-sitter-graphql"
    "tree-sitter-dart"
    "tree-sitter-haskell"
    "tree-sitter-r"
    "tree-sitter-julia"
    "tree-sitter-clojure"
    "tree-sitter-zig"
    "tree-sitter-nix"
    "tree-sitter-perl"
    "tree-sitter-latex"
    "tree-sitter-make"
    "tree-sitter-cmake"
    "tree-sitter-verilog"
    "tree-sitter-erlang"
    "tree-sitter-d"
    "tree-sitter-nim"
    "tree-sitter-pascal"
    "tree-sitter-scheme"
    "tree-sitter-commonlisp"
    "tree-sitter-racket"
    "tree-sitter-fennel"
    "tree-sitter-vimdoc"
    "tree-sitter-regex"
    "tree-sitter-prisma"
    "tree-sitter-gleam"
    "tree-sitter-vue"
    "tree-sitter-astro"
    "tree-sitter-wgsl"
    "tree-sitter-glsl"
    "tree-sitter-hlsl"
)

for lang in "${languages[@]}"; do
    result=$(cargo search "$lang" 2>&1 | head -1)
    if [[ "$result" == *"$lang"* ]]; then
        echo "✅ $lang - Available: $result"
    else
        echo "❌ $lang - Not found"
    fi
done
