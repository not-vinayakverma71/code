#!/bin/bash

echo "Fixing external grammars by checking out compatible versions or regenerating parsers..."
echo "====================================================================================="

# Function to regenerate parser
regenerate_parser() {
    local dir=$1
    echo "  Regenerating parser for $(basename $dir)..."
    
    if [ -f "$dir/grammar.js" ]; then
        cd "$dir"
        # Try to generate parser with tree-sitter 0.23
        npx tree-sitter generate 2>/dev/null || tree-sitter generate 2>/dev/null || echo "    Failed to regenerate"
        cd - > /dev/null
    elif [ -f "$dir/grammar.ts" ]; then
        cd "$dir"
        npx tree-sitter generate 2>/dev/null || tree-sitter generate 2>/dev/null || echo "    Failed to regenerate"
        cd - > /dev/null
    else
        echo "    No grammar file found"
    fi
}

# Fix JavaScript - use older version or regenerate
echo "1. Fixing tree-sitter-javascript..."
cd external-grammars/tree-sitter-javascript
if [ -d .git ]; then
    # Try v0.20.1 which should be compatible with tree-sitter 0.23
    git checkout v0.20.1 2>/dev/null || git checkout v0.20.0 2>/dev/null || echo "  No compatible tag found"
fi
cd ../..
regenerate_parser "external-grammars/tree-sitter-javascript"

# Fix Markdown - try v0.2.3
echo ""
echo "2. Fixing tree-sitter-markdown..."
cd external-grammars/tree-sitter-markdown
if [ -d .git ]; then
    git checkout v0.2.3 2>/dev/null || echo "  No v0.2.3 tag"
fi
cd ../..

# Fix SQL - try v0.3.0
echo ""
echo "3. Fixing tree-sitter-sql..."
cd external-grammars/tree-sitter-sql
if [ -d .git ]; then
    git checkout v0.3.0 2>/dev/null || echo "  No v0.3.0 tag"
fi
cd ../..

# Fix Vim - try v0.4.0
echo ""
echo "4. Fixing tree-sitter-vim..."
cd external-grammars/tree-sitter-vim
if [ -d .git ]; then
    git checkout v0.4.0 2>/dev/null || echo "  No v0.4.0 tag"
fi
cd ../..

# Fix MATLAB - try v1.0.3
echo ""
echo "5. Fixing tree-sitter-matlab..."
cd external-grammars/tree-sitter-matlab
if [ -d .git ]; then
    git checkout v1.0.3 2>/dev/null || echo "  No v1.0.3 tag"
fi
cd ../..

# Fix Solidity - try v1.0.0 or earlier
echo ""
echo "6. Fixing tree-sitter-solidity..."
cd external-grammars/tree-sitter-solidity
if [ -d .git ]; then
    git checkout v1.0.0 2>/dev/null || git checkout v0.0.3 2>/dev/null || echo "  No compatible tag"
fi
cd ../..

# Fix F# - use v0.1.0
echo ""
echo "7. Fixing tree-sitter-fsharp..."
cd external-grammars/tree-sitter-fsharp
if [ -d .git ]; then
    git checkout v0.1.0 2>/dev/null || echo "  No v0.1.0 tag"
fi
cd ../..

# Install tree-sitter CLI if not available
echo ""
echo "Installing tree-sitter CLI version 0.20.8 (compatible with tree-sitter 0.23)..."
npm install -g tree-sitter-cli@0.20.8 2>/dev/null || echo "  Failed to install tree-sitter CLI"

# For grammars without git repos, we'll need to regenerate
echo ""
echo "Regenerating parsers for grammars without version control..."

for grammar_dir in external-grammars/tree-sitter-*/; do
    if [ ! -d "$grammar_dir/.git" ] && [ -f "$grammar_dir/grammar.js" -o -f "$grammar_dir/grammar.ts" ]; then
        echo "Regenerating $(basename $grammar_dir)..."
        regenerate_parser "$grammar_dir"
    fi
done

echo ""
echo "====================================================================================="
echo "Fixing complete! Now rebuilding..."
