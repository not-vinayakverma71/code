#!/bin/bash

echo "Checking for missing languages from 60 essential list..."
echo "=================================================="

# Languages we need to check for
missing_languages=(
    "tree-sitter-hcl"
    "tree-sitter-terraform" 
    "tree-sitter-solidity"
    "tree-sitter-fsharp"
    "tree-sitter-prolog"
    "tree-sitter-powershell"
    "tree-sitter-vhdl"
    "tree-sitter-systemverilog"
    "tree-sitter-cypher"
    "tree-sitter-sas"
    "tree-sitter-stata"
    "tree-sitter-vyper"
    "tree-sitter-cairo"
    "tree-sitter-abap"
    "tree-sitter-rpg"
    "tree-sitter-mumps"
    "tree-sitter-applescript"
    "tree-sitter-assembly"
    "tree-sitter-asm"
    "tree-sitter-simulink"
)

echo "Searching for available parsers..."
echo ""

available=0
unavailable=0

for lang in "${missing_languages[@]}"; do
    result=$(cargo search "$lang" 2>&1 | head -1)
    if [[ "$result" == *"$lang"* && "$result" == *"="* ]]; then
        echo "✅ FOUND: $result"
        ((available++))
    else
        echo "❌ NOT FOUND: $lang"
        ((unavailable++))
    fi
done

echo ""
echo "Summary:"
echo "  Available: $available"
echo "  Not available: $unavailable"
