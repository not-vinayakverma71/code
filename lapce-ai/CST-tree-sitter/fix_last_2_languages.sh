#!/bin/bash

echo "Fixing the last 2 languages: SystemVerilog and XML"
echo "=================================================="

cd external-grammars

# 1. SystemVerilog (using tree-sitter-verilog)
echo "1. Fixing SystemVerilog..."
cd tree-sitter-systemverilog
npm install tree-sitter-cli@0.20.8
npx tree-sitter generate
echo "SystemVerilog parser version: $(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c 2>/dev/null | head -1)"

# Create Cargo.toml if missing
if [ ! -f "Cargo.toml" ]; then
    cat > Cargo.toml << 'EOF'
[package]
name = "tree-sitter-systemverilog"
description = "SystemVerilog grammar for tree-sitter"
version = "0.1.0"
edition = "2021"
build = "bindings/rust/build.rs"

[lib]
path = "bindings/rust/lib.rs"

[dependencies]
tree-sitter = "0.23.0"

[build-dependencies]
cc = "1.0"
EOF
    echo "Created Cargo.toml for SystemVerilog"
fi
cd ..

# 2. XML - regenerate with older tree-sitter
echo ""
echo "2. Fixing XML..."
cd tree-sitter-xml

# XML has multiple parsers (xml, dtd)
for subdir in xml dtd; do
    if [ -d "$subdir" ]; then
        echo "  Processing $subdir..."
        cd "$subdir"
        
        # Use tree-sitter 0.20.8
        npm install tree-sitter-cli@0.20.8
        npx tree-sitter generate
        
        version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' src/parser.c 2>/dev/null | head -1)
        echo "  $subdir parser version: $version"
        
        cd ..
    fi
done

# Update main Cargo.toml
if [ -f "Cargo.toml" ]; then
    sed -i 's/tree-sitter = ".*"/tree-sitter = "0.23.0"/g' Cargo.toml
    echo "Updated XML Cargo.toml"
fi

cd ..

echo ""
echo "=================================================="
echo "Final check of all 6 languages:"
echo ""

# Check JSX separately (it uses JavaScript parser)
echo "Note: JSX uses the JavaScript parser"
echo ""

all_working=true
for lang in javascript systemverilog elm xml cobol; do
    dir="tree-sitter-$lang"
    if [ -d "$dir" ]; then
        # Find parser.c
        parser_file=$(find "$dir" -name "parser.c" -type f 2>/dev/null | head -1)
        if [ ! -z "$parser_file" ]; then
            version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' "$parser_file" | head -1)
            if [ "$version" = "14" ]; then
                echo "✅ $lang - Version $version (WORKING)"
            else
                echo "⚠️  $lang - Version $version (needs fix)"
                all_working=false
            fi
        else
            echo "❌ $lang - No parser.c"
            all_working=false
        fi
    else
        echo "❌ $lang - Not found"
        all_working=false
    fi
done

echo ""
if [ "$all_working" = true ]; then
    echo "🎉 ALL LANGUAGES FIXED AND READY!"
else
    echo "Some languages still need attention"
fi

cd ..
