#!/bin/bash

# Fix all imports systematically

# Remove all duplicate ModelInfo lines from provider files  
find src/ai_providers -name "*.rs" -exec sh -c '
    tmpfile=$(mktemp)
    awk "!(/ModelInfo/ && seen[\$0]++)" "$1" > "$tmpfile"
    mv "$tmpfile" "$1"
' _ {} \;

# Ensure each provider file has exactly one ModelInfo import at the top
find src/ai_providers -name "providers_*.rs" -exec sh -c '
    if ! grep -q "^use crate::ai_providers::types::ModelInfo;" "$1"; then
        tmpfile=$(mktemp)
        echo "use crate::ai_providers::types::ModelInfo;" > "$tmpfile"
        grep -v "^use crate::ai_providers::types::ModelInfo;" "$1" >> "$tmpfile"
        mv "$tmpfile" "$1"
    fi
' _ {} \;

# Remove duplicate ContentPart definitions
sed -i '/^pub enum ContentPart {/,/^}$/d' src/ai_providers/r1_format.rs 2>/dev/null || true

echo "Imports fixed"
