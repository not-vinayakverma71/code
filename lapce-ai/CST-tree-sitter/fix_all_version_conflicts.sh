#!/bin/bash

echo "Fixing ALL external grammar version conflicts..."

# Find and update all Cargo.toml files in external grammars to use tree-sitter 0.23.0
for grammar_dir in external-grammars/tree-sitter-*/; do
  cargo_file="$grammar_dir/Cargo.toml"
  if [ -f "$cargo_file" ]; then
    grammar_name=$(basename $grammar_dir)
    echo "Updating $grammar_name..."
    
    # Replace all tree-sitter versions with 0.23.0
    sed -i 's/tree-sitter = "0\.[0-9]\+\.[0-9]\+"/tree-sitter = "0.23.0"/g' "$cargo_file"
    sed -i 's/tree-sitter = "0\.[0-9]\+"/tree-sitter = "0.23.0"/g' "$cargo_file"
    
    # Also fix dev-dependencies
    sed -i 's/\[dev-dependencies\]/[dev-dependencies]/g' "$cargo_file"
    sed -i '/\[dev-dependencies\]/,/^\[/{s/tree-sitter = "0\.[0-9]\+\.[0-9]\+"/tree-sitter = "0.23.0"/g}' "$cargo_file"
  fi
done

# Special handling for problem grammars
echo "Special handling for specific grammars..."

# JavaScript - ensure it's using 0.23
if [ -f "external-grammars/tree-sitter-javascript/Cargo.toml" ]; then
  echo "Fixing JavaScript..."
  cat > external-grammars/tree-sitter-javascript/Cargo.toml << 'EOF'
[package]
name = "tree-sitter-javascript"
description = "JavaScript grammar for tree-sitter"
version = "0.25.0"
authors = ["Max Brunsfeld <maxbrunsfeld@gmail.com>", "Amaan Qureshi <amaanq12@gmail.com>"]
license = "MIT"
readme = "README.md"
keywords = ["incremental", "parsing", "tree-sitter", "javascript"]
categories = ["parser-implementations", "parsing", "text-editors"]
repository = "https://github.com/tree-sitter/tree-sitter-javascript"
edition = "2021"
autoexamples = false

build = "bindings/rust/build.rs"
include = ["bindings/rust/*", "grammar.js", "queries/*", "src/*", "tree-sitter.json", "LICENSE"]

[lib]
path = "bindings/rust/lib.rs"

[dependencies]
tree-sitter = "0.23.0"

[build-dependencies]
cc = "1.0"

[dev-dependencies]
tree-sitter = "0.23.0"
EOF
fi

# TypeScript - ensure it's using 0.23
if [ -f "external-grammars/tree-sitter-typescript/Cargo.toml" ]; then
  echo "Fixing TypeScript..."
  sed -i 's/tree-sitter = "0\.[0-9]\+\.[0-9]\+"/tree-sitter = "0.23.0"/g' external-grammars/tree-sitter-typescript/Cargo.toml
fi

# Fix Markdown
if [ -f "external-grammars/tree-sitter-markdown/Cargo.toml" ]; then
  echo "Fixing Markdown..."
  sed -i 's/tree-sitter = "0\.[0-9]\+\.[0-9]\+"/tree-sitter = "0.23.0"/g' external-grammars/tree-sitter-markdown/Cargo.toml
fi

# Check for any remaining version conflicts
echo ""
echo "Checking for remaining version conflicts..."
grep -r "tree-sitter.*0\.[2-9][4-9]" external-grammars/*/Cargo.toml 2>/dev/null | head -20 || echo "No version conflicts found"

echo "Done!"
