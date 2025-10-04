#!/bin/bash

echo "Applying comprehensive fix for all remaining errors..."

# Fix duplicate ModelInfo in providers_vertex.rs
sed -i '/^use crate::ai_providers::types::ModelInfo;$/d' src/ai_providers/providers_vertex.rs
if grep -q "ModelInfo" src/ai_providers/providers_vertex.rs && ! grep -q "^use crate::ai_providers::types::ModelInfo;" src/ai_providers/providers_vertex.rs; then
    sed -i '1i use crate::ai_providers::types::ModelInfo;' src/ai_providers/providers_vertex.rs
fi

# Add missing trait
echo "
pub trait SingleCompletionHandler: Send + Sync {
    fn handle(&self, text: &str);
}" >> src/ai_providers/openai_provider_handler.rs

echo "Compilation errors remaining:"
cargo build --lib 2>&1 | grep "error: could not compile"
