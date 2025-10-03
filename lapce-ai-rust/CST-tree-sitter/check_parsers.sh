#!/bin/bash
cd external-grammars
for dir in tree-sitter-*/; do
    name="${dir%/}"
    if [ -f "$dir/src/parser.c" ]; then
        echo "✅ $name"
    else
        echo "❌ $name (no parser.c)"
    fi
done
