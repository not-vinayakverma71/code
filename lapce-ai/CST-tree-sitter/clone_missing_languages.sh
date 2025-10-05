#!/bin/bash

echo "Cloning and fixing JavaScript and SystemVerilog..."
echo "=================================================="

cd external-grammars

# 1. JavaScript - get it directly
echo "1. Cloning JavaScript..."
if [ ! -d "tree-sitter-javascript" ]; then
    git clone https://github.com/tree-sitter/tree-sitter-javascript.git
    cd tree-sitter-javascript
    # Checkout older version before v0.21.0
    git checkout tags/v0.20.4 2>/dev/null || git checkout tags/v0.20.3 2>/dev/null || git checkout tags/v0.20.2 2>/dev/null || git checkout tags/v0.20.1 2>/dev/null
    
    # Install and generate with old tree-sitter
    npm install tree-sitter-cli@0.20.8
    npx tree-sitter generate
    
    echo "JavaScript parser version:"
    grep '#define LANGUAGE_VERSION' src/parser.c | head -1
    cd ..
else
    echo "JavaScript already exists"
fi

# 2. SystemVerilog 
echo ""
echo "2. Cloning SystemVerilog..."
if [ ! -d "tree-sitter-systemverilog" ]; then
    git clone https://github.com/zhangwwpeng/tree-sitter-systemverilog.git
    cd tree-sitter-systemverilog
    
    # Fix the reserved function issue first
    cat > grammar_temp.js << 'EOF'
// Helper function for SystemVerilog
function reserved(name, rule) {
    return rule;
}

EOF
    cat grammar.js >> grammar_temp.js
    mv grammar_temp.js grammar.js
    
    npm install tree-sitter-cli@0.20.8
    npx tree-sitter generate
    
    echo "SystemVerilog parser version:"
    grep '#define LANGUAGE_VERSION' src/parser.c | head -1
    cd ..
else
    echo "SystemVerilog already exists"
fi

echo ""
echo "=================================================="
echo "Verifying all languages are present:"
echo ""

for lang in javascript systemverilog elm xml cobol; do
    if [ -d "tree-sitter-$lang" ]; then
        echo "✓ tree-sitter-$lang"
        # Check parser version
        parser_file=$(find "tree-sitter-$lang" -name "parser.c" -type f | head -1)
        if [ ! -z "$parser_file" ]; then
            version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' "$parser_file" | head -1)
            echo "  Version: $version"
        fi
    else
        echo "✗ tree-sitter-$lang MISSING"
    fi
done

cd ..
