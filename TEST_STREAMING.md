# Test Streaming with Gemini API

## Setup Complete ✅

1. **Backend Handler Registered** - `lapce_ipc_server.rs` now has provider streaming handler
2. **Message Types Added** - `MessageType::ProviderChatStream` = 11
3. **API Key Configured** - `lapce-ai/.env` has `GEMINI_API_KEY`

## Test Steps

### 1. Start Backend Server

```bash
cd lapce-ai
cargo run --bin lapce_ipc_server
```

**Expected Output:**
```
[INFO] Starting Lapce IPC Server
[INFO] Configuration loaded from: lapce-ipc.toml
✓ Loaded 1 AI provider(s) from environment
  - gemini
[INFO] Provider manager initialized with 1 providers
[INFO] IPC server created at: /tmp/lapce-ai.sock
[INFO] Provider streaming handler registered
[INFO] Starting IPC server...
```

### 2. Launch Lapce

```bash
cd lapce-app
cargo run
```

### 3. Open AI Chat Panel

- Click AI icon in sidebar (or Ctrl+Shift+A)
- Should see Windsurf-style chat interface

### 4. Send Test Message

Type: **"Tell me a short joke"**

Press Enter or click Send button

### Expected Behavior

1. **User message appears immediately** in chat
2. **Streaming text appears** below in real-time:
   ```
   [AI message with "Thought for 3s" header]
   Why did the programmer quit...
   ```
3. **Text streams character-by-character** (60fps updates)
4. **When complete**, streaming text moves to message history
5. **Backend logs**:
   ```
   [Provider] Streaming chat request: model=gemini-pro, 1 messages
   [Provider] Stream chunk: "Why did"
   [Provider] Stream chunk: " the programmer"
   ...
   [AI Chat] Stream complete - tokens: 10 prompt + 50 completion = 60 total
   ```

## Troubleshooting

### Backend doesn't start
- Check: `cargo build --bin lapce_ipc_server` compiles successfully
- Check: Socket path `/tmp/lapce-ai.sock` is writable
- Check: No other process using the socket

### UI doesn't connect
- Check: Backend is running
- Check: UI logs show `[SHM_TRANSPORT] Connecting to: /tmp/lapce-ai.sock`
- Check: `[SHM_TRANSPORT] Connected via real IPC` appears

### No streaming response
- Check: Backend logs show `[Provider] Streaming chat request`
- Check: Gemini API key is valid (test with curl)
- Check: Network connection is working
- Check: UI polling loop is running (look for periodic `poll_messages()` activity)

### Test Gemini API directly

```bash
curl -X POST \
  "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key=YOUR_API_KEY" \
  -H 'Content-Type: application/json' \
  -d '{"contents":[{"parts":[{"text":"Tell me a joke"}]}]}'
```

Should return JSON with generated text.

## Debug Mode

Enable verbose logging:

```bash
# Backend
RUST_LOG=lapce_ai_rust=debug cargo run --bin lapce_ipc_server

# UI
RUST_LOG=lapce=debug cargo run
```

## Success Criteria

- [x] Backend compiles and starts
- [ ] UI connects to backend via IPC
- [ ] User message appears in UI
- [ ] Backend receives ProviderChatStreamRequest
- [ ] Gemini API responds with streaming
- [ ] UI displays streaming text live
- [ ] Streaming completes and moves to history
- [ ] Token usage logged

## Next Steps After Success

1. **Test with longer responses** - "Write a 500-word essay about Rust"
2. **Test multiple messages** - Send several in a row
3. **Test error handling** - Invalid API key, network failure
4. **Test other models** - Add OpenAI/Anthropic keys and switch models
5. **Performance profiling** - Check latency and memory usage
