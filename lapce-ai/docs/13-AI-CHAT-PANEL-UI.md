# Step 22: AI Chat Panel UI Integration
## Exact Codex Chat Panel but  in Lapce Editor

## ⚠️ CRITICAL: 1:1 PORT OF CODEX UI TO LAPCE
**EXACT SAME CHAT EXPERIENCE - NO CHANGES** - The UI/UX needed to be integrated into Lapce IDE -  `/home/verma/lapce/lapce-app`

**TRANSLATE LINE-BY-LINE FROM**:
- `/home/verma/lapce/Codex/` - There are thousands of files that needs pure translation 
- Same layout, same styling, same interactions
- Years of UX refinement - PRESERVE ALL


## ✅ Success Criteria
- [ ] **UI Position**: Right side panel (same as Codex)
- [ ] **Toggle Shortcut**: Cmd+Shift+L (same as Codex)
- [ ] **Memory Usage**: < 5MB for UI components
- [ ] **Render Speed**: < 16ms per frame (60 FPS)
- [ ] **Message Streaming**: Real-time token display
- [ ] **Code Blocks**: Syntax highlighting + copy button
- [ ] **Markdown Support**: Full GFM rendering
- [ ] **Chat History**: Persist across sessions

## Overview
Port the Codex AI chat panel to Lapce's Floem UI framework, maintaining exact same UX.

## UI Structure (FROM CODEX)

### Chat Panel Layout
```rust
// Translate from Codex TypeScript to Lapce Floem
pub struct AIChatPanel {
    // Panel container - 400px default width
    container: Container,
    
    // Header bar with model selector
    header: ChatHeader,
    
    // Message list with virtual scrolling
    messages: MessageList,
    
    // Input area with multiline support
    input: ChatInput,
    
    // Token counter and status
    status_bar: StatusBar,
}
```

### Codex UI Elements to Port

#### 1. Header Bar
```typescript
// From Codex: packages/ui/src/chat/ChatHeader.tsx
// Model selector dropdown
// Settings icon
// Clear chat button
// Minimize/maximize toggle
```

#### 2. Message List
```typescript
// From Codex: packages/ui/src/chat/MessageList.tsx
// User messages (right aligned, blue)
// Assistant messages (left aligned, gray)
// Code blocks with syntax highlighting
// Copy button on hover
// Markdown rendering
// Streaming token animation
```

#### 3. Input Area
```typescript
// From Codex: packages/ui/src/chat/ChatInput.tsx
// Auto-expanding textarea
// Shift+Enter for new line
// Enter to send
// File attachment support
// @mentions for context
```

## Lapce Floem Implementation

### Panel Registration
```rust
// lapce-app/src/panel/ai_chat.rs
use floem::{
    reactive::*,
    view::*,
    widgets::*,
};

pub struct AIChatPanelData {
    messages: RwSignal<Vec<ChatMessage>>,
    input_text: RwSignal<String>,
    selected_model: RwSignal<String>,
    is_streaming: RwSignal<bool>,
}

pub fn ai_chat_panel(
    window_tab_data: Rc<WindowTabData>,
) -> impl View {
    let panel_data = AIChatPanelData::new();
    
    container(
        stack((
            // Header
            chat_header(&panel_data),
            
            // Messages
            virtual_list(
                move || panel_data.messages.get(),
                move |msg| msg.id.clone(),
                move |msg| message_item(msg),
            )
            .style(|s| s
                .flex_grow(1.0)
                .padding(10)
            ),
            
            // Input
            chat_input(&panel_data),
        ))
        .style(|s| s
            .flex_col()
            .width_px(400)
            .height_pct(100.0)
        )
    )
}
```

### Message Rendering
```rust
fn message_item(msg: ChatMessage) -> impl View {
    let is_user = msg.role == MessageRole::User;
    
    container(
        stack((
            // Avatar
            label(if is_user { "You" } else { "AI" })
                .style(|s| s
                    .width_px(32)
                    .height_px(32)
                    .border_radius(16)
                ),
            
            // Message content
            if msg.content_type == ContentType::Code {
                code_block(msg.content, msg.language)
            } else {
                markdown_view(msg.content)
            }
            .style(|s| s.flex_grow(1.0)),
        ))
        .style(move |s| {
            s.flex_row()
             .padding(8)
             .margin_bottom(8)
             .background(if is_user {
                 Color::rgb8(59, 130, 246) // Blue
             } else {
                 Color::rgb8(75, 85, 99) // Gray
             })
             .border_radius(8)
             .align_items(if is_user {
                 AlignItems::End
             } else {
                 AlignItems::Start
             })
        })
    )
}
```

### Streaming Support
```rust
pub async fn handle_streaming_response(
    response: Stream<Token>,
    message_signal: RwSignal<String>,
) {
    let mut buffer = String::new();
    
    while let Some(token) = response.next().await {
        buffer.push_str(&token.text);
        
        // Update UI in real-time
        message_signal.set(buffer.clone());
        
        // 60 FPS update rate
        tokio::time::sleep(Duration::from_millis(16)).await;
    }
}
```

### Code Block Component
```rust
fn code_block(code: String, language: String) -> impl View {
    let copied = create_rw_signal(false);
    
    container(
        stack((
            // Language label
            label(&language)
                .style(|s| s
                    .font_size(12)
                    .color(Color::rgb8(156, 163, 175))
                ),
            
            // Code with syntax highlighting
            code_view(&code, &language)
                .style(|s| s
                    .font_family("JetBrains Mono")
                    .font_size(13)
                ),
            
            // Copy button
            button("Copy")
                .on_click(move |_| {
                    copy_to_clipboard(&code);
                    copied.set(true);
                })
                .style(|s| s
                    .position_absolute()
                    .top_px(8)
                    .right_px(8)
                ),
        ))
        .style(|s| s
            .background(Color::rgb8(31, 41, 55))
            .border_radius(6)
            .padding(12)
        )
    )
}
```

### Integration Points

#### 1. Connect to AI Backend
```rust
// Connect to Phase 4 AI providers
use lapce_ai_rust::providers::{
    OpenAIProvider,
    AnthropicProvider,
    CompletionRequest,
};

async fn send_message(
    content: String,
    provider: Arc<dyn Provider>,
) -> Result<Stream<Token>> {
    let request = CompletionRequest {
        messages: vec![Message::user(content)],
        model: "gpt-4".to_string(),
        stream: true,
    };
    
    provider.complete_streaming(request).await
}
```

#### 2. Keyboard Shortcuts
```rust
// lapce-app/src/keymap/default.rs
commands.insert(
    "ai.toggle_chat",
    vec![KeyBinding {
        key: vec![Key::Cmd, Key::Shift, Key::L],
        when: None,
    }]
);
```

#### 3. Panel Registration
```rust
// lapce-app/src/panel/mod.rs
pub enum PanelKind {
    Terminal,
    FileExplorer,
    Search,
    SourceControl,
    AIChat, // Add this
}

impl PanelKind {
    pub fn view(&self) -> Box<dyn View> {
        match self {
            PanelKind::AIChat => Box::new(ai_chat_panel()),
            // ... other panels
        }
    }
}
```

## Testing Requirements

```rust
#[test]
fn test_chat_ui_renders() {
    let panel = ai_chat_panel(test_data());
    assert!(panel.layout().width == 400);
}

#[test]
fn test_message_streaming() {
    // Verify 60 FPS updates
    // Test token-by-token display
}

#[test]
fn test_code_block_copy() {
    // Test syntax highlighting
    // Test copy functionality
}
```

## Implementation Checklist
- [ ] Add all UI/UX from Codex - Thousand of files, Just Rust Syntax
- [ ] Port ChatHeader component from Codex
- [ ] Port MessageList with virtual scrolling
- [ ] Port ChatInput with multiline support
- [ ] Implement streaming token display
- [ ] Add syntax highlighting for code blocks
- [ ] Implement markdown rendering
- [ ] Add copy button for code blocks
- [ ] Connect to AI backend (Phase 4)
- [ ] Add keyboard shortcuts
- [ ] Persist chat history
- [ ] Test 60 FPS performance
