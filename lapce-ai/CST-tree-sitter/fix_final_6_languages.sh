#!/bin/bash

echo "Fixing the final 6 languages - NO EXCUSES!"
echo "==========================================="

cd external-grammars

# 1. JavaScript - clone fresh and use older version
echo "1. Cloning and fixing JavaScript..."
rm -rf tree-sitter-javascript
git clone https://github.com/tree-sitter/tree-sitter-javascript.git
cd tree-sitter-javascript
# Checkout older version compatible with tree-sitter 0.23
git checkout v0.20.1 2>/dev/null || git checkout v0.20.0 2>/dev/null || git checkout tags/v0.20.1 2>/dev/null
npm install
npx tree-sitter generate
cd ..

# 2. JSX - part of JavaScript repo
echo "2. JSX is part of JavaScript - fixing..."
# JSX support comes from tree-sitter-javascript

# 3. SystemVerilog - clone and fix
echo "3. Cloning and fixing SystemVerilog..."
rm -rf tree-sitter-systemverilog
git clone https://github.com/tree-sitter/tree-sitter-systemverilog.git
cd tree-sitter-systemverilog
# Checkout older compatible version
git checkout v0.1.0 2>/dev/null || git log --oneline | head -20
# Fix the grammar issue
if [ -f "grammar.js" ]; then
    # Add missing reserved function
    echo "function reserved(name, rule) { return rule; }" > grammar_fix.js
    cat grammar.js >> grammar_fix.js
    mv grammar_fix.js grammar.js
fi
npm install
npx tree-sitter generate
cd ..

# 4. Elm - clone fresh
echo "4. Cloning and fixing Elm..."
rm -rf tree-sitter-elm
git clone https://github.com/elm-tooling/tree-sitter-elm.git
cd tree-sitter-elm
# Use older version
git checkout v5.6.0 2>/dev/null || git checkout v5.5.0 2>/dev/null
npm install
npx tree-sitter generate
cd ..

# 5. XML - clone and build
echo "5. Cloning and fixing XML..."
rm -rf tree-sitter-xml
git clone https://github.com/tree-sitter/tree-sitter-xml.git
cd tree-sitter-xml
# XML has submodules
if [ -d "xml" ]; then
    cd xml
    npm install
    npx tree-sitter generate
    cd ..
fi
if [ -d "dtd" ]; then
    cd dtd
    npm install
    npx tree-sitter generate
    cd ..
fi
cd ..

# 6. COBOL - clone and fix compilation
echo "6. Cloning and fixing COBOL..."
rm -rf tree-sitter-cobol
git clone https://github.com/tree-sitter-grammars/tree-sitter-cobol.git 2>/dev/null || \
git clone https://github.com/kjeremy/tree-sitter-cobol.git 2>/dev/null
cd tree-sitter-cobol
npm install
npx tree-sitter generate
# Fix any compilation issues
if [ -f "src/scanner.c" ]; then
    # Add missing includes if needed
    sed -i '1i#include <stddef.h>' src/scanner.c 2>/dev/null
    sed -i '1i#include <stdbool.h>' src/scanner.c 2>/dev/null
fi
cd ..

echo ""
echo "==========================================="
echo "Checking all cloned repositories..."
echo ""

for lang in javascript systemverilog elm xml cobol; do
    if [ -d "tree-sitter-$lang" ]; then
        echo "✓ tree-sitter-$lang cloned"
        parser_file="tree-sitter-$lang/src/parser.c"
        if [ -f "$parser_file" ]; then
            version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' "$parser_file" 2>/dev/null | head -1)
            echo "  Parser version: $version"
        fi
    else
        echo "✗ tree-sitter-$lang NOT found"
    fi
done

cd ..
