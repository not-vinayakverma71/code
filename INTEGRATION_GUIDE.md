# 🔌 Integration Guide - Wire All Components

**Purpose:** How to integrate all new Windsurf components into `chat_view.rs`

---

## 📍 Current State

`/home/verma/lapce/lapce-app/src/panel/ai_chat/components/chat_view.rs` currently has:
- Basic message list with `dyn_stack`
- Simple welcome screen (old version)
- Text input area at bottom

**Goal:** Replace with our new comprehensive components!

---

## 🎯 Integration Steps

### Step 1: Update Imports

```rust
use crate::{
    config::LapceConfig,
    panel::ai_chat::{
        icons::*,  // Icon library
        components::{
            // New comprehensive components
            message_bubble::{MessageBubbleProps, MessageRole, message_bubble},
            thinking_indicator::thinking_indicator,
            code_block::{CodeBlockProps, code_block},
            welcome_screen_v2::welcome_screen_v2,
            model_selector_v2::{ModelSelectorProps, model_selector_v2, default_models},
            file_attachment_v2::{FileAttachmentProps, file_attachment_list},
            chat_text_area::{ChatTextAreaProps, chat_text_area},
        },
    },
};
```

### Step 2: Add State Signals

```rust
pub fn chat_view(
    props: ChatViewProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let input_value = props.input_value;
    let messages_signal = props.messages_signal;
    
    // NEW: Add these signals
    let is_streaming = create_rw_signal(false);
    let attached_files = create_rw_signal(Vec::new());
    let current_model = create_rw_signal("GPT-4".to_string());
    let model_dropdown_open = create_rw_signal(false);
    
    // ... rest of function
}
```

### Step 3: Replace Welcome Screen

```rust
// OLD:
container(welcome_screen(config))

// NEW:
container(welcome_screen_v2(config))
```

### Step 4: Replace Message Rendering

```rust
// OLD:
dyn_stack(
    move || messages_signal.get(),
    |msg| msg.ts,
    move |msg| {
        let chat_msg = ChatMessage { /* ... */ };
        chat_row(ChatRowProps { /* ... */ }, config)
    },
)

// NEW:
dyn_stack(
    move || messages_signal.get(),
    |msg| msg.ts,
    move |msg| {
        // Convert to new message bubble
        message_bubble(
            MessageBubbleProps {
                role: if msg.is_user {
                    MessageRole::User
                } else {
                    MessageRole::Assistant
                },
                content: msg.content.clone(),
                timestamp: format_timestamp(msg.ts),
                is_streaming: msg.partial,
            },
            config,
        )
    },
)
```

### Step 5: Add Thinking Indicator

```rust
// After message list, before closing scroll container:
container(
    if is_streaming.get() {
        thinking_indicator(Some("Thinking...".to_string()), config)
    } else {
        empty().into_any()
    }
)
```

### Step 6: Add File Attachments Above Input

```rust
v_stack((
    // Messages area (existing)
    container(/* ... */),
    
    // NEW: File attachments
    file_attachment_list(
        FileAttachmentProps {
            attached_files,
        },
        config,
    ),
    
    // Input area (existing but enhanced)
    /* ... */
))
```

### Step 7: Enhance Input Area

```rust
// In the input container, add model selector:
h_stack((
    // Left side: existing buttons
    add_files_button(config),
    code_button(config),
    
    // NEW: Model selector
    model_selector_v2(
        ModelSelectorProps {
            current_model,
            available_models: default_models(),
            is_open: model_dropdown_open,
        },
        config,
    ),
    
    // Spacer
    container(empty()).style(|s| s.flex_grow(1.0)),
    
    // Right side: mic and send
    mic_button(config),
    send_button(props.sending_disabled, props.on_send.clone(), config),
))
```

---

## 📦 Complete Example

Here's the full integrated `chat_view.rs`:

```rust
pub fn chat_view(
    props: ChatViewProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let input_value = props.input_value;
    let messages_signal = props.messages_signal;
    
    // State management
    let is_streaming = create_rw_signal(false);
    let attached_files = create_rw_signal(Vec::new());
    let current_model = create_rw_signal("GPT-4".to_string());
    let model_dropdown_open = create_rw_signal(false);
    
    v_stack((
        // === MESSAGES AREA ===
        container(
            scroll(
                v_stack((
                    // Welcome screen (when empty)
                    container(welcome_screen_v2(config))
                        .style(move |s| {
                            let msgs = messages_signal.get();
                            if msgs.is_empty() {
                                s.width_full().flex_grow(1.0)
                            } else {
                                s.width(0.0).height(0.0)
                            }
                        }),
                    
                    // Message list
                    dyn_stack(
                        move || messages_signal.get(),
                        |msg| msg.ts,
                        move |msg| {
                            message_bubble(
                                MessageBubbleProps {
                                    role: if msg.is_user {
                                        MessageRole::User
                                    } else {
                                        MessageRole::Assistant
                                    },
                                    content: msg.content.clone(),
                                    timestamp: format_timestamp(msg.ts),
                                    is_streaming: msg.partial,
                                },
                                config,
                            )
                        },
                    ),
                    
                    // Thinking indicator
                    container(
                        if is_streaming.get() {
                            thinking_indicator(None, config).into_any()
                        } else {
                            empty().into_any()
                        }
                    ),
                ))
                .style(|s| s.padding(12.0).width_full().flex_col())
            )
            .style(|s| s.flex_grow(1.0).width_full())
        )
        .style(move |s| {
            let cfg = config();
            s.flex_grow(1.0)
                .width_full()
                .background(cfg.color("editor.background"))
        }),
        
        // === FILE ATTACHMENTS ===
        file_attachment_list(
            FileAttachmentProps { attached_files },
            config,
        ),
        
        // === INPUT AREA ===
        container(
            v_stack((
                // Text input
                chat_text_area(
                    ChatTextAreaProps {
                        input_value,
                        sending_disabled: props.sending_disabled,
                        placeholder_text: "Ask anything (Ctrl+L)".to_string(),
                        on_send: props.on_send.clone(),
                    },
                    config,
                ),
                
                // Button row with model selector
                h_stack((
                    add_files_button(attached_files, config),
                    code_button(config),
                    model_selector_v2(
                        ModelSelectorProps {
                            current_model,
                            available_models: default_models(),
                            is_open: model_dropdown_open,
                        },
                        config,
                    ),
                    container(empty()).style(|s| s.flex_grow(1.0)),
                    mic_button(config),
                    send_button(props.sending_disabled, props.on_send, config),
                ))
                .style(|s| s.gap(6.0).items_center()),
            ))
        )
        .style(move |s| {
            let cfg = config();
            s.width_full()
                .border_top(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("panel.background"))
                .padding(6.0)
        }),
    ))
    .style(|s| s.width_full().height_full().flex_col())
}

// Helper function
fn format_timestamp(ts: u64) -> String {
    // Convert timestamp to readable format
    // e.g., "2:34 PM" or "Oct 12, 2:34 PM"
    format!("{}", ts)  // Placeholder
}
```

---

## 🎨 Button Extraction

Since chat_text_area.rs already has the buttons, you can:

**Option A:** Keep buttons in chat_text_area.rs (current)  
**Option B:** Move button row to chat_view.rs for more control

Recommended: **Keep in chat_text_area.rs** for encapsulation.

---

## 🔌 Event Handlers

### File Upload
```rust
fn add_files_button(
    attached_files: RwSignal<Vec<AttachedFile>>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(/* ... */)
        .on_click_stop(move |_| {
            // TODO: Open file picker
            // On selection, update attached_files signal
        })
}
```

### Model Selection
Already handled in `model_selector_v2.rs` - updates `current_model` signal automatically.

### Message Streaming
```rust
// When new message chunk arrives:
is_streaming.set(true);
messages_signal.update(|msgs| {
    if let Some(last) = msgs.last_mut() {
        last.content.push_str(chunk);
        last.partial = true;
    }
});

// When complete:
is_streaming.set(false);
messages_signal.update(|msgs| {
    if let Some(last) = msgs.last_mut() {
        last.partial = false;
    }
});
```

---

## 🧪 Testing Checklist

After integration:

- [ ] Welcome screen shows when empty
- [ ] Messages display with correct roles
- [ ] Thinking indicator appears during streaming
- [ ] File attachments show above input
- [ ] Model selector opens on click
- [ ] All buttons have icons
- [ ] 20px send button renders correctly
- [ ] Hover states work
- [ ] Theme colors apply
- [ ] No compilation errors
- [ ] No runtime panics

---

## 🚀 Quick Integration Command

```bash
# 1. Backup current chat_view.rs
cp lapce-app/src/panel/ai_chat/components/chat_view.rs \
   lapce-app/src/panel/ai_chat/components/chat_view.rs.backup

# 2. Edit with your preferred editor
# (Follow the steps above)

# 3. Compile and test
cargo check --package lapce-app
cargo build --release --package lapce-app

# 4. Run and verify
./target/release/lapce
```

---

## 💡 Pro Tips

1. **Start Small** - Integrate one component at a time
2. **Test Often** - Run `cargo check` after each change
3. **Use Signals** - All state should be reactive
4. **Theme Everything** - Use `config().color()` for all colors
5. **Document** - Add comments for complex logic

---

## 🎯 Expected Result

After integration, your AI Chat panel will have:
- ✨ Beautiful welcome screen
- 💬 Professional message bubbles
- 💻 Syntax-highlighted code blocks
- 🔄 Thinking indicator
- 📎 File attachments
- 🎛️ Model selector
- ⌨️ Perfect input area
- 🎨 Theme-aware styling

**It will look EXACTLY like Windsurf!** 🚀

---

**Integration should take ~30 minutes if following this guide!**
