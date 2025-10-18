#!/bin/bash
# Setup auto-start for Lapce AI backend (Linux desktop)

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}🚀 Lapce AI Backend - Auto-Start Setup${NC}"
echo "========================================"
echo ""

# Check if API key is set
if [ -z "$GEMINI_API_KEY" ]; then
    echo -e "${YELLOW}⚠️  GEMINI_API_KEY not found in environment${NC}"
    echo ""
    read -p "Enter your Gemini API key: " api_key
    export GEMINI_API_KEY="$api_key"
fi

# Create .env file
echo "Creating .env file..."
cat > .env << EOF
# Lapce AI Backend Environment Variables
GEMINI_API_KEY=$GEMINI_API_KEY
EOF
chmod 600 .env
echo -e "${GREEN}✓ Created .env file${NC}"

# Update run-backend.sh to source .env
echo "Updating run-backend.sh..."
if ! grep -q "source .env" run-backend.sh; then
    # Backup
    cp run-backend.sh run-backend.sh.bak
    
    # Add .env sourcing after shebang
    sed -i '2i\
# Load environment from .env if present\
if [ -f "$(dirname "$0")/.env" ]; then\
    source "$(dirname "$0")/.env"\
fi\
' run-backend.sh
    echo -e "${GREEN}✓ Updated run-backend.sh${NC}"
else
    echo -e "${GREEN}✓ run-backend.sh already configured${NC}"
fi

# Create autostart directory
mkdir -p ~/.config/autostart

# Create desktop entry
echo "Creating autostart entry..."
cat > ~/.config/autostart/lapce-ai.desktop << EOF
[Desktop Entry]
Type=Application
Name=Lapce AI Backend
Comment=Auto-start Lapce AI IPC Server
Exec=$(pwd)/run-backend.sh
Path=$(pwd)
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
Terminal=false
EOF

echo -e "${GREEN}✓ Created autostart entry${NC}"
echo ""
echo -e "${GREEN}✅ Setup Complete!${NC}"
echo ""
echo "What happens now:"
echo "  1. Backend will auto-start when you login to desktop"
echo "  2. Socket will be at: /tmp/lapce_ai.sock"
echo "  3. Just launch Lapce and use AI Chat panel!"
echo ""
echo "To verify it's working after next login:"
echo "  pgrep -f lapce_ipc_server"
echo ""
echo "To disable auto-start:"
echo "  rm ~/.config/autostart/lapce-ai.desktop"
echo ""
echo "To test now (manual start):"
echo "  ./run-backend.sh"
