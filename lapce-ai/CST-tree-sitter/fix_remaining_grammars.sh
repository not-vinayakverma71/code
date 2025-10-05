#!/bin/bash

echo "Fixing remaining grammars with version conflicts..."
echo "===================================================="

# Function to clean and rebuild a grammar
rebuild_grammar() {
    local dir=$1
    local name=$(basename $dir)
    
    echo "Rebuilding $name..."
    cd "$dir"
    
    # Clean build artifacts
    rm -rf target/ 2>/dev/null
    rm -rf node_modules/ 2>/dev/null
    
    # If parser.c exists, check its version
    if [ -f "src/parser.c" ]; then
        version=$(grep -oP 'TREE_SITTER_LANGUAGE_VERSION \K\d+' src/parser.c 2>/dev/null | head -1)
        echo "  Current parser version: $version (need 14 for tree-sitter 0.23)"
        
        if [ "$version" = "15" ]; then
            echo "  Parser needs regeneration..."
            
            # Try to regenerate with tree-sitter CLI
            if [ -f "grammar.js" ]; then
                echo "  Regenerating from grammar.js..."
                npx tree-sitter generate 2>/dev/null
                
                # Check if regeneration worked
                new_version=$(grep -oP 'TREE_SITTER_LANGUAGE_VERSION \K\d+' src/parser.c 2>/dev/null | head -1)
                echo "  New parser version: $new_version"
            fi
        fi
    fi
    
    cd - > /dev/null
}

# Fix JavaScript - it's still using version 15
echo "1. Fixing JavaScript..."
rebuild_grammar "external-grammars/tree-sitter-javascript"

# Fix markdown
echo ""
echo "2. Fixing Markdown..."
rebuild_grammar "external-grammars/tree-sitter-markdown"

# Fix F#
echo ""
echo "3. Fixing F#..."
# F# needs special handling as it has multiple languages
cd external-grammars/tree-sitter-fsharp
if [ -f "fsharp/src/parser.c" ]; then
    echo "  Found F# parser"
    version=$(grep -oP 'TREE_SITTER_LANGUAGE_VERSION \K\d+' fsharp/src/parser.c 2>/dev/null | head -1)
    echo "  Current version: $version"
fi
cd ../..

# Fix SystemVerilog
echo ""
echo "4. Fixing SystemVerilog..."
rebuild_grammar "external-grammars/tree-sitter-systemverilog"

# Fix Elm
echo ""
echo "5. Fixing Elm..."
rebuild_grammar "external-grammars/tree-sitter-elm"

# Fix XML - needs special handling as it might not have grammar.js
echo ""
echo "6. Fixing XML..."
cd external-grammars/tree-sitter-xml
if [ ! -f "grammar.js" ]; then
    echo "  No grammar.js found"
    # XML might be in a subdirectory
    if [ -f "xml/grammar.js" ]; then
        echo "  Found xml/grammar.js"
        cd xml
        npx tree-sitter generate 2>/dev/null
        cd ..
    fi
fi
cd ../..

# Fix external grammars (scheme, fennel, gleam, etc.)
echo ""
echo "7. Enabling external grammars..."
# These need to be added to the SupportedLanguage enum implementation

echo ""
echo "===================================================="
echo "Checking parser versions after fixes..."

for grammar in tree-sitter-javascript tree-sitter-markdown tree-sitter-fsharp tree-sitter-systemverilog tree-sitter-elm tree-sitter-xml; do
    parser_file="external-grammars/$grammar/src/parser.c"
    if [ -f "$parser_file" ]; then
        version=$(grep -oP 'TREE_SITTER_LANGUAGE_VERSION \K\d+' "$parser_file" 2>/dev/null | head -1)
        echo "$grammar: version $version"
    else
        # Check alternative locations
        for alt in "$grammar/src/parser.c" "external-grammars/$grammar/*/src/parser.c"; do
            if [ -f "$alt" ]; then
                version=$(grep -oP 'TREE_SITTER_LANGUAGE_VERSION \K\d+' "$alt" 2>/dev/null | head -1)
                echo "$grammar: version $version (from $(dirname $alt))"
                break
            fi
        done
    fi
done
