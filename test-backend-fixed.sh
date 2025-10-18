#!/bin/bash
# Quick test to verify the backend fix

cd "$(dirname "$0")"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo ""
echo "🔧 Testing Backend Fix..."
echo "========================"
echo ""

# Kill old backend
echo -n "1. Cleaning up old backend...     "
pkill -9 lapce_ipc_server 2>/dev/null
sleep 1
echo -e "${GREEN}✓${NC}"

# Clean old sockets
echo -n "2. Cleaning old sockets...        "
rm -f /tmp/lapce_ai.sock.ctl 2>/dev/null
rm -rf /tmp/lapce_ai.sock_locks 2>/dev/null
echo -e "${GREEN}✓${NC}"

# Start new backend in background
echo -n "3. Starting new backend...        "
cd lapce-ai
export GEMINI_API_KEY="test-key"
./target/debug/lapce_ipc_server > /tmp/backend-test.log 2>&1 &
BACKEND_PID=$!
cd ..
sleep 2
echo -e "${GREEN}✓ PID: $BACKEND_PID${NC}"

# Check if control socket was created
echo -n "4. Checking control socket...     "
if [ -S /tmp/lapce_ai.sock.ctl ]; then
    echo -e "${GREEN}✓ CREATED!${NC}"
    ls -lh /tmp/lapce_ai.sock.ctl
else
    echo -e "${RED}✗ NOT FOUND${NC}"
    echo ""
    echo "Backend logs:"
    tail -20 /tmp/backend-test.log
    kill $BACKEND_PID 2>/dev/null
    exit 1
fi

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ SUCCESS! Backend is working correctly!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Backend is running with PID: $BACKEND_PID"
echo "Control socket: /tmp/lapce_ai.sock.ctl"
echo ""
echo "You can now:"
echo "  1. Launch Lapce: cd /home/verma/lapce && ./target/release/lapce"
echo "  2. Open AI Chat panel (right sidebar)"
echo "  3. Send a message"
echo "  4. Get AI response! 🎉"
echo ""
echo -n "Stop backend? (y/n): "
read -r RESPONSE

if [ "$RESPONSE" = "y" ] || [ "$RESPONSE" = "Y" ]; then
    kill $BACKEND_PID
    echo "Backend stopped"
else
    echo "Backend still running (PID: $BACKEND_PID)"
    echo "To stop: kill $BACKEND_PID"
fi

echo ""
