#!/bin/bash
# Quick Start Script for Lapce AI Backend
# This starts the IPC server that the UI connects to

set -e

cd "$(dirname "$0")"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║      🚀 Lapce AI Backend Startup            ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════╝${NC}"
echo ""

# Check if API key is set
if [ -z "$GEMINI_API_KEY" ] && [ -z "$OPENAI_API_KEY" ] && [ -z "$ANTHROPIC_API_KEY" ]; then
    echo -e "${YELLOW}⚠️  No API keys found in environment${NC}"
    echo ""
    echo "You need at least one API key. Set one of:"
    echo -e "  ${GREEN}export GEMINI_API_KEY='your-key-here'${NC}"
    echo -e "  ${GREEN}export OPENAI_API_KEY='sk-...'${NC}"
    echo -e "  ${GREEN}export ANTHROPIC_API_KEY='sk-ant-...'${NC}"
    echo ""
    echo -e "${YELLOW}Using test mode (messages won't get real AI responses)${NC}"
    echo ""
    export GEMINI_API_KEY="test-key-for-development"
fi

# Show which providers are available
echo -e "${GREEN}📡 Provider Status:${NC}"
[ ! -z "$GEMINI_API_KEY" ] && echo -e "  ${GREEN}✓${NC} Gemini (Google)"
[ ! -z "$OPENAI_API_KEY" ] && echo -e "  ${GREEN}✓${NC} OpenAI (GPT-4)"
[ ! -z "$ANTHROPIC_API_KEY" ] && echo -e "  ${GREEN}✓${NC} Anthropic (Claude)"
[ ! -z "$XAI_API_KEY" ] && echo -e "  ${GREEN}✓${NC} xAI (Grok)"
echo ""

# Clean up old socket
if [ -f /tmp/lapce_ai.sock ]; then
    echo -e "${YELLOW}🧹 Cleaning old socket...${NC}"
    rm -f /tmp/lapce_ai.sock
fi

# Check if binary exists
if [ ! -f ./target/debug/lapce_ipc_server ]; then
    echo -e "${RED}❌ Server binary not found!${NC}"
    echo ""
    echo "Building server..."
    cargo build --bin lapce_ipc_server
    echo ""
fi

echo -e "${GREEN}✓ Server binary ready${NC}"
echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}🎯 Starting IPC Server...${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "Socket:  ${YELLOW}/tmp/lapce_ai.sock${NC}"
echo -e "Metrics: ${YELLOW}http://localhost:9090${NC}"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop${NC}"
echo ""

# Start server
exec ./target/debug/lapce_ipc_server
