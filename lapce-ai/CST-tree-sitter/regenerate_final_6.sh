#!/bin/bash

echo "Regenerating parsers for final 6 languages with tree-sitter 0.20.8..."
echo "====================================================================="

cd external-grammars

# Install tree-sitter CLI 0.20.8 globally for this session
npm install -g tree-sitter-cli@0.20.8

echo "Using tree-sitter version:"
tree-sitter --version

# 1. JavaScript
echo ""
echo "1. Regenerating JavaScript parser..."
cd tree-sitter-javascript
tree-sitter generate
echo "JavaScript parser version: $(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c | head -1)"
cd ..

# 2. SystemVerilog - fix grammar first
echo ""
echo "2. Regenerating SystemVerilog parser..."
cd tree-sitter-systemverilog
# Add missing reserved function
if ! grep -q "function reserved" grammar.js; then
    cat > grammar_temp.js << 'EOF'
// Helper functions
function reserved(name, rule) {
    return rule;
}

EOF
    cat grammar.js >> grammar_temp.js
    mv grammar_temp.js grammar.js
fi
tree-sitter generate
echo "SystemVerilog parser version: $(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c | head -1)"
cd ..

# 3. Elm
echo ""
echo "3. Regenerating Elm parser..."
cd tree-sitter-elm
tree-sitter generate
echo "Elm parser version: $(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c | head -1)"
cd ..

# 4. XML - handle submodules
echo ""
echo "4. Regenerating XML parser..."
cd tree-sitter-xml
if [ -d "xml" ]; then
    cd xml
    tree-sitter generate
    echo "XML parser version: $(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c | head -1)"
    cd ..
fi
if [ -d "dtd" ]; then
    cd dtd
    tree-sitter generate
    echo "DTD parser version: $(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c | head -1)"
    cd ..
fi
cd ..

# 5. COBOL
echo ""
echo "5. Regenerating COBOL parser..."
cd tree-sitter-cobol
tree-sitter generate

# Fix scanner.c compilation issues
if [ -f "src/scanner.c" ]; then
    # Check if includes are missing and add them
    if ! grep -q "#include <stddef.h>" src/scanner.c; then
        cat > scanner_temp.c << 'EOF'
#include <stddef.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>
#include <stdio.h>

EOF
        cat src/scanner.c >> scanner_temp.c
        mv scanner_temp.c src/scanner.c
    fi
fi

echo "COBOL parser version: $(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c | head -1)"
cd ..

# JSX is handled by JavaScript, no separate parser needed

echo ""
echo "====================================================================="
echo "Final verification of all 6 languages:"
echo ""

for lang in javascript systemverilog elm xml cobol; do
    dir="tree-sitter-$lang"
    if [ -d "$dir" ]; then
        echo "✓ $dir"
        
        # Find parser.c
        parser_file=$(find "$dir" -name "parser.c" -type f | head -1)
        if [ ! -z "$parser_file" ]; then
            version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' "$parser_file" | head -1)
            if [ "$version" = "14" ]; then
                echo "  ✅ Version $version - Compatible with tree-sitter 0.23"
            else
                echo "  ⚠️  Version $version - May need adjustment"
            fi
        else
            echo "  ❌ No parser.c found"
        fi
    else
        echo "✗ $dir NOT FOUND"
    fi
done

cd ..
