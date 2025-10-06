#!/bin/bash

echo "🔧 Fixing all compilation errors systematically"
echo "==============================================="

# Step 1: Fix library warnings
echo "Step 1: Fixing library warnings..."
cargo fix --lib --allow-dirty 2>&1 | tail -5

# Step 2: Check current state
echo -e "\nStep 2: Checking library compilation..."
if cargo build --lib 2>&1 | grep -q "Finished"; then
    echo "✅ Library compiles successfully"
else
    echo "❌ Library has errors"
fi

# Step 3: Fix each problematic binary
echo -e "\nStep 3: Fixing problematic binaries..."

# Get list of all binaries with errors
FAILED_BINS=$(cargo build --bins 2>&1 | grep "error: could not compile" | sed -n 's/.*bin "\([^"]*\)".*/\1/p' | sort -u)

echo "Found ${#FAILED_BINS[@]} binaries with errors"

# Fix each binary
for bin in $FAILED_BINS; do
    echo "  Fixing $bin..."
    cargo fix --bin $bin --allow-dirty 2>&1 | tail -2
done

# Step 4: Final verification
echo -e "\nStep 4: Final build verification..."
cargo build 2>&1 | tail -10

echo -e "\n✅ Fix process complete"
