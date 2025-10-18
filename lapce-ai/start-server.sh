#!/bin/bash
# Lapce AI IPC Server Startup Script
# Usage: ./start-server.sh [--openai KEY | --anthropic KEY | --gemini KEY | --env FILE]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Lapce AI IPC Server - Startup Script${NC}"
echo "======================================"
echo ""

# Change to script directory
cd "$(dirname "$0")"

# Check if binary exists
if [ ! -f "./target/debug/lapce_ipc_server" ]; then
    echo -e "${RED}Error: Binary not found at ./target/debug/lapce_ipc_server${NC}"
    echo "Please build the binary first:"
    echo "  cargo build --bin lapce_ipc_server"
    exit 1
fi

# Parse command line arguments
if [ $# -eq 0 ]; then
    echo -e "${YELLOW}No API key provided. Checking environment...${NC}"
    echo ""
    
    # Check for existing environment variables
    FOUND_KEY=0
    
    if [ -n "$OPENAI_API_KEY" ]; then
        echo -e "${GREEN}✓ Found OPENAI_API_KEY${NC}"
        FOUND_KEY=1
    fi
    
    if [ -n "$ANTHROPIC_API_KEY" ]; then
        echo -e "${GREEN}✓ Found ANTHROPIC_API_KEY${NC}"
        FOUND_KEY=1
    fi
    
    if [ -n "$GEMINI_API_KEY" ]; then
        echo -e "${GREEN}✓ Found GEMINI_API_KEY${NC}"
        FOUND_KEY=1
    fi
    
    if [ -n "$XAI_API_KEY" ]; then
        echo -e "${GREEN}✓ Found XAI_API_KEY${NC}"
        FOUND_KEY=1
    fi
    
    if [ $FOUND_KEY -eq 0 ]; then
        echo -e "${RED}No API keys found in environment!${NC}"
        echo ""
        echo "Usage:"
        echo "  $0 --openai sk-your-key-here"
        echo "  $0 --anthropic sk-ant-your-key-here"
        echo "  $0 --gemini your-key-here"
        echo "  $0 --env .env"
        echo ""
        echo "Or set environment variable manually:"
        echo "  export OPENAI_API_KEY='sk-your-key-here'"
        echo "  $0"
        exit 1
    fi
else
    # Parse arguments
    case "$1" in
        --openai)
            export OPENAI_API_KEY="$2"
            echo -e "${GREEN}Using OpenAI API key${NC}"
            ;;
        --anthropic)
            export ANTHROPIC_API_KEY="$2"
            echo -e "${GREEN}Using Anthropic API key${NC}"
            ;;
        --gemini)
            export GEMINI_API_KEY="$2"
            echo -e "${GREEN}Using Gemini API key${NC}"
            ;;
        --xai)
            export XAI_API_KEY="$2"
            echo -e "${GREEN}Using xAI API key${NC}"
            ;;
        --env)
            if [ ! -f "$2" ]; then
                echo -e "${RED}Error: Environment file not found: $2${NC}"
                exit 1
            fi
            source "$2"
            echo -e "${GREEN}Loaded environment from: $2${NC}"
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Usage: $0 [--openai KEY | --anthropic KEY | --gemini KEY | --env FILE]"
            exit 1
            ;;
    esac
fi

echo ""
echo "Starting IPC Server..."
echo "Socket: /tmp/lapce-ai.sock"
echo "Press Ctrl+C to stop"
echo ""

# Clean up old socket if exists
if [ -S "/tmp/lapce-ai.sock" ]; then
    echo -e "${YELLOW}Removing old socket file...${NC}"
    rm -f /tmp/lapce-ai.sock
fi

# Start the server
exec ./target/debug/lapce_ipc_server
