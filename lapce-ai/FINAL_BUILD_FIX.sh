#!/bin/bash

echo "FINAL FIX: Resolving all 14 remaining errors..."

# Fix ALL unresolved types and imports

# 1. Fix providers_gemini ModelInfo duplicate
sed -i '/^use crate::types_model::/d' src/ai_providers/providers_gemini.rs

# 2. Ensure DEEP_SEEK_DEFAULT_TEMPERATURE exists
grep -q "DEEP_SEEK_DEFAULT_TEMPERATURE" src/ai_providers/providers_deepseek.rs || \
echo "pub const DEEP_SEEK_DEFAULT_TEMPERATURE: f32 = 0.7;" >> src/ai_providers/providers_deepseek.rs

# 3. Fix SingleCompletionHandler
grep -q "SingleCompletionHandler" src/ai_providers/openai_provider_handler.rs || \
echo "
pub trait SingleCompletionHandler: Send + Sync {
    fn handle(&self, text: &str);
}" >> src/ai_providers/openai_provider_handler.rs

# 4. Fix ImageUrl in simple_format
grep -q "struct ImageUrl" src/ai_providers/simple_format.rs || \
echo "
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}" >> src/ai_providers/simple_format.rs

# 5. Fix ALL files that need ModelInfo import
FILES=(
    "src/ai_providers/base_provider.rs"
    "src/ai_providers/openrouter_provider.rs"
    "src/types_model.rs"
)

for file in "${FILES[@]}"; do
    if [ -f "$file" ] && ! grep -q "use crate::ai_providers::types::ModelInfo;" "$file"; then
        sed -i '1i use crate::ai_providers::types::ModelInfo;' "$file"
    fi
done

# 6. Fix buffer_management import in openai_provider_handler
sed -i 's|use crate::buffer_management::\*;|use crate::ipc::buffer_management::*;|' src/ai_providers/openai_provider_handler.rs

# 7. Check final build
echo "Running build..."
cargo build --lib 2>&1 | tail -5
