#!/bin/bash

echo "Final fix for remaining 8 languages with version conflicts..."
echo "=============================================================="

# Function to download and use older parser.c
download_older_parser() {
    local grammar=$1
    local url=$2
    
    echo "Downloading older parser for $grammar..."
    cd "external-grammars/$grammar"
    
    if [ ! -z "$url" ]; then
        # Download older parser.c
        curl -L "$url" -o src/parser.c 2>/dev/null
        if [ $? -eq 0 ]; then
            echo "  ✅ Downloaded older parser"
        else
            echo "  ❌ Failed to download"
        fi
    fi
    
    cd ../..
}

# Check current versions
echo "Current parser versions:"
for grammar in javascript markdown fsharp systemverilog elm xml; do
    parser_file="external-grammars/tree-sitter-$grammar/src/parser.c"
    if [ -f "$parser_file" ]; then
        version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' "$parser_file" 2>/dev/null | head -1)
        echo "  tree-sitter-$grammar: version $version"
    fi
done

echo ""
echo "Attempting to fix by using pre-built parser.c files from tree-sitter 0.20.x era..."

# JavaScript - use npm to get older version
echo ""
echo "1. Fixing JavaScript..."
cd external-grammars/tree-sitter-javascript
npm install tree-sitter-cli@0.20.8 2>/dev/null
npx tree-sitter generate 2>/dev/null
cd ../..

# Markdown - special handling for dual parsers
echo ""
echo "2. Fixing Markdown..."
cd external-grammars/tree-sitter-markdown
# Markdown has two parsers
if [ -f "tree-sitter-markdown/grammar.js" ]; then
    cd tree-sitter-markdown
    npx tree-sitter generate 2>/dev/null
    cd ..
fi
if [ -f "tree-sitter-markdown-inline/grammar.js" ]; then
    cd tree-sitter-markdown-inline
    npx tree-sitter generate 2>/dev/null
    cd ..
fi
cd ../..

# F# - has two parsers
echo ""
echo "3. Fixing F#..."
cd external-grammars/tree-sitter-fsharp
if [ -f "fsharp/grammar.js" ]; then
    cd fsharp
    npx tree-sitter generate 2>/dev/null
    cd ..
fi
if [ -f "fsharp_signature/grammar.js" ]; then
    cd fsharp_signature
    npx tree-sitter generate 2>/dev/null
    cd ..
fi
cd ../..

# XML - check if it's a submodule issue
echo ""
echo "4. Fixing XML..."
cd external-grammars/tree-sitter-xml
if [ -f "xml/grammar.js" ]; then
    cd xml
    npx tree-sitter generate 2>/dev/null
    cd ..
fi
if [ -f "dtd/grammar.js" ]; then
    cd dtd
    npx tree-sitter generate 2>/dev/null
    cd ..
fi
cd ../..

# SystemVerilog - try to fix grammar.js issue
echo ""
echo "5. Fixing SystemVerilog..."
cd external-grammars/tree-sitter-systemverilog
# Fix the reserved function issue
if [ -f "grammar.js" ]; then
    # Add the reserved function if it's missing
    if ! grep -q "function reserved" grammar.js; then
        echo "function reserved(name, rule) { return rule; }" >> grammar.js.tmp
        cat grammar.js >> grammar.js.tmp
        mv grammar.js.tmp grammar.js
    fi
    npx tree-sitter generate 2>/dev/null
fi
cd ../..

# Elm
echo ""
echo "6. Fixing Elm..."
cd external-grammars/tree-sitter-elm
npx tree-sitter generate 2>/dev/null
cd ../..

echo ""
echo "=============================================================="
echo "Checking versions after fixes..."

for grammar in javascript markdown fsharp systemverilog elm xml; do
    parser_file="external-grammars/tree-sitter-$grammar/src/parser.c"
    if [ -f "$parser_file" ]; then
        version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' "$parser_file" 2>/dev/null | head -1)
        echo "  tree-sitter-$grammar: version $version"
    else
        # Check alternative locations
        for alt in "external-grammars/tree-sitter-$grammar/*/src/parser.c"; do
            if [ -f "$alt" ]; then
                version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' "$alt" 2>/dev/null | head -1)
                echo "  tree-sitter-$grammar: version $version (from $(dirname $(dirname $alt)))"
                break
            fi
        done
    fi
done

echo ""
echo "Done!"
