#!/bin/bash

echo "Regenerating all parsers with tree-sitter CLI 0.20.8..."
echo "======================================================="

# Check tree-sitter CLI version
echo "Tree-sitter CLI version:"
npx tree-sitter --version

# List of grammars that need regeneration
GRAMMARS_TO_REGENERATE=(
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
    "tree-sitter-typescript"
)

# Function to regenerate a parser
regenerate_parser() {
    local dir=$1
    local grammar_name=$(basename $dir)
    
    echo ""
    echo "Processing $grammar_name..."
    
    if [ ! -d "$dir" ]; then
        echo "  Directory not found: $dir"
        return 1
    fi
    
    cd "$dir"
    
    # Find grammar file
    if [ -f "grammar.js" ]; then
        echo "  Found grammar.js"
        # Backup existing parser
        if [ -f "src/parser.c" ]; then
            cp src/parser.c src/parser.c.bak 2>/dev/null
        fi
        
        # Generate new parser
        echo "  Generating parser..."
        npx tree-sitter generate
        
        if [ $? -eq 0 ]; then
            echo "  ✅ Successfully regenerated parser"
        else
            echo "  ❌ Failed to regenerate parser"
            # Restore backup if generation failed
            if [ -f "src/parser.c.bak" ]; then
                mv src/parser.c.bak src/parser.c
            fi
        fi
    elif [ -f "grammar.ts" ]; then
        echo "  Found grammar.ts"
        # TypeScript grammar needs compilation first
        echo "  Compiling TypeScript grammar..."
        npm install 2>/dev/null
        npm run build 2>/dev/null || npx tsc 2>/dev/null
        
        echo "  Generating parser..."
        npx tree-sitter generate
        
        if [ $? -eq 0 ]; then
            echo "  ✅ Successfully regenerated parser"
        else
            echo "  ❌ Failed to regenerate parser"
        fi
    else
        echo "  ❌ No grammar file found"
    fi
    
    cd - > /dev/null
}

# Regenerate each parser
for grammar in "${GRAMMARS_TO_REGENERATE[@]}"; do
    regenerate_parser "external-grammars/$grammar"
done

echo ""
echo "======================================================="
echo "Parser regeneration complete!"
echo ""
echo "Now updating Cargo.toml files to ensure tree-sitter 0.23.0..."

# Update all Cargo.toml files to use tree-sitter 0.23.0
for grammar_dir in external-grammars/tree-sitter-*/; do
    cargo_file="$grammar_dir/Cargo.toml"
    if [ -f "$cargo_file" ]; then
        sed -i 's/tree-sitter = ".*"/tree-sitter = "0.23.0"/g' "$cargo_file"
        sed -i 's/tree-sitter-language = ".*"/tree-sitter = "0.23.0"/g' "$cargo_file"
        echo "Updated $(basename $grammar_dir)/Cargo.toml"
    fi
done

echo ""
echo "All updates complete!"
