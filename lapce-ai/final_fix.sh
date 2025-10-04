#!/bin/bash

echo "Final fix to compile the library..."

# Step 1: Remove ALL ModelInfo imports from provider files
find src/ai_providers -name "providers_*.rs" -exec sed -i '/use.*ModelInfo/d' {} \;

# Step 2: Add the correct import ONCE at the beginning of each file that needs it
for file in src/ai_providers/providers_*.rs; do
    if grep -q "ModelInfo" "$file"; then
        # Create temp file with import at top
        tmpfile=$(mktemp)
        echo "use crate::ai_providers::types::ModelInfo;" > "$tmpfile"
        cat "$file" >> "$tmpfile"
        mv "$tmpfile" "$file"
    fi
done

# Step 3: Also remove from types_model duplicate imports
find src/ai_providers -name "*.rs" -exec sed -i 's/use crate::types_model::{ModelInfo,/use crate::types_model::{/g' {} \;
find src/ai_providers -name "*.rs" -exec sed -i 's/use crate::types_model::{ModelInfo}//' {} \;

# Step 4: Build and count errors
echo "Building..."
cargo build --lib 2>&1 | grep "could not compile" | tail -1
