# Quick Start - AI Chat Demo

## Run the Demo

```bash
cd /home/verma/lapce/lapce-app
cargo run --example ai_chat_demo
```

## What You'll See

A window with an AI chat interface:
- Welcome message from mock AI
- Input box at bottom
- Send button (↑)

## Try It

1. **Type**: "hello"
2. **Press**: Enter (or click ↑)
3. **See**: Instant response

4. **Type**: "How do I build Lapce?"
5. **Press**: Enter
6. **See**: Streaming response (character-by-character)

## Features Working

✅ User messages (normal text)  
✅ AI messages (dim text @ 55% opacity)  
✅ Inline code rendering (`` `code` ``)  
✅ Code block rendering (` ```code``` `)  
✅ Streaming responses  
✅ Enter key to send  
✅ Click button to send  
✅ Exact Windsurf styling  

## Mock LLM

- 6 pre-written responses about Lapce/Floem/Rust
- Cycles through responses
- Streaming animation (10ms per character)
- Quick responses for greetings

## Files Created

```
lapce-app/src/
├── ai_theme.rs          # Windsurf colors/fonts
├── ai_chat_widgets.rs   # UI components
├── ai_mock_llm.rs       # Mock LLM ← NEW
├── ai_panel_example.rs  # Complete UI (updated with mock)
└── ai_bridge.rs         # Stub

lapce-app/examples/
└── ai_chat_demo.rs      # Demo app ← NEW
```

## Next: Real AI Backend

When `ai_bridge.rs` is fully implemented:

```rust
// Replace MockLlm with real AI
ai_bridge.send_message(user_text).await;

ai_bridge.on_stream_chunk(|chunk| {
    // Update UI with streaming response
});
```

---

**That's it! The chat panel is ready to test.** 🚀
