#!/bin/bash

echo "Fixing ALL 44 compilation errors..."

# 1. Create missing handler_registration_types.rs
cat > src/ipc/handler_registration_types.rs << 'EOF'
/// Handler registration types
use std::sync::Arc;
use anyhow::Result;

pub type HandlerFn = Arc<dyn Fn() -> Result<()> + Send + Sync>;

pub struct HandlerContext {
    pub id: String,
    pub name: String,
}

pub enum HandlerType {
    Command,
    Event,
    Request,
}
EOF

# 2. Fix the imports in handler_registration.rs
sed -i 's|// use super::handler_registration_types::\*;|use super::handler_registration_types::*;|' src/ipc/handler_registration.rs

# 3. Add handler_registration_types to ipc mod.rs
if ! grep -q "pub mod handler_registration_types;" src/ipc/mod.rs; then
    sed -i '/pub mod handler_registration;/a pub mod handler_registration_types;' src/ipc/mod.rs
fi

# 4. Fix ModelInfo imports in all files
echo "Fixing ModelInfo imports..."
find src/ai_providers -name "*.rs" -exec grep -l "cannot find type \`ModelInfo\`" {} \; | while read file; do
    if ! grep -q "use crate::ai_providers::types::ModelInfo;" "$file"; then
        sed -i '1i use crate::ai_providers::types::ModelInfo;' "$file"
    fi
done

# 5. Add DEEP_SEEK_DEFAULT_TEMPERATURE
if ! grep -q "DEEP_SEEK_DEFAULT_TEMPERATURE" src/ai_providers/providers_deepseek.rs; then
    echo "pub const DEEP_SEEK_DEFAULT_TEMPERATURE: f32 = 0.7;" >> src/ai_providers/providers_deepseek.rs
fi

# 6. Add ImageUrl to simple_format.rs if missing
if ! grep -q "struct ImageUrl" src/ai_providers/simple_format.rs; then
    echo '
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}' >> src/ai_providers/simple_format.rs
fi

# 7. Add SingleCompletionHandler trait
if ! grep -q "trait SingleCompletionHandler" src/ai_providers/openai_provider_handler.rs; then
    echo '
pub trait SingleCompletionHandler: Send + Sync {
    fn handle(&self, text: &str);
}' >> src/ai_providers/openai_provider_handler.rs
fi

echo "Fixes applied. Building..."
cargo build --lib 2>&1 | grep "error: could not compile"
