#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/queries

# Our 67 active languages
declare -A langs=(
    ["rust"]=1 ["javascript"]=1 ["typescript"]=1 ["python"]=1 ["go"]=1 
    ["java"]=1 ["c"]=1 ["cpp"]=1 ["c-sharp"]=1 ["ruby"]=1 ["php"]=1 
    ["lua"]=1 ["bash"]=1 ["css"]=1 ["json"]=1 ["swift"]=1 ["scala"]=1
    ["elixir"]=1 ["html"]=1 ["elm"]=1 ["toml"]=1 ["ocaml"]=1 ["nix"]=1 
    ["latex"]=1 ["make"]=1 ["cmake"]=1 ["verilog"]=1 ["erlang"]=1 ["d"]=1 
    ["dockerfile"]=1 ["pascal"]=1 ["commonlisp"]=1 ["prisma"]=1 ["hlsl"]=1 
    ["objc"]=1 ["cobol"]=1 ["groovy"]=1 ["hcl"]=1 ["solidity"]=1 ["fsharp"]=1 
    ["powershell"]=1 ["systemverilog"]=1 ["embedded-template"]=1 ["kotlin"]=1 
    ["yaml"]=1 ["r"]=1 ["matlab"]=1 ["perl"]=1 ["dart"]=1 ["julia"]=1 
    ["haskell"]=1 ["graphql"]=1 ["sql"]=1 ["zig"]=1 ["vim"]=1 ["abap"]=1 
    ["nim"]=1 ["clojure"]=1 ["crystal"]=1 ["fortran"]=1 ["vhdl"]=1 ["racket"]=1 
    ["ada"]=1 ["prolog"]=1 ["gradle"]=1 ["xml"]=1
)

echo "=== CHECKING 67 LANGUAGES ===" 
echo ""

present=0
missing=0
missing_list=()

for lang in "${!langs[@]}"; do
    # Normalize name
    norm_lang="${lang//-/_}"
    
    # Check multiple variants
    if [ -f "${lang}.scm" ] || [ -f "${norm_lang}.scm" ] || \
       [ -d "$lang" ] || [ -d "$norm_lang" ] || \
       [ -d "csharp" ] && [ "$lang" = "c-sharp" ]; then
        present=$((present + 1))
        echo "✓ $lang"
    else
        missing=$((missing + 1))
        missing_list+=("$lang")
        echo "✗ $lang"
    fi
done

echo ""
echo "=== SUMMARY ==="
echo "Present: $present/67"
echo "Missing: $missing/67"

if [ $missing -gt 0 ]; then
    echo ""
    echo "=== MISSING LANGUAGES ==="
    printf '%s\n' "${missing_list[@]}" | sort
fi
