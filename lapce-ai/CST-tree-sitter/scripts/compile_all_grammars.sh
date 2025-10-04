#!/bin/bash
# Compile ALL 122 tree-sitter grammars into FFI bindings

set -e
cd grammars

echo "Compiling all tree-sitter grammars..."

# Function to compile a single grammar
compile_grammar() {
    local dir=$1
    local name=$2
    
    if [ ! -d "$dir" ]; then
        echo "Skipping $name (not downloaded)"
        return
    fi
    
    echo "Compiling $name..."
    cd "$dir"
    
    # Check if parser.c exists
    if [ ! -f "src/parser.c" ]; then
        # Try to generate it if grammar.js exists
        if [ -f "grammar.js" ] && command -v npx &> /dev/null; then
            npx tree-sitter generate 2>/dev/null || true
        fi
    fi
    
    # Compile parser.c if it exists
    if [ -f "src/parser.c" ]; then
        gcc -c -fPIC src/parser.c -o parser.o -I. -Isrc 2>/dev/null || true
    fi
    
    # Compile scanner if exists
    if [ -f "src/scanner.cc" ]; then
        g++ -c -fPIC src/scanner.cc -o scanner.o -I. -Isrc 2>/dev/null || true
    elif [ -f "src/scanner.c" ]; then
        gcc -c -fPIC src/scanner.c -o scanner.o -I. -Isrc 2>/dev/null || true
    fi
    
    # Create static library
    if [ -f "parser.o" ]; then
        if [ -f "scanner.o" ]; then
            ar rcs ../lib${name}.a parser.o scanner.o 2>/dev/null && echo "  ✅ lib${name}.a created"
        else
            ar rcs ../lib${name}.a parser.o 2>/dev/null && echo "  ✅ lib${name}.a created (no scanner)"
        fi
    else
        echo "  ❌ Failed to compile $name"
    fi
    
    cd ..
}

# Compile all existing grammars
for dir in */; do
    if [ -d "$dir" ]; then
        name=${dir%/}
        compile_grammar "$dir" "$name"
    fi
done

# Count results
echo ""
echo "Compilation complete!"
echo "Libraries created: $(ls *.a 2>/dev/null | wc -l)"
echo "Available at: $(pwd)/*.a"
