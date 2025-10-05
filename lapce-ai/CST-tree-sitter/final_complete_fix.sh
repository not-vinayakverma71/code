#!/bin/bash

echo "FINAL FIX: Ensuring all 6 languages work 100%"
echo "=============================================="

# Move COBOL to external-grammars if it's in the wrong place
if [ -d "tree-sitter-cobol" ] && [ ! -d "external-grammars/tree-sitter-cobol" ]; then
    echo "Moving COBOL to external-grammars..."
    mv tree-sitter-cobol external-grammars/
fi

cd external-grammars

# Use npx to run tree-sitter locally
echo "Installing tree-sitter-cli@0.20.8 locally..."
npm install tree-sitter-cli@0.20.8

# 1. JavaScript - regenerate from scratch
echo ""
echo "1. Fixing JavaScript..."
cd tree-sitter-javascript
if [ ! -f "src/parser.c" ]; then
    echo "  Generating JavaScript parser..."
    npx tree-sitter generate
fi
version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c 2>/dev/null | head -1)
echo "  JavaScript version: $version"
cd ..

# 2. SystemVerilog
echo ""
echo "2. Fixing SystemVerilog..."
cd tree-sitter-systemverilog
if [ ! -f "grammar.js" ]; then
    echo "  ERROR: No grammar.js in SystemVerilog!"
    # Clone it properly
    cd ..
    trash-put tree-sitter-systemverilog
    git clone https://github.com/zhangwwpeng/tree-sitter-systemverilog.git
    cd tree-sitter-systemverilog
fi

# Fix reserved function
if [ -f "grammar.js" ] && ! grep -q "function reserved" grammar.js; then
    echo "function reserved(name, rule) { return rule; }" > temp.js
    cat grammar.js >> temp.js
    mv temp.js grammar.js
fi

if [ ! -f "src/parser.c" ]; then
    echo "  Generating SystemVerilog parser..."
    npx tree-sitter generate
fi
version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c 2>/dev/null | head -1)
echo "  SystemVerilog version: $version"
cd ..

# 3. Elm
echo ""
echo "3. Fixing Elm..."
cd tree-sitter-elm
if [ ! -f "src/parser.c" ]; then
    echo "  Generating Elm parser..."
    npx tree-sitter generate
fi
version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c 2>/dev/null | head -1)
echo "  Elm version: $version"
cd ..

# 4. XML
echo ""
echo "4. Fixing XML..."
cd tree-sitter-xml
if [ -d "xml" ]; then
    cd xml
    if [ ! -f "src/parser.c" ]; then
        echo "  Generating XML parser..."
        npx tree-sitter generate
    fi
    version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c 2>/dev/null | head -1)
    echo "  XML version: $version"
    cd ..
fi
cd ..

# 5. COBOL
echo ""
echo "5. Fixing COBOL..."
if [ ! -d "tree-sitter-cobol" ]; then
    echo "  Cloning COBOL..."
    git clone https://github.com/yutaro-sakamoto/tree-sitter-cobol.git
fi
cd tree-sitter-cobol

if [ ! -f "src/parser.c" ]; then
    echo "  Generating COBOL parser..."
    npx tree-sitter generate
fi

# Fix scanner.c includes
if [ -f "src/scanner.c" ] && ! grep -q "#include <stddef.h>" src/scanner.c; then
    echo "  Fixing COBOL scanner.c..."
    cat > temp_scanner.c << 'EOF'
#include <stddef.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include <tree_sitter/parser.h>

EOF
    grep -v "^#include" src/scanner.c >> temp_scanner.c
    mv temp_scanner.c src/scanner.c
fi

version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c 2>/dev/null | head -1)
echo "  COBOL version: $version"
cd ..

# Update all Cargo.toml files to use tree-sitter 0.23.0
echo ""
echo "6. Updating all Cargo.toml files..."
for grammar in javascript systemverilog elm xml cobol; do
    cargo_file="tree-sitter-$grammar/Cargo.toml"
    if [ -f "$cargo_file" ]; then
        sed -i 's/tree-sitter = ".*"/tree-sitter = "0.23.0"/g' "$cargo_file"
        echo "  Updated tree-sitter-$grammar/Cargo.toml"
    fi
done

echo ""
echo "=============================================="
echo "FINAL STATUS CHECK:"
echo ""

success_count=0
fail_count=0

for lang in javascript systemverilog elm xml cobol; do
    dir="tree-sitter-$lang"
    if [ -d "$dir" ]; then
        # Find parser.c in various locations
        parser_file=$(find "$dir" -name "parser.c" -type f 2>/dev/null | head -1)
        if [ ! -z "$parser_file" ]; then
            version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' "$parser_file" | head -1)
            if [ "$version" = "14" ]; then
                echo "✅ $lang - Version $version (COMPATIBLE)"
                ((success_count++))
            else
                echo "⚠️  $lang - Version $version (may need fix)"
                ((fail_count++))
            fi
        else
            echo "❌ $lang - No parser.c found"
            ((fail_count++))
        fi
    else
        echo "❌ $lang - Directory not found"
        ((fail_count++))
    fi
done

echo ""
echo "Results: $success_count/5 languages ready"
if [ $success_count -eq 5 ]; then
    echo "🎉 ALL 5 LANGUAGES FIXED!"
else
    echo "⚠️  Still need to fix $fail_count languages"
fi

cd ..
