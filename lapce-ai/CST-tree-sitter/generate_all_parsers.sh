#!/bin/bash
# Generate parser.c for all grammars that need it

cd external-grammars

# Install tree-sitter CLI locally
npm install tree-sitter-cli

export PATH="./node_modules/.bin:$PATH"

for dir in tree-sitter-powershell tree-sitter-toml; do
    if [ -d "$dir" ] && [ ! -f "$dir/src/parser.c" ]; then
        echo "Generating parser for $dir..."
        cd "$dir"
        
        if [ -f "grammar.js" ]; then
            npx tree-sitter generate || echo "Failed to generate parser for $dir"
        fi
        
        cd ..
    fi
done

echo "✅ Parser generation complete"
