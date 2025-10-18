# 🚀 Backend Auto-Start Options

**Current Status**: ❌ Backend does NOT auto-start (manual start required)

---

## What You Have Now

### ✅ Panel Integration
- **Location**: Right sidebar, top section (first panel)
- **Panel Name**: "AI Chat"
- **File**: `/home/verma/lapce/lapce-app/src/panel/ai_chat_view.rs`
- **Status**: ✅ Fully wired and ready to use

### ❌ Backend Auto-Start
- **Current**: Backend must be started manually each time
- **Commands**: 
  ```bash
  cd /home/verma/lapce/lapce-ai
  export GEMINI_API_KEY="AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU"
  ./run-backend.sh
  ```

---

## Auto-Start Options

### Option 1: Systemd Service (Recommended for Linux)

**Create service file**: `/etc/systemd/system/lapce-ai.service`

```ini
[Unit]
Description=Lapce AI IPC Server
After=network.target

[Service]
Type=simple
User=verma
WorkingDirectory=/home/verma/lapce/lapce-ai
Environment="GEMINI_API_KEY=AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU"
ExecStart=/home/verma/lapce/lapce-ai/target/debug/lapce_ipc_server
Restart=always
RestartSec=3
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```

**Enable and start**:
```bash
# Install service (run once)
sudo cp /home/verma/lapce/lapce-ai/lapce-ai.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable lapce-ai

# Start service
sudo systemctl start lapce-ai

# Check status
sudo systemctl status lapce-ai

# View logs
journalctl -u lapce-ai -f
```

**Benefits**:
- ✅ Starts automatically on system boot
- ✅ Auto-restarts if crashes
- ✅ Professional solution
- ✅ Logs managed by systemd

---

### Option 2: Desktop Autostart Entry

**Create**: `~/.config/autostart/lapce-ai.desktop`

```desktop
[Desktop Entry]
Type=Application
Name=Lapce AI Backend
Exec=/home/verma/lapce/lapce-ai/run-backend.sh
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
```

**Setup**:
```bash
# Create autostart directory
mkdir -p ~/.config/autostart

# Create desktop file
cat > ~/.config/autostart/lapce-ai.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=Lapce AI Backend
Exec=/home/verma/lapce/lapce-ai/run-backend.sh
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
EOF

# Make script executable
chmod +x /home/verma/lapce/lapce-ai/run-backend.sh

# Create .env file with API key
echo 'GEMINI_API_KEY=AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU' > /home/verma/lapce/lapce-ai/.env
```

**Update run-backend.sh to use .env**:
```bash
# Add at top of run-backend.sh
if [ -f .env ]; then
    source .env
fi
```

**Benefits**:
- ✅ Starts when you login to desktop
- ✅ No sudo/root required
- ✅ User-level service
- ✅ Easy to disable (delete .desktop file)

---

### Option 3: Shell Profile Auto-Start

**Add to** `~/.bashrc` or `~/.zshrc`:

```bash
# Auto-start Lapce AI backend if not running
if ! pgrep -f lapce_ipc_server > /dev/null; then
    (
        cd /home/verma/lapce/lapce-ai
        export GEMINI_API_KEY="AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU"
        nohup ./target/debug/lapce_ipc_server > /tmp/lapce-ai.log 2>&1 &
    )
    echo "✅ Lapce AI backend started"
fi
```

**Benefits**:
- ✅ Starts on terminal launch
- ✅ Simple to add/remove
- ❌ Only starts when you open a terminal

---

### Option 4: Cron @reboot

**Edit crontab**:
```bash
crontab -e
```

**Add**:
```cron
@reboot cd /home/verma/lapce/lapce-ai && GEMINI_API_KEY="AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU" ./target/debug/lapce_ipc_server >> /tmp/lapce-ai.log 2>&1
```

**Benefits**:
- ✅ Runs on system boot
- ✅ Simple setup
- ❌ Harder to debug (no easy logs)

---

### Option 5: Launch from Lapce App (Future Feature)

**NOT IMPLEMENTED YET** - Would require modifying Lapce app to:
1. Check if backend is running on startup
2. Auto-spawn `lapce_ipc_server` as child process
3. Pass API key from Lapce settings

**Code location** (if implementing):
```rust
// In lapce-app/src/app.rs or main.rs
use std::process::Command;

fn ensure_backend_running() {
    // Check if already running
    if !is_backend_running() {
        // Spawn backend as child process
        Command::new("/path/to/lapce_ipc_server")
            .env("GEMINI_API_KEY", get_api_key_from_config())
            .spawn()
            .expect("Failed to start backend");
    }
}
```

---

## Current Panel Status

### ✅ What's Working Now
```
┌─────────────────────────────────────┐
│  Lapce UI (Right Sidebar)          │
│                                     │
│  📋 [AI Chat]  ← Click here!       │
│  📄 Document Symbol                 │
│                                     │
└─────────────────────────────────────┘
```

**To use the panel RIGHT NOW**:
1. Start backend manually: `./run-backend.sh`
2. Start Lapce: `cargo run --release`
3. Click "AI Chat" icon in right sidebar
4. Type message and press Enter
5. Get Gemini response! 🎉

### Panel Features
- ✅ **Position**: Right sidebar, top section
- ✅ **Default Order**: First panel in right-top position
- ✅ **Icon**: Extensions icon (TODO: add AI-specific icon)
- ✅ **Visibility**: Available but not auto-opened (click to open)
- ✅ **Connection**: Automatically connects to `/tmp/lapce_ai.sock`
- ✅ **Retry Logic**: Auto-reconnects if backend restarts
- ✅ **Streaming**: Real-time text streaming from Gemini

---

## Recommended Setup (Best UX)

**For Development (Current)**:
```bash
# Terminal 1: Backend (manual start, see logs)
cd /home/verma/lapce/lapce-ai
./run-backend.sh

# Terminal 2: Lapce UI
cd /home/verma/lapce
cargo run --release
```

**For Production Use (Systemd)**:
```bash
# One-time setup
sudo systemctl enable lapce-ai
sudo systemctl start lapce-ai

# Daily use - just launch Lapce
cd /home/verma/lapce
cargo run --release
# Backend is already running in background!
```

---

## Quick Commands

### Check if backend is running
```bash
pgrep -f lapce_ipc_server
# If output: PID number → Running
# If no output → Not running
```

### Start backend manually
```bash
cd /home/verma/lapce/lapce-ai
./run-backend.sh
```

### Stop backend
```bash
pkill -f lapce_ipc_server
```

### View backend logs (if using systemd)
```bash
journalctl -u lapce-ai -f
```

---

## Summary

| Method | Auto-Start | Ease | Debug | Recommended |
|--------|-----------|------|-------|-------------|
| **Manual** | ❌ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Development |
| **Systemd** | ✅ Boot | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | Production |
| **Autostart** | ✅ Login | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | Desktop Use |
| **Shell RC** | ✅ Terminal | ⭐⭐⭐ | ⭐⭐ | Quick Hack |
| **Cron** | ✅ Boot | ⭐⭐ | ⭐ | Not Recommended |
| **App Spawn** | ✅ App Start | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Future Feature |

**Current Answer**: No, backend does NOT auto-start. Use Option 2 (Autostart) or Option 1 (Systemd) for automatic startup.

**Panel Answer**: Yes, AI Chat panel is **fully connected** and in the right sidebar! Just click it to open. 🎯
