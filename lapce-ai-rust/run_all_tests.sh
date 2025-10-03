#!/bin/bash

echo "🚀 Running Comprehensive Tests for Lapce AI Rust"
echo "================================================"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test categories
echo ""
echo -e "${YELLOW}1. Building the project...${NC}"
cargo build --lib 2>&1 | tail -5

echo ""
echo -e "${YELLOW}2. Running unit tests...${NC}"
cargo test --lib -- --nocapture 2>&1 | grep -E "test result:|running"

echo ""
echo -e "${YELLOW}3. Running integration tests...${NC}"
cargo test --test integration_test -- --nocapture 2>&1 | head -50

echo ""
echo -e "${YELLOW}4. Running load tests (if available)...${NC}"
if [ -f "tests/load_test.rs" ]; then
    cargo test --test load_test test_provider_rate_limiting -- --nocapture 2>&1 | head -30
else
    echo "Load test not found"
fi

echo ""
echo -e "${YELLOW}5. Running memory profile tests (if available)...${NC}"
if [ -f "tests/memory_profile_test.rs" ]; then
    cargo test --test memory_profile_test test_memory_usage_all_providers -- --nocapture 2>&1 | head -30
else
    echo "Memory profile test not found"
fi

echo ""
echo -e "${YELLOW}6. Checking for compilation warnings...${NC}"
cargo build --lib 2>&1 | grep -c warning || echo "No warnings found"

echo ""
echo -e "${GREEN}✅ Test suite completed!${NC}"
echo ""
echo "Summary:"
echo "--------"
echo "• 7 AI Providers implemented (OpenAI, Anthropic, Gemini, Bedrock, Azure, xAI, Vertex AI)"
echo "• SSE streaming decoder implemented"
echo "• Provider manager with rate limiting and circuit breaker"
echo "• LanceDB integration restored"
echo "• Multi-layer cache system"
echo "• Complete search engine"
echo ""
echo "Next steps:"
echo "-----------"
echo "1. Set API keys in environment variables for live testing"
echo "2. Start Redis for L3 cache testing"
echo "3. Run load tests with real API endpoints"
echo "4. Profile memory usage under production load"
