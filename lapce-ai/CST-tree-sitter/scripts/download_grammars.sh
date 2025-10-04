#!/bin/bash
# Download and prepare tree-sitter grammars for compilation

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
GRAMMARS_DIR="$PROJECT_DIR/grammars"

echo "Setting up grammars directory at $GRAMMARS_DIR"
mkdir -p "$GRAMMARS_DIR"

# Function to clone or update a grammar
download_grammar() {
    local name=$1
    local repo=$2
    local branch=${3:-master}
    
    echo "Downloading $name from $repo"
    
    if [ -d "$GRAMMARS_DIR/$name" ]; then
        echo "  Updating existing $name"
        cd "$GRAMMARS_DIR/$name"
        git pull origin "$branch"
    else
        echo "  Cloning $name"
        cd "$GRAMMARS_DIR"
        git clone --depth 1 --branch "$branch" "$repo" "$name"
    fi
    
    # Generate parser if needed
    if [ -f "$GRAMMARS_DIR/$name/grammar.js" ]; then
        echo "  Generating parser for $name"
        cd "$GRAMMARS_DIR/$name"
        if command -v tree-sitter &> /dev/null; then
            tree-sitter generate || true
        else
            echo "  Warning: tree-sitter CLI not found, skipping generation"
        fi
    fi
}

# Download grammars that need version 0.22+
echo "Downloading tree-sitter 0.22 grammars..."
download_grammar "swift" "https://github.com/alex-pinkus/tree-sitter-swift"
download_grammar "kotlin" "https://github.com/fwcd/tree-sitter-kotlin"
download_grammar "scala" "https://github.com/tree-sitter/tree-sitter-scala"
download_grammar "julia" "https://github.com/tree-sitter/tree-sitter-julia"
download_grammar "r" "https://github.com/r-lib/tree-sitter-r"
download_grammar "nix" "https://github.com/cstrahan/tree-sitter-nix"
download_grammar "d" "https://github.com/gdamore/tree-sitter-d"
download_grammar "solidity" "https://github.com/JoranHonig/tree-sitter-solidity"
download_grammar "agda" "https://github.com/tree-sitter/tree-sitter-agda"

# Download grammars that need version 0.23+
echo "Downloading tree-sitter 0.23 grammars..."
download_grammar "elixir" "https://github.com/elixir-lang/tree-sitter-elixir"
download_grammar "yaml" "https://github.com/ikatyang/tree-sitter-yaml"
download_grammar "xml" "https://github.com/tree-sitter/tree-sitter-xml"

# Download grammars that need version 0.24+
echo "Downloading tree-sitter 0.24 grammars..."
download_grammar "ocaml" "https://github.com/tree-sitter/tree-sitter-ocaml"
download_grammar "zig" "https://github.com/maxxnino/tree-sitter-zig"

# Download grammars that need version 0.25+
echo "Downloading tree-sitter 0.25 grammars..."
download_grammar "erlang" "https://github.com/WhatsApp/tree-sitter-erlang"
download_grammar "clojure" "https://github.com/sogaiu/tree-sitter-clojure"
download_grammar "haskell" "https://github.com/tree-sitter/tree-sitter-haskell"

echo "Grammar download complete!"
echo "Total grammars: $(ls -1 "$GRAMMARS_DIR" | wc -l)"
echo ""
echo "To compile these grammars, run:"
echo "  cd $PROJECT_DIR"
echo "  cargo build --features ffi-languages"
