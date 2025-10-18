# AI Chat Panel - Testing Guide

## Quick Start

Run the demo:
```bash
cd lapce-app
cargo run --example ai_chat_demo
```

This will open a window with the AI chat panel and mock LLM.

---

## Features

### ✅ Working Features

1. **Chat Interface**
   - Send messages by typing and pressing Enter or clicking ↑ button
   - User messages appear in normal text
   - AI responses appear in **dim text (55% opacity)**
   
2. **Mock LLM Responses**
   - Quick responses for greetings ("hello", "hi", "how are you")
   - Streaming responses (character-by-character) for other queries
   - 6 pre-programmed responses about Lapce, Floem, and Rust

3. **Markdown Rendering**
   - Inline code: `` `code` `` renders with gray background
   - Code blocks: ` ```code``` ` renders with dark background
   - Monospace font for all code

4. **Styling**
   - Exact Windsurf colors and spacing
   - Rounded borders, shadows
   - Hover effects on buttons

---

## Try These Queries

### Quick Responses (instant)
- "hello"
- "hi"  
- "how are you"
- "what can you do"

### Streaming Responses (character-by-character)
- "How do I build Lapce?"
- "Tell me about Floem"
- "What is LSP?"
- "How do I add themes?"
- "Explain the IPC architecture"
- Anything else will cycle through 6 pre-written responses

---

## What to Look For

### Visual Checks
1. **AI text should be dim** (lighter gray than user text)
2. **Inline code** (like `cargo build`) should have:
   - Gray background
   - Rounded corners
   - Monospace font
3. **Code blocks** should have:
   - Dark background (#2b2b2b)
   - Proper padding
   - Syntax: ` ```rust\n...code...\n``` `
4. **Input box** should have:
   - Placeholder "Ask anything (Ctrl+L)"
   - Rounded borders
   - Send button (circular with ↑)

### Behavior Checks
1. Type a message → Press Enter → See streaming response
2. Click send button (↑) → Same result
3. Scroll up/down through messages
4. Try greetings ("hello") → Get instant response
5. Try other queries → See streaming (fast typing effect)

---

## Mock LLM Details

### Implementation
File: `lapce-app/src/ai_mock_llm.rs`

### Response Types

**Quick Responses** (`get_quick_response()`):
- Pattern matching for common greetings
- Returns instantly without streaming

**Streaming Responses** (`stream_response()`):
- Spawns thread to simulate typing
- 10ms delay per character (adjustable)
- Updates message content in real-time
- Cycles through 6 pre-written responses:
  1. Lapce overview + Floem example
  2. Build instructions
  3. Layout widgets
  4. IPC architecture
  5. Theme customization
  6. LSP features

### Customization

Edit `lapce-app/src/ai_mock_llm.rs` to:
- Add more responses to `self.responses` vector
- Change typing speed (line 76: `Duration::from_millis(10)`)
- Add more quick responses patterns

---

## Troubleshooting

### Build Errors

If you see compilation errors:
```bash
cd lapce-app
cargo clean
cargo build --example ai_chat_demo
```

### Runtime Issues

**Message not sending?**
- Check console for errors
- Make sure input text is not empty

**No streaming animation?**
- Streaming uses threads - should work on all platforms
- If too fast, increase delay in `ai_mock_llm.rs:76`

**AI text not dim?**
- Check `ai_theme.rs` - `ai_text_medium` should be `0.55` opacity
- Verify theme is being applied in message view

---

## Next Steps

### After Testing

1. **Wire to Real AI Backend**
   - Replace `MockLlm` with `AiBridge` (when ready)
   - Connect to lapce-ai IPC
   - Stream real AI responses

2. **Add More Features**
   - Copy button (clipboard integration)
   - Model selector dropdown
   - Syntax highlighting for code blocks
   - File attachment support
   - Conversation history

3. **Polish**
   - Better markdown parser (tables, lists, bold/italic)
   - Thinking dots animation
   - Error handling UI
   - Loading states

---

## Code Structure

```
lapce-app/src/
├── ai_theme.rs              # Colors, fonts, spacing from Windsurf
├── ai_chat_widgets.rs       # UI components (inline_code, code_block, chat_message)
├── ai_mock_llm.rs           # Mock LLM with streaming responses
├── ai_panel_example.rs      # Complete chat panel UI
└── lib.rs                   # Module declarations

lapce-app/examples/
└── ai_chat_demo.rs          # Standalone demo app
```

---

## Performance

With mock LLM:
- **Memory**: ~5MB for chat panel
- **CPU**: Minimal (only during streaming)
- **Startup**: Instant

The mock LLM simulates realistic AI behavior without network calls or heavy compute.

---

## Screenshots

*Run the demo to see:*
- Dark theme with dim AI text
- Inline code with gray background
- Code blocks with proper formatting
- Streaming character-by-character responses
- Clean, minimal UI matching Windsurf

---

## Questions?

Check the documentation:
- `AI_CHAT_PANEL_IMPLEMENTATION.md` - Full implementation guide
- `CSS_TO_FLOEM_MAPPINGS.md` - CSS ↔ Floem reference

Or ask in the chat (once it's running)! 😄
