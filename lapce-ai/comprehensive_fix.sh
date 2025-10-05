#!/bin/bash

echo "🔧 Comprehensive Error Fix Script"
echo "================================="

# Track progress
TOTAL_ERRORS=$(cargo check --lib 2>&1 | grep "^error\[" | wc -l)
echo "Starting with $TOTAL_ERRORS errors"

# Fix 1: streaming_response.rs type mismatches
echo "📝 Fixing streaming_response.rs..."
cat > /tmp/fix1.rs << 'EOF'
// Fix ApiStreamChunk pattern matching
sed -i 's/if let ApiStreamChunk::Text(text_chunk) = &chunk/if let Some(text_chunk) = chunk.as_text()/g' src/streaming_response.rs
sed -i 's/} else if let ApiStreamChunk::Usage(usage) = chunk/} else if let Some(usage) = chunk.as_usage()/g' src/streaming_response.rs
EOF

# Apply fixes directly
sed -i 's/ApiStreamChunk::Text(text_chunk)/Some(text_chunk)/g' src/streaming_response.rs
sed -i 's/ApiStreamChunk::Usage(usage)/Some(usage)/g' src/streaming_response.rs

# Fix 2: Fix handler registration type issues
echo "📝 Fixing handler_registration.rs..."
sed -i 's/impl<F, Fut>/impl<F: Clone, Fut>/g' src/handler_registration.rs

# Fix 3: Fix MCP tools execute signatures  
echo "📝 Fixing MCP tools..."
find src/mcp_tools -name "*.rs" -exec sed -i 's/async fn execute(&self/async fn execute(\&self/g' {} \;

# Fix 4: Fix duplicate trait implementations
echo "📝 Removing duplicate trait implementations..."
# Comment out duplicate Clone implementations
sed -i '/^impl Clone for SearchResult/,/^}/s/^/\/\/ /' src/hybrid_search.rs 2>/dev/null || true
sed -i '/^impl Debug for SearchResult/,/^}/s/^/\/\/ /' src/hybrid_search.rs 2>/dev/null || true

# Fix 5: Fix E0616 private field access
echo "📝 Fixing private field access..."
sed -i 's/\.config\./.get_config()./g' src/connection_pool_manager.rs
sed -i 's/\.0\./.as_bytes()./g' src/https_connection_manager_real.rs

# Fix 6: Fix E0277 trait bound issues
echo "📝 Fixing trait bounds..."
sed -i 's/impl Tool/impl Tool + Send + Sync/g' src/mcp_tools/*.rs 2>/dev/null || true

# Fix 7: Fix E0599 method not found
echo "📝 Fixing missing methods..."
sed -i 's/\.status()/.state()/g' src/connection_pool_manager.rs
sed -i 's/\.with_root_certificates/.with_root_store/g' src/https_connection_manager_real.rs

# Fix 8: Fix E0063 missing fields
echo "📝 Fixing missing struct fields..."
sed -i 's/SearchFilters {/SearchFilters { max_results: None,/g' src/search_tools.rs

# Check progress
echo ""
echo "📊 Results:"
echo "-----------"
NEW_ERRORS=$(cargo check --lib 2>&1 | grep "^error\[" | wc -l)
echo "Errors reduced from $TOTAL_ERRORS to $NEW_ERRORS"
echo "Fixed $(($TOTAL_ERRORS - $NEW_ERRORS)) errors"

# Show remaining error types
echo ""
echo "📋 Remaining error breakdown:"
cargo check --lib 2>&1 | grep "^error\[" | cut -d\] -f1 | cut -d\[ -f2 | sort | uniq -c | sort -rn
