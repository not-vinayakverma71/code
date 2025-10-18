#!/bin/bash

echo "Systematically fixing all remaining errors..."

# Fix 1: Type mismatches in streaming_response.rs
echo "Fixing type mismatches..."
cat > /tmp/fix_streaming_response.patch << 'EOF'
--- a/src/streaming_response.rs
+++ b/src/streaming_response.rs
@@ -140,3 +140,3 @@
                 model: model_id.clone(),
                 messages: converted_messages,
-                temperature: self.get_temperature(&model_id),
+                temperature: self.get_temperature(&model_id).unwrap_or(0.7),
@@ -143 +143 @@
-                max_tokens: self.get_max_tokens(&model_info),
+                max_tokens: self.get_max_tokens(&model_info).unwrap_or(4096),
EOF

# Fix 2: Remove duplicate trait implementations
echo "Removing duplicate implementations..."
find src -name "*.rs" -exec sed -i 's/impl Clone for SearchResult/\/\/ impl Clone for SearchResult/g' {} \;
find src -name "*.rs" -exec sed -i 's/impl Debug for SearchResult/\/\/ impl Debug for SearchResult/g' {} \;

# Fix 3: Fix async trait signatures
echo "Fixing async trait signatures..."
sed -i 's/async fn execute(&self,/async fn execute(\&self,/g' src/mcp_tools/tools/*.rs

# Fix 4: Fix field access issues
echo "Fixing field access issues..."
sed -i 's/\.path_field/\.path/g' src/**/*.rs 2>/dev/null || true
sed -i 's/\.content_field/\.content/g' src/**/*.rs 2>/dev/null || true

echo "Done with automatic fixes. Checking remaining errors..."
cargo check --lib 2>&1 | grep "^error\[" | wc -l
