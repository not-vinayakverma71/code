# 💬 How to Use AI Chat Panel

## Current Status

### ❌ Backend Auto-Start
**No**, the backend does NOT auto-start. You must run it manually (for now).

### ✅ Panel Location
**Yes**, the AI Chat panel **IS** connected and ready to use!
- **Location**: Right sidebar, top section
- **Name**: "AI Chat"
- **Position**: First panel in right sidebar

---

## Quick Start (2 Steps)

### Step 1: Start Backend (Terminal)

```bash
cd /home/verma/lapce/lapce-ai
./run-backend.sh
```

✅ Backend starts and waits for connection at `/tmp/lapce_ai.sock`

### Step 2: Launch Lapce & Click Panel

```bash
cd /home/verma/lapce
cargo run --release
```

**Then in Lapce UI:**
1. Look at **right sidebar** (right edge of window)
2. Find **"AI Chat"** panel icon (should be first in top section)
3. **Click** it to open
4. Type your message in the input box
5. Press **Enter** or click **Send**
6. Watch Gemini respond in real-time! 🎉

---

## Visual Guide

```
┌──────────────────────────────────────────────────────────┐
│  Lapce Editor Window                                     │
│                                                           │
│  ┌────────────────────────┐  ┌─────────────────────┐    │
│  │                        │  │  Right Sidebar      │    │
│  │   Code Editor          │  │                     │    │
│  │   (main area)          │  │  ┌──────────────┐  │    │
│  │                        │  │  │ 🤖 AI Chat   │◄─┼────┼─ CLICK HERE!
│  │                        │  │  └──────────────┘  │    │
│  │                        │  │                     │    │
│  │                        │  │  ┌──────────────┐  │    │
│  │                        │  │  │ 📄 Doc Symbol│  │    │
│  │                        │  │  └──────────────┘  │    │
│  └────────────────────────┘  └─────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

---

## Panel Features

### What You Can Do
- ✅ **Send messages** to Gemini AI
- ✅ **See streaming responses** (text appears as it's generated)
- ✅ **View message history** (scrollable conversation)
- ✅ **Select AI model** (dropdown in panel header)
- ✅ **Copy responses** (click copy icon)

### Current Limitations
- ⏳ **No auto-start**: Must start backend manually
- ⏳ **Basic UI**: Windsurf-style components (functional, not polished)
- ⏳ **Gemini only**: Your API key is for Gemini (OpenAI/Anthropic need different keys)

---

## Enable Auto-Start (Optional)

If you want the backend to start automatically when you login:

```bash
cd /home/verma/lapce/lapce-ai
./setup-autostart.sh
```

This will:
1. Create `.env` file with your API key
2. Add desktop autostart entry
3. Backend runs in background on every login

**After setup**:
- ✅ Backend starts when you login
- ✅ Just launch Lapce and use AI Chat
- ✅ No manual terminal commands needed

**To disable**:
```bash
rm ~/.config/autostart/lapce-ai.desktop
```

---

## Troubleshooting

### "Panel shows but no responses"
**Check**: Is backend running?
```bash
pgrep -f lapce_ipc_server
# If no output → backend not running
# Solution: ./run-backend.sh
```

### "Can't find AI Chat panel"
**Location**: Right sidebar, top section
**Look for**: Extensions icon (📦) labeled "AI Chat"
**Try**: Click each icon in right sidebar to find it

### "Connection error" in panel
**Check**: Socket file exists
```bash
ls -l /tmp/lapce_ai.sock
# Should show: srwxrwxr-x
```

**Fix**: Restart backend
```bash
pkill -f lapce_ipc_server
cd /home/verma/lapce/lapce-ai
./run-backend.sh
```

### "Backend says 'No API key'"
**Fix**: Set your key in .env
```bash
cd /home/verma/lapce/lapce-ai
echo 'GEMINI_API_KEY=AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU' > .env
```

---

## Example Conversation

**You type:**
```
Hello! Can you help me understand Rust ownership?
```

**Gemini responds (streaming):**
```
Hello! I'd be happy to explain Rust ownership.

Rust's ownership system is one of its most distinctive features...
[text appears in real-time as it's generated]
```

**You type:**
```
Can you give me a code example?
```

**Gemini responds:**
```rust
// Example of ownership in Rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1;  // s1 is moved to s2
    
    // println!("{}", s1);  // Error! s1 no longer valid
    println!("{}", s2);  // OK! s2 owns the string
}
```

---

## Summary

| Question | Answer |
|----------|--------|
| **Auto-start?** | ❌ No (manual start required) |
| **Panel connected?** | ✅ Yes (right sidebar, click to open) |
| **Where is it?** | Right sidebar → "AI Chat" icon |
| **How to enable auto-start?** | Run `./setup-autostart.sh` |
| **Which AI?** | Gemini (your API key) |
| **Streaming?** | ✅ Yes (real-time responses) |

---

## Next Steps

1. **Start backend**: `cd /home/verma/lapce/lapce-ai && ./run-backend.sh`
2. **Launch Lapce**: `cd /home/verma/lapce && cargo run --release`
3. **Open AI Chat**: Click panel icon in right sidebar
4. **Start chatting**: Type message and press Enter!

**Optional**: Run `./setup-autostart.sh` to enable auto-start on login

---

**Ready to chat with AI!** 🚀
