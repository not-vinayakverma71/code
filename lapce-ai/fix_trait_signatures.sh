#!/bin/bash

# Fix trait signature issues

# Find files with incorrect stream signatures and list them
echo "Files with trait implementation issues:"
grep -l "async fn stream.*ProviderResponse" src/ai_providers/*.rs

# The issue is that Provider trait expects stream to return ProviderResponse but some implementations might differ

# Check actual Provider trait definition
echo "Current Provider trait:"
grep -A2 "async fn stream" src/ai_providers/providers_openai.rs

# Add missing DEEP_SEEK_DEFAULT_TEMPERATURE
grep -q "DEEP_SEEK_DEFAULT_TEMPERATURE" src/ai_providers/providers_deepseek.rs || echo "pub const DEEP_SEEK_DEFAULT_TEMPERATURE: f32 = 0.7;" >> src/ai_providers/providers_deepseek.rs

# Add missing ImageUrl to simple_format
echo "
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}" >> src/ai_providers/simple_format.rs

cargo build --lib 2>&1 | grep "error: could not compile"
