#!/bin/bash
# Quick start script for Lapce AI IPC Server

set -e

cd "$(dirname "$0")"

# Load environment from .env if present
if [ -f .env ]; then
    echo "Loading environment from .env..."
    set -a  # Auto-export all variables
    source .env
    set +a
fi

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}🚀 Starting Lapce AI IPC Server${NC}"
echo "================================"
echo ""

# Check API key
if [ -z "$GEMINI_API_KEY" ]; then
    echo -e "${RED}❌ GEMINI_API_KEY not set${NC}"
    echo "Please run: export GEMINI_API_KEY='your-key-here'"
    exit 1
fi

echo -e "${GREEN}✓ GEMINI_API_KEY found${NC}"

# Clean up old sockets
echo "Cleaning old socket files..."
trash-put /tmp/lapce_ai.sock /tmp/lapce-ai.sock 2>/dev/null || /bin/rm -f /tmp/lapce_ai.sock /tmp/lapce-ai.sock

# Start server
echo ""
echo -e "${YELLOW}Starting server on: /tmp/lapce_ai.sock${NC}"
echo "Press Ctrl+C to stop"
echo "================================"
echo ""

export GEMINI_API_KEY
exec ./target/debug/lapce_ipc_server
