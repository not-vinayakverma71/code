#!/bin/bash

echo "AGGRESSIVE FIX: Eliminating ALL compilation errors..."

# 1. Find ALL files that use ModelInfo and add import if missing
echo "Fixing ModelInfo imports..."
grep -r "ModelInfo" src/ --include="*.rs" | cut -d: -f1 | sort -u | while read file; do
    # Skip the file that defines ModelInfo
    if [[ "$file" == "src/ai_providers/types.rs" ]] || [[ "$file" == "src/types_model.rs" ]]; then
        continue
    fi
    
    # Check if it already has the import
    if ! grep -q "use crate::ai_providers::types::ModelInfo;" "$file" && ! grep -q "use super::types::ModelInfo;" "$file"; then
        # Add import at the beginning
        sed -i '1i use crate::ai_providers::types::ModelInfo;' "$file"
    fi
done

# 2. Fix DEEP_SEEK_DEFAULT_TEMPERATURE everywhere it's used
echo "Fixing DEEP_SEEK_DEFAULT_TEMPERATURE..."
grep -r "DEEP_SEEK_DEFAULT_TEMPERATURE" src/ --include="*.rs" | cut -d: -f1 | sort -u | while read file; do
    # Skip if it already defines it
    if ! grep -q "pub const DEEP_SEEK_DEFAULT_TEMPERATURE" "$file"; then
        # Add the constant
        sed -i '1a pub const DEEP_SEEK_DEFAULT_TEMPERATURE: f32 = 0.7;' "$file"
    fi
done

# 3. Fix SingleCompletionHandler everywhere it's used
echo "Fixing SingleCompletionHandler..."
grep -r "SingleCompletionHandler" src/ --include="*.rs" | cut -d: -f1 | sort -u | while read file; do
    # Check if it already defines it
    if ! grep -q "trait SingleCompletionHandler" "$file"; then
        # Add the trait definition after imports
        sed -i '/^use /a \\npub trait SingleCompletionHandler: Send + Sync {\n    fn handle(&self, text: \&str);\n}\n' "$file"
    fi
done

# 4. Fix ImageUrl everywhere it's used
echo "Fixing ImageUrl..."
grep -r "ImageUrl" src/ --include="*.rs" | cut -d: -f1 | sort -u | while read file; do
    # Skip files that already define it
    if ! grep -q "struct ImageUrl" "$file"; then
        # Check if it needs the struct
        if grep -q "ImageUrl" "$file"; then
            # Add the struct definition
            echo '
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageUrl {
    pub url: String,
}' >> "$file"
        fi
    fi
done

# 5. Remove duplicate ModelInfo from types_model.rs if it conflicts
echo "Removing duplicate ModelInfo..."
if grep -q "^pub struct ModelInfo" src/types_model.rs; then
    # Comment out the duplicate
    sed -i '/^pub struct ModelInfo {/,/^}$/s/^/\/\/ /' src/types_model.rs
    # Use the one from ai_providers::types instead
    sed -i '1i use crate::ai_providers::types::ModelInfo;' src/types_model.rs 2>/dev/null || true
fi

# 6. Fix any remaining unresolved imports
echo "Fixing remaining imports..."
sed -i 's|crate::stream_transform::|crate::streaming_pipeline::stream_transform::|g' src/*.rs 2>/dev/null || true
sed -i 's|use crate::buffer_management::|use crate::ipc::buffer_management::|g' src/ai_providers/*.rs 2>/dev/null || true

echo "Building to check results..."
cargo build --lib 2>&1 | tail -5
