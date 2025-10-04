#!/bin/bash

# Quick fix to get the library to compile

# Add missing constants
echo "pub const DEEP_SEEK_DEFAULT_TEMPERATURE: f32 = 0.7;" >> src/ai_providers/providers_deepseek.rs

# Fix remaining compilation errors by adding stub types and implementations
cat << 'EOF' > src/fix_missing_types.rs
// Temporary fixes for missing types
pub type ChatCompletionStream = futures::stream::BoxStream<'static, Result<String, anyhow::Error>>;
EOF

# Now try to build and show remaining errors
cargo build --lib 2>&1 | grep -E "^error\[" | head -20
