#!/bin/bash

echo "Final systematic fix for all errors..."

# 1. Fix duplicate ModelInfo in providers_vertex
sed -i '/^use crate::types_model::ModelInfo;/d' src/ai_providers/providers_vertex.rs

# 2. Add ModelInfo imports where missing
for file in \
    src/ai_providers/providers_groq.rs \
    src/ai_providers/providers_mistral.rs \
    src/ai_providers/providers_deepseek.rs \
    src/ai_providers/providers_ollama.rs \
    src/ai_providers/providers_bedrock.rs \
    src/ai_providers/providers_cerebras.rs \
    src/ai_providers/providers_sambanova.rs \
    src/ai_providers/providers_xai.rs \
    src/ai_providers/providers_moonshot.rs \
    src/ai_providers/providers_fireworks.rs \
    src/ai_providers/base_provider.rs \
    src/ai_providers/openrouter_provider.rs
do
    if [ -f "$file" ] && ! grep -q "use crate::ai_providers::types::ModelInfo;" "$file"; then
        sed -i '1i use crate::ai_providers::types::ModelInfo;' "$file"
    fi
done

# 3. Fix DEEP_SEEK_DEFAULT_TEMPERATURE
if ! grep -q "pub const DEEP_SEEK_DEFAULT_TEMPERATURE" src/ai_providers/providers_deepseek.rs; then
    echo "pub const DEEP_SEEK_DEFAULT_TEMPERATURE: f32 = 0.7;" >> src/ai_providers/providers_deepseek.rs
fi

# 4. Add SingleCompletionHandler trait
if ! grep -q "trait SingleCompletionHandler" src/ai_providers/openai_provider_handler.rs; then
    cat >> src/ai_providers/openai_provider_handler.rs << 'EOF'

pub trait SingleCompletionHandler: Send + Sync {
    fn handle(&self, text: &str);
}
EOF
fi

# 5. Add ImageUrl to simple_format.rs
if ! grep -q "struct ImageUrl" src/ai_providers/simple_format.rs; then
    cat >> src/ai_providers/simple_format.rs << 'EOF'

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}
EOF
fi

# 6. Add ModelCost to types_provider_settings.rs
if ! grep -q "struct ModelCost" src/ai_providers/types_provider_settings.rs; then
    sed -i '/use crate::ai_providers::types::ModelInfo;/a \
\
#[derive(Debug, Clone, Serialize, Deserialize)]\
pub struct ModelCost {\
    pub prompt: f64,\
    pub completion: f64,\
}' src/ai_providers/types_provider_settings.rs
fi

echo "All fixes applied. Building..."
cargo build --lib 2>&1 | grep "error: could not compile"
