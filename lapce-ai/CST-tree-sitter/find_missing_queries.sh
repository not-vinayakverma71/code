#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/queries

# Our 67 active languages
langs=(
    "rust" "javascript" "typescript" "python" "go" "java" "c" "cpp" 
    "c-sharp" "ruby" "php" "lua" "bash" "css" "json" "swift" "scala"
    "elixir" "html" "elm" "toml" "ocaml" "nix" "latex" "make" "cmake"
    "verilog" "erlang" "d" "dockerfile" "pascal" "commonlisp" "prisma"
    "hlsl" "objc" "cobol" "groovy" "hcl" "solidity" "fsharp" "powershell"
    "systemverilog" "embedded-template" "kotlin" "yaml" "r" "matlab" "perl"
    "dart" "julia" "haskell" "graphql" "sql" "zig" "vim" "abap" "nim"
    "clojure" "crystal" "fortran" "vhdl" "racket" "ada" "prolog" "gradle" "xml"
)

echo "=== CHECKING QUERY FILES FOR 67 LANGUAGES ==="
echo ""

present=0
missing=0
missing_langs=()

for lang in "${langs[@]}"; do
    # Check if .scm file exists
    if [ -f "${lang}.scm" ]; then
        present=$((present + 1))
        echo "✓ $lang (file)"
    # Check if directory exists
    elif [ -d "$lang" ]; then
        present=$((present + 1))
        echo "✓ $lang (dir)"
    # Check alternate naming
    elif [ -f "${lang//-/_}.scm" ]; then
        present=$((present + 1))
        echo "✓ $lang (alternate)"
    elif [ -d "${lang//-/_}" ]; then
        present=$((present + 1))
        echo "✓ $lang (alternate dir)"
    else
        missing=$((missing + 1))
        missing_langs+=("$lang")
        echo "✗ $lang MISSING"
    fi
done

echo ""
echo "=== SUMMARY ==="
echo "Present: $present/67"
echo "Missing: $missing/67"

echo ""
echo "=== MISSING LANGUAGES ==="
for lang in "${missing_langs[@]}"; do
    echo "  - $lang"
done
