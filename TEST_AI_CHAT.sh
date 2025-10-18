#!/bin/bash
# Quick test script to verify AI Chat is working

cd "$(dirname "$0")"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   🧪 AI Chat System Test                  ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
echo ""

# Test 1: Binary exists
echo -n "1. Backend binary...          "
if [ -f lapce-ai/target/debug/lapce_ipc_server ]; then
    echo -e "${GREEN}✓ EXISTS${NC}"
    BINARY_OK=1
else
    echo -e "${RED}✗ MISSING${NC}"
    echo "   Build with: cd lapce-ai && cargo build --bin lapce_ipc_server"
    BINARY_OK=0
fi

# Test 2: Backend running
echo -n "2. Backend process...         "
if ps aux | grep -q "[l]apce_ipc_server"; then
    echo -e "${GREEN}✓ RUNNING${NC}"
    BACKEND_OK=1
else
    echo -e "${RED}✗ NOT RUNNING${NC}"
    echo "   Start with: cd lapce-ai && ./START_BACKEND.sh"
    BACKEND_OK=0
fi

# Test 3: Socket exists
echo -n "3. IPC socket...              "
if [ -S /tmp/lapce_ai.sock ]; then
    echo -e "${GREEN}✓ EXISTS${NC}"
    SOCKET_OK=1
else
    echo -e "${RED}✗ MISSING${NC}"
    echo "   Backend must be running"
    SOCKET_OK=0
fi

# Test 4: API keys
echo -n "4. API keys...                "
if [ ! -z "$GEMINI_API_KEY" ] || [ ! -z "$OPENAI_API_KEY" ] || [ ! -z "$ANTHROPIC_API_KEY" ]; then
    echo -e "${GREEN}✓ CONFIGURED${NC}"
    APIKEY_OK=1
else
    echo -e "${YELLOW}⚠ NOT SET (will use test mode)${NC}"
    APIKEY_OK=0
fi

# Test 5: Lapce binary
echo -n "5. Lapce binary...            "
if [ -f target/release/lapce ] || [ -f target/debug/lapce ]; then
    echo -e "${GREEN}✓ EXISTS${NC}"
    LAPCE_OK=1
else
    echo -e "${YELLOW}⚠ NOT BUILT${NC}"
    echo "   Build with: cargo build --release"
    LAPCE_OK=0
fi

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Results:${NC}"
echo ""

TOTAL_TESTS=5
PASSED_TESTS=$((BINARY_OK + BACKEND_OK + SOCKET_OK + APIKEY_OK + LAPCE_OK))

if [ $BACKEND_OK -eq 1 ] && [ $SOCKET_OK -eq 1 ]; then
    echo -e "${GREEN}✅ System Status: READY${NC}"
    echo ""
    echo "Everything looks good! The AI Chat should work."
    echo ""
    echo "📝 To test:"
    echo "   1. Launch Lapce (if not running): cargo run --release"
    echo "   2. Open AI Chat panel (right sidebar)"
    echo "   3. Send a message"
    echo "   4. Watch for streaming response!"
    echo ""
elif [ $BINARY_OK -eq 1 ] && [ $BACKEND_OK -eq 0 ]; then
    echo -e "${YELLOW}⚠️  System Status: BACKEND NOT RUNNING${NC}"
    echo ""
    echo "The backend server is not running."
    echo ""
    echo "🔧 Quick fix:"
    echo ""
    echo "   Terminal 1 - Start backend:"
    echo "   ────────────────────────────"
    echo "   cd lapce-ai"
    
    if [ $APIKEY_OK -eq 0 ]; then
        echo "   export GEMINI_API_KEY='your-key-here'  # Add your API key"
    fi
    
    echo "   ./START_BACKEND.sh"
    echo ""
    echo "   Terminal 2 - Start Lapce:"
    echo "   ─────────────────────────"
    echo "   cd /home/verma/lapce"
    echo "   cargo run --release"
    echo ""
else
    echo -e "${RED}❌ System Status: NOT READY${NC}"
    echo ""
    echo "Some components are missing. Please:"
    echo ""
    
    if [ $BINARY_OK -eq 0 ]; then
        echo "   1. Build backend:"
        echo "      cd lapce-ai && cargo build --bin lapce_ipc_server"
        echo ""
    fi
    
    if [ $LAPCE_OK -eq 0 ]; then
        echo "   2. Build Lapce:"
        echo "      cargo build --release"
        echo ""
    fi
    
    if [ $APIKEY_OK -eq 0 ]; then
        echo "   3. Set API key (optional):"
        echo "      export GEMINI_API_KEY='your-key'"
        echo ""
    fi
    
    echo "   4. Start backend:"
    echo "      cd lapce-ai && ./START_BACKEND.sh"
    echo ""
    echo "   5. Start Lapce:"
    echo "      cargo run --release"
    echo ""
fi

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Show detailed info if backend is running
if [ $BACKEND_OK -eq 1 ]; then
    echo -e "${GREEN}Backend Details:${NC}"
    ps aux | grep "[l]apce_ipc_server" | awk '{print "  PID: "$2" | CPU: "$3"% | MEM: "$4"% | Started: "$9}'
    echo ""
fi

# Exit code reflects readiness
if [ $BACKEND_OK -eq 1 ] && [ $SOCKET_OK -eq 1 ]; then
    exit 0
else
    exit 1
fi
