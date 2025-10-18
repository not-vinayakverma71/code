# 🔧 AI Chat Panel - Troubleshooting Guide

## 🚨 Issue: No Reply When Sending Messages

**Symptom**: You send a message in the AI Chat panel, but get no response.

**Root Cause**: The **backend IPC server is not running**!

---

## ✅ Quick Fix (2 Steps)

### Step 1: Start the Backend Server

Open a **new terminal** and run:

```bash
cd /home/verma/lapce/lapce-ai
./START_BACKEND.sh
```

You should see:
```
╔══════════════════════════════════════════════╗
║      🚀 Lapce AI Backend Startup            ║
╚══════════════════════════════════════════════╝

📡 Provider Status:
  ✓ Gemini (Google)

✓ Server binary ready

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎯 Starting IPC Server...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Socket:  /tmp/lapce_ai.sock
Metrics: http://localhost:9090

Press Ctrl+C to stop

[ACCEPT] Waiting for connection...
```

**Keep this terminal open!** The server must stay running.

---

### Step 2: Reload Lapce (or Just Try Again)

The UI will automatically reconnect. Just send another message!

---

## 🔍 Detailed Diagnosis

### Check 1: Is the Backend Running?

```bash
ps aux | grep lapce_ipc_server | grep -v grep
```

**Expected**: Should show a process  
**If empty**: Backend is not running → Start it with `./START_BACKEND.sh`

---

### Check 2: Does the Socket Exist?

```bash
ls -lh /tmp/lapce_ai.sock
```

**Expected**: 
```
srwxrwxr-x 1 verma verma 0 Oct 18 14:00 /tmp/lapce_ai.sock
```

**If "No such file"**: Backend is not running

---

### Check 3: Can You Connect to the Socket?

```bash
nc -U /tmp/lapce_ai.sock
```

**Expected**: Connection stays open (press Ctrl+C to exit)  
**If "Connection refused"**: Backend crashed or not started

---

## 🎯 API Key Configuration

The backend needs at least **one API key** to work properly.

### Option 1: Environment Variables (Temporary)

```bash
# Google Gemini (recommended for testing)
export GEMINI_API_KEY="your-key-here"

# Or OpenAI
export OPENAI_API_KEY="sk-..."

# Or Anthropic Claude
export ANTHROPIC_API_KEY="sk-ant-..."

# Or xAI Grok
export XAI_API_KEY="xai-..."

# Then start backend
cd /home/verma/lapce/lapce-ai
./START_BACKEND.sh
```

### Option 2: `.env` File (Persistent)

Create `/home/verma/lapce/lapce-ai/.env`:
```bash
# At least one of these:
GEMINI_API_KEY=your-key-here
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
XAI_API_KEY=xai-...
```

The startup script will automatically load it!

---

## 📊 Full System Check

Run this command to check everything:

```bash
cd /home/verma/lapce/lapce-ai
cat << 'EOF' | bash
#!/bin/bash
echo "🔍 Lapce AI System Check"
echo "========================"
echo ""

# 1. Binary exists?
if [ -f ./target/debug/lapce_ipc_server ]; then
    echo "✅ Backend binary: EXISTS"
else
    echo "❌ Backend binary: MISSING"
    echo "   Run: cargo build --bin lapce_ipc_server"
fi

# 2. Backend running?
if ps aux | grep -q "[l]apce_ipc_server"; then
    echo "✅ Backend process: RUNNING"
    ps aux | grep "[l]apce_ipc_server" | awk '{print "   PID: "$2" | Started: "$9}'
else
    echo "❌ Backend process: NOT RUNNING"
    echo "   Run: ./START_BACKEND.sh"
fi

# 3. Socket exists?
if [ -S /tmp/lapce_ai.sock ]; then
    echo "✅ IPC socket: EXISTS"
    ls -lh /tmp/lapce_ai.sock | awk '{print "   "$1" "$3" "$4" "$9}'
else
    echo "❌ IPC socket: MISSING"
    echo "   Backend must be running to create socket"
fi

# 4. API keys configured?
echo ""
echo "🔑 API Keys:"
[ ! -z "$GEMINI_API_KEY" ] && echo "   ✅ GEMINI_API_KEY: SET" || echo "   ❌ GEMINI_API_KEY: NOT SET"
[ ! -z "$OPENAI_API_KEY" ] && echo "   ✅ OPENAI_API_KEY: SET" || echo "   ❌ OPENAI_API_KEY: NOT SET"
[ ! -z "$ANTHROPIC_API_KEY" ] && echo "   ✅ ANTHROPIC_API_KEY: SET" || echo "   ❌ ANTHROPIC_API_KEY: NOT SET"
[ ! -z "$XAI_API_KEY" ] && echo "   ✅ XAI_API_KEY: SET" || echo "   ❌ XAI_API_KEY: NOT SET"

# 5. Lapce running?
echo ""
if ps aux | grep -q "[l]apce"; then
    echo "✅ Lapce: RUNNING"
else
    echo "⚠️  Lapce: NOT RUNNING"
fi

echo ""
echo "========================"
echo "💡 Next Steps:"
echo ""
if ! ps aux | grep -q "[l]apce_ipc_server"; then
    echo "1. Start backend: ./START_BACKEND.sh"
fi
if ! ps aux | grep -q "[l]apce"; then
    echo "2. Start Lapce:   cd /home/verma/lapce && cargo run"
fi
echo "3. Open AI Chat panel in Lapce (right sidebar)"
echo "4. Send a message and watch for response!"
echo ""
EOF
```

---

## 🚀 Complete Startup Sequence

### Terminal 1: Start Backend (leave running)
```bash
cd /home/verma/lapce/lapce-ai

# If you have an API key:
export GEMINI_API_KEY="your-key"

# Start server
./START_BACKEND.sh

# You should see: [ACCEPT] Waiting for connection...
```

### Terminal 2: Start Lapce
```bash
cd /home/verma/lapce
cargo run --release

# Or if already built:
./target/release/lapce
```

### In Lapce UI:
1. Look at **right sidebar**
2. Click **"AI Chat"** tab
3. Type: "Hello! Write a Rust function"
4. Press **Enter**
5. Watch the streaming response! 🎉

---

## 🐛 Common Issues

### Issue: "Connection refused"
**Cause**: Backend not started  
**Fix**: Run `./START_BACKEND.sh` in Terminal 1

### Issue: Backend starts but immediately exits
**Cause**: Configuration error  
**Fix**: Check the error message, might need API key

### Issue: "No API keys found"
**Cause**: No environment variables set  
**Fix**: Export at least one API key (see above)

### Issue: Backend running but UI still no response
**Cause**: Socket path mismatch  
**Fix**: Check both are using `/tmp/lapce_ai.sock`:
```bash
# Backend logs should show:
INFO main ThreadId(01) IPC server created at: /tmp/lapce_ai.sock

# UI logs should show:
[AI Chat] Connecting to backend at /tmp/lapce_ai.sock
```

### Issue: "Provider error: API key invalid"
**Cause**: Wrong API key  
**Fix**: Check your API key is correct and active

---

## 📈 Monitoring the Backend

### View Real-Time Logs
Backend logs appear in Terminal 1 where you started it.

Look for:
```
✅ Client connected
✅ Received ProviderChatStream request
✅ Streaming chunk: "Hello! I..."
✅ Stream complete
```

### Check Metrics
Backend exposes metrics on port 9090:
```bash
curl http://localhost:9090/metrics
```

Shows:
- Active connections
- Messages processed
- Latency stats
- Error rates

---

## 🎯 Success Indicators

### Backend Terminal (Terminal 1)
```
[ACCEPT] Waiting for connection...
INFO Client connected from: Lapce UI
INFO Received: ProviderChatStream
INFO Provider: gemini, Model: gemini-pro
INFO Streaming response...
INFO Stream complete, tokens: 156, duration: 2.3s
```

### Lapce Console (Terminal 2)
```
[AI Chat] Connecting to backend at /tmp/lapce_ai.sock
[SHM_TRANSPORT] Connected successfully
[AI Chat] Sending: Hello! (model: Claude Sonnet 4.5, mode: Code)
[AI Chat] Received chunk: "Hello! I..."
[AI Chat] Stream complete
```

### UI (Lapce Window)
- Message appears in chat (right side)
- AI response streams in (left side)
- "Thought for 2s" header shows
- Response text appears word-by-word
- Feedback buttons (👍👎) visible

---

## 🔥 Nuclear Option (Complete Reset)

If nothing works, try this:

```bash
# 1. Kill everything
pkill -9 lapce_ipc_server
pkill -9 lapce

# 2. Clean sockets
rm -f /tmp/lapce_ai.sock

# 3. Rebuild backend
cd /home/verma/lapce/lapce-ai
cargo clean
cargo build --bin lapce_ipc_server

# 4. Start fresh
export GEMINI_API_KEY="your-key"
./START_BACKEND.sh

# 5. In another terminal, start Lapce
cd /home/verma/lapce
cargo run --release
```

---

## 📞 Quick Reference

| Action | Command |
|--------|---------|
| Start backend | `cd lapce-ai && ./START_BACKEND.sh` |
| Check if running | `ps aux \| grep lapce_ipc_server` |
| Check socket | `ls -lh /tmp/lapce_ai.sock` |
| View logs | Terminal 1 (where backend runs) |
| Stop backend | Press Ctrl+C in Terminal 1 |
| Build backend | `cargo build --bin lapce_ipc_server` |
| Set API key | `export GEMINI_API_KEY="..."` |

---

## 🎉 TL;DR (Too Long; Didn't Read)

**Problem**: No AI response in chat panel  
**Cause**: Backend server not running  
**Fix**:

```bash
# Terminal 1 - Start backend (leave open)
cd /home/verma/lapce/lapce-ai
export GEMINI_API_KEY="your-key-here"
./START_BACKEND.sh

# Terminal 2 - Start Lapce
cd /home/verma/lapce
cargo run --release

# In Lapce: Right sidebar → AI Chat → Send message → Get response! 🚀
```

---

**Last Updated**: 2025-10-18 14:00 IST  
**Status**: Ready to fix!  
**Next**: Start the backend with `./START_BACKEND.sh` 🚀
