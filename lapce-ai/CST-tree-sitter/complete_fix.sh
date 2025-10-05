#!/bin/bash

echo "Complete rebuild of all external grammars with tree-sitter 0.23..."

# Clean all external grammar builds
for dir in external-grammars/tree-sitter-*/; do
  if [ -d "$dir" ]; then
    echo "Cleaning $(basename $dir)..."
    (cd "$dir" && cargo clean 2>/dev/null)
  fi
done

# Clean main build
cargo clean

# Force rebuild everything
echo "Building test binary..."
cargo build --release --bin test_all_63_languages 2>&1 | tail -5

echo "Build complete!"
