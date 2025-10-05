#!/bin/bash

echo "Finding compatible versions of external grammars for tree-sitter 0.23..."
echo "==============================================================="

# List of grammars that need fixing
FAILED_GRAMMARS=(
    "tree-sitter-javascript"
    "tree-sitter-markdown" 
    "tree-sitter-matlab"
    "tree-sitter-sql"
    "tree-sitter-vim"
    "tree-sitter-solidity"
    "tree-sitter-fsharp"
    "tree-sitter-systemverilog"
    "tree-sitter-elm"
    "tree-sitter-hcl"
    "tree-sitter-xml"
    "tree-sitter-fortran"
    "tree-sitter-vhdl"
    "tree-sitter-scheme"
    "tree-sitter-fennel"
    "tree-sitter-gleam"
    "tree-sitter-astro"
    "tree-sitter-wgsl"
    "tree-sitter-glsl"
    "tree-sitter-tcl"
    "tree-sitter-cairo"
)

# Check each grammar for compatible versions
for grammar in "${FAILED_GRAMMARS[@]}"; do
    echo ""
    echo "Checking $grammar..."
    
    # Get the current git repository info if it exists
    if [ -d "external-grammars/$grammar/.git" ]; then
        cd "external-grammars/$grammar"
        
        # Get current commit
        current_commit=$(git rev-parse --short HEAD 2>/dev/null)
        echo "  Current commit: $current_commit"
        
        # Look for tags that might be compatible with tree-sitter 0.23
        echo "  Available tags:"
        git tag -l | tail -10
        
        # Check if there's a v0.20.x or v0.19.x tag (likely compatible with tree-sitter 0.23)
        compatible_tags=$(git tag -l "v0.19.*" "v0.20.*" "v0.21.*" "v0.22.*" | tail -5)
        if [ ! -z "$compatible_tags" ]; then
            echo "  Potentially compatible tags: $compatible_tags"
        fi
        
        cd ../..
    else
        echo "  No git repository found"
    fi
done

echo ""
echo "==============================================================="
echo "Checking crates.io for older versions..."
echo ""

# Check crates.io for older versions
for grammar in "${FAILED_GRAMMARS[@]}"; do
    crate_name=${grammar/tree-sitter-/tree-sitter-}
    echo "Checking $crate_name on crates.io..."
    cargo search "$crate_name" --limit 1 2>/dev/null | head -2
done
