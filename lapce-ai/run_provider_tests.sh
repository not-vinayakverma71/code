#!/bin/bash

# Comprehensive Provider Testing Script
# Tests all 7 AI providers systematically

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}     🚀 AI Provider Comprehensive Testing Suite${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}\n"

# Check if .env file exists
if [ ! -f .env ]; then
    echo -e "${YELLOW}⚠️  Warning: .env file not found${NC}"
    echo -e "Creating .env from .env.example..."
    cp .env.example .env
    echo -e "${RED}Please edit .env and add your API keys${NC}"
    exit 1
fi

# Function to run tests and capture results
run_test() {
    local test_name=$1
    local command=$2
    
    echo -e "\n${BLUE}▶ Running: ${test_name}${NC}"
    echo "────────────────────────────────────────────────"
    
    if eval "$command"; then
        echo -e "${GREEN}✅ ${test_name} PASSED${NC}"
        return 0
    else
        echo -e "${RED}❌ ${test_name} FAILED${NC}"
        return 1
    fi
}

# Build the project first
echo -e "${YELLOW}📦 Building project...${NC}"
cargo build --release --bin test_providers

# Track test results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=()

# Test 1: Health Check
echo -e "\n${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}Stage 1: Provider Health Checks${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"

if run_test "Health Check" "cargo run --release --bin test_providers -- health"; then
    ((PASSED_TESTS++))
else
    FAILED_TESTS+=("Health Check")
fi
((TOTAL_TESTS++))

# Test 2: Quick Test All Providers
echo -e "\n${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}Stage 2: Quick Provider Tests${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"

if run_test "Quick Test All" "cargo run --release --bin test_providers -- test-all --quick"; then
    ((PASSED_TESTS++))
else
    FAILED_TESTS+=("Quick Test All")
fi
((TOTAL_TESTS++))

# Test 3: Individual Provider Tests
echo -e "\n${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}Stage 3: Individual Provider Tests${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"

PROVIDERS=("openai" "anthropic" "gemini" "azure" "vertex" "openrouter" "bedrock")

for provider in "${PROVIDERS[@]}"; do
    # Non-streaming test
    if run_test "${provider} (non-streaming)" "timeout 30 cargo run --release --bin test_providers -- test ${provider}"; then
        ((PASSED_TESTS++))
    else
        FAILED_TESTS+=("${provider} (non-streaming)")
    fi
    ((TOTAL_TESTS++))
    
    # Streaming test
    if run_test "${provider} (streaming)" "timeout 30 cargo run --release --bin test_providers -- test ${provider} --stream"; then
        ((PASSED_TESTS++))
    else
        FAILED_TESTS+=("${provider} (streaming)")
    fi
    ((TOTAL_TESTS++))
done

# Test 4: Comprehensive Tests
echo -e "\n${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}Stage 4: Comprehensive Provider Tests${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"

if run_test "Comprehensive Tests" "timeout 120 cargo run --release --bin test_providers -- test-all"; then
    ((PASSED_TESTS++))
else
    FAILED_TESTS+=("Comprehensive Tests")
fi
((TOTAL_TESTS++))

# Test 5: Performance Benchmarks
echo -e "\n${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}Stage 5: Performance Benchmarks${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"

if run_test "Performance Benchmark" "cargo run --release --bin test_providers -- benchmark --iterations 5"; then
    ((PASSED_TESTS++))
else
    FAILED_TESTS+=("Performance Benchmark")
fi
((TOTAL_TESTS++))

# Test 6: Integration Tests
echo -e "\n${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}Stage 6: Integration Tests${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"

if run_test "Integration Tests" "cargo test --test provider_integration_tests -- --nocapture"; then
    ((PASSED_TESTS++))
else
    FAILED_TESTS+=("Integration Tests")
fi
((TOTAL_TESTS++))

# Test 7: Unit Tests
echo -e "\n${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}Stage 7: Unit Tests${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"

if run_test "Unit Tests" "cargo test --lib ai_providers"; then
    ((PASSED_TESTS++))
else
    FAILED_TESTS+=("Unit Tests")
fi
((TOTAL_TESTS++))

# Final Report
echo -e "\n${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}📊 TEST REPORT${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"

echo -e "\nTotal Tests: ${TOTAL_TESTS}"
echo -e "${GREEN}Passed: ${PASSED_TESTS}${NC}"
echo -e "${RED}Failed: $((TOTAL_TESTS - PASSED_TESTS))${NC}"

if [ ${#FAILED_TESTS[@]} -gt 0 ]; then
    echo -e "\n${RED}Failed Tests:${NC}"
    for test in "${FAILED_TESTS[@]}"; do
        echo -e "  ${RED}❌ ${test}${NC}"
    done
fi

# Calculate success rate
SUCCESS_RATE=$((PASSED_TESTS * 100 / TOTAL_TESTS))

echo -e "\n${CYAN}═══════════════════════════════════════════════════════════${NC}"
if [ ${SUCCESS_RATE} -eq 100 ]; then
    echo -e "${GREEN}🎉 ALL TESTS PASSED! (100%)${NC}"
    echo -e "${GREEN}All 7 AI providers are working correctly!${NC}"
elif [ ${SUCCESS_RATE} -ge 80 ]; then
    echo -e "${YELLOW}⚠️  MOSTLY PASSED (${SUCCESS_RATE}%)${NC}"
    echo -e "${YELLOW}Most providers are working, but some issues remain.${NC}"
else
    echo -e "${RED}❌ TESTS FAILED (${SUCCESS_RATE}% passed)${NC}"
    echo -e "${RED}Significant issues with provider implementations.${NC}"
fi
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"

# Generate detailed log
LOG_FILE="provider_test_results_$(date +%Y%m%d_%H%M%S).log"
echo -e "\n📝 Detailed results saved to: ${LOG_FILE}"

# Exit with appropriate code
if [ ${SUCCESS_RATE} -eq 100 ]; then
    exit 0
else
    exit 1
fi
