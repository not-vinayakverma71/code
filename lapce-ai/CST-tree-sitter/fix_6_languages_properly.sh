#!/bin/bash

echo "PROPERLY fixing all 6 remaining languages..."
echo "============================================="

cd external-grammars

# 1. JavaScript - use specific older commit
echo "1. Fixing JavaScript with older commit..."
cd tree-sitter-javascript
git fetch --all --tags
# Use commit from before tree-sitter 0.24
git checkout f772967f7b7bc7c28f845be2420a38472b16a8ee 2>/dev/null || git checkout v0.20.1 2>/dev/null
# Install old tree-sitter CLI locally
npm install tree-sitter-cli@0.20.8
npx tree-sitter generate
echo "JavaScript parser version:"
grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c | head -1
cd ..

# 2. SystemVerilog - fix and regenerate
echo ""
echo "2. Fixing SystemVerilog..."
if [ ! -d "tree-sitter-systemverilog" ]; then
    git clone https://github.com/zhangwwpeng/tree-sitter-systemverilog.git
fi
cd tree-sitter-systemverilog
# Add missing function
cat > grammar_fix.js << 'EOF'
function reserved(name, rule) {
    return rule;
}
EOF
cat grammar.js >> grammar_fix.js
mv grammar_fix.js grammar.js
npm install tree-sitter-cli@0.20.8
npx tree-sitter generate
echo "SystemVerilog parser version:"
grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c | head -1
cd ..

# 3. Elm - use compatible version
echo ""
echo "3. Fixing Elm..."
cd tree-sitter-elm
git fetch --all --tags
git checkout v5.6.3 2>/dev/null || git checkout v5.6.0 2>/dev/null
npm install tree-sitter-cli@0.20.8
npx tree-sitter generate
echo "Elm parser version:"
grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c | head -1
cd ..

# 4. JSX - part of tree-sitter-javascript
echo ""
echo "4. JSX is handled by JavaScript parser"

# 5. XML - use tree-sitter-grammars version
echo ""
echo "5. Fixing XML..."
if [ ! -d "tree-sitter-xml" ] || [ ! -f "tree-sitter-xml/xml/grammar.js" ]; then
    rm -rf tree-sitter-xml
    git clone https://github.com/tree-sitter-grammars/tree-sitter-xml.git
fi
cd tree-sitter-xml
if [ -f "xml/grammar.js" ]; then
    cd xml
    npm install tree-sitter-cli@0.20.8
    npx tree-sitter generate
    cd ..
fi
if [ -f "dtd/grammar.js" ]; then
    cd dtd
    npm install tree-sitter-cli@0.20.8
    npx tree-sitter generate
    cd ..
fi
echo "XML parser version:"
if [ -f "xml/src/parser.c" ]; then
    grep -oP '#define LANGUAGE_VERSION \K\d+' xml/src/parser.c | head -1
fi
cd ..

# 6. COBOL - fix compilation
echo ""
echo "6. Fixing COBOL..."
if [ ! -d "tree-sitter-cobol" ]; then
    git clone https://github.com/yutaro-sakamoto/tree-sitter-cobol.git
fi
cd tree-sitter-cobol
npm install tree-sitter-cli@0.20.8
npx tree-sitter generate

# Fix scanner.c if exists
if [ -f "src/scanner.c" ]; then
    # Add missing includes at the top
    echo '#include <stddef.h>' > scanner_fixed.c
    echo '#include <stdbool.h>' >> scanner_fixed.c
    echo '#include <stdint.h>' >> scanner_fixed.c
    echo '#include <string.h>' >> scanner_fixed.c
    cat src/scanner.c >> scanner_fixed.c
    mv scanner_fixed.c src/scanner.c
fi

echo "COBOL parser version:"
grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c | head -1
cd ..

echo ""
echo "============================================="
echo "Final status check:"
echo ""

for lang in javascript systemverilog elm xml cobol; do
    if [ -d "tree-sitter-$lang" ]; then
        echo "✓ tree-sitter-$lang exists"
        
        # Find parser.c in various locations
        parser_files=$(find "tree-sitter-$lang" -name "parser.c" -type f 2>/dev/null | head -1)
        if [ ! -z "$parser_files" ]; then
            version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' $parser_files 2>/dev/null | head -1)
            echo "  Parser version: $version (need 14 for tree-sitter 0.23)"
        else
            echo "  No parser.c found"
        fi
    else
        echo "✗ tree-sitter-$lang NOT found"
    fi
done

cd ..
