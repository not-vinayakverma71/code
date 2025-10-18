#!/bin/bash
# Script to download and compile tree-sitter grammars for FFI
# Hours 140-200: Adding languages via FFI

set -e

GRAMMARS_DIR="grammars"
mkdir -p "$GRAMMARS_DIR"

# Function to download and build a grammar
build_grammar() {
    local name=$1
    local repo=$2
    local branch=${3:-master}
    
    echo "Building $name from $repo..."
    
    cd "$GRAMMARS_DIR"
    
    if [ ! -d "$name" ]; then
        git clone --depth 1 --branch "$branch" "$repo" "$name"
    fi
    
    cd "$name"
    
    # Generate parser if grammar.js exists
    if [ -f "grammar.js" ]; then
        npx tree-sitter generate
    fi
    
    # Compile C sources
    if [ -f "src/parser.c" ]; then
        gcc -c -fPIC src/parser.c -o parser.o -I.
    fi
    
    # Compile C++ scanner if exists
    if [ -f "src/scanner.cc" ]; then
        g++ -c -fPIC src/scanner.cc -o scanner.o -I.
    elif [ -f "src/scanner.c" ]; then
        gcc -c -fPIC src/scanner.c -o scanner.o -I.
    fi
    
    # Create static library
    if [ -f "scanner.o" ]; then
        ar rcs libtree-sitter-$name.a parser.o scanner.o
    else
        ar rcs libtree-sitter-$name.a parser.o
    fi
    
    echo "✅ Built $name"
    cd ../..
}

# Download and build first batch of FFI languages
echo "Starting FFI grammar builds..."

# Swift
build_grammar "swift" "https://github.com/alex-pinkus/tree-sitter-swift" "main"

# Kotlin  
build_grammar "kotlin" "https://github.com/fwcd/tree-sitter-kotlin" "main"

# Scala
build_grammar "scala" "https://github.com/tree-sitter/tree-sitter-scala" "master"

# Haskell
build_grammar "haskell" "https://github.com/tree-sitter/tree-sitter-haskell" "master"

# Erlang
build_grammar "erlang" "https://github.com/WhatsApp/tree-sitter-erlang" "main"

# Elixir
build_grammar "elixir" "https://github.com/elixir-lang/tree-sitter-elixir" "main"

# Clojure
build_grammar "clojure" "https://github.com/sogaiu/tree-sitter-clojure" "master"

# OCaml
build_grammar "ocaml" "https://github.com/tree-sitter/tree-sitter-ocaml" "master"

# R
build_grammar "r" "https://github.com/r-lib/tree-sitter-r" "master"

# Julia
build_grammar "julia" "https://github.com/tree-sitter/tree-sitter-julia" "master"

echo "✅ All grammars built successfully!"
echo "Libraries created in $GRAMMARS_DIR/*/libtree-sitter-*.a"
