// Phase 0-6: Full chat UI with state integration

use std::{rc::Rc, sync::Arc};

use floem::{
    reactive::{create_effect, create_rw_signal, SignalGet, SignalUpdate},
    views::{container, v_stack, Decorators},
    IntoView,
};

use crate::{
    ai_bridge::{
        BridgeClient, ShmTransport, default_socket_path, transport::Transport,
        messages::{OutboundMessage, ProviderChatMessage},
    },
    ai_state::AIChatState,
    panel::ai_chat::components::{
        chat_view::{ChatViewProps, chat_view},
    },
    window_tab::WindowTabData,
};

/// Create the AI Chat panel view
pub fn ai_chat_panel(
    window_tab_data: Rc<WindowTabData>,
) -> impl IntoView {
    let config = window_tab_data.common.config;
    
    // Initialize AI state with real IPC transport
    let socket_path = default_socket_path();
    let mut transport = ShmTransport::new(socket_path.clone());
    
    // Attempt connection to backend (non-blocking, will retry on send if needed)
    if let Err(e) = Transport::connect(&mut transport) {
        eprintln!("[AI Chat] Failed to connect to backend at {}: {}", socket_path, e);
        eprintln!("[AI Chat] Messages will be queued until connection succeeds");
    }
    
    let bridge = Arc::new(BridgeClient::new(Box::new(transport)));
    let ai_state = Arc::new(AIChatState::new(bridge));
    
    // Local input state
    let input_value = create_rw_signal(String::new());
    let sending_disabled = false;
    
    // Model and mode selection state
    let selected_model = create_rw_signal("Claude Sonnet 4.5 Thinking ".to_string());
    let selected_mode = create_rw_signal("Code".to_string());
    
    // Message handler
    let ai_state_clone = ai_state.clone();
    let on_send = Rc::new(move || {
        let msg = input_value.get();
        eprintln!("[AI CHAT] on_send called with message: {}", msg);
        if !msg.trim().is_empty() {
            let model = selected_model.get();
            let mode = selected_mode.get();
            println!("[AI Chat] Sending: {} (model: {}, mode: {})", msg, model, mode);
            
            // Add user message to state
            eprintln!("[AI CHAT] Adding user message to UI...");
            ai_state_clone.messages.update(|msgs| {
                msgs.push(crate::ai_state::ChatMessage {
                    ts: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    message_type: crate::ai_bridge::messages::MessageType::Say,
                    content: msg.clone(),
                    partial: false,
                });
            });
            eprintln!("[AI CHAT] User message added to UI");
            
            // Send via IPC bridge - REAL STREAMING
            let bridge = ai_state_clone.bridge.clone();
            let prompt = msg.clone();
            
            // Build provider chat messages (just user message for now)
            let provider_messages = vec![
                ProviderChatMessage {
                    role: "user".to_string(),
                    content: prompt,
                },
            ];
            
            // Map UI model names to backend model IDs
            let backend_model = match model.trim() {
                "Claude Sonnet 4.5 Thinking" => "claude-3-5-sonnet-20241022",
                "Claude Sonnet 4" => "claude-3-opus-20240229",
                "GPT-4" => "gpt-4",
                "Gemini Pro" => "gemini-1.5-flash",
                _ => "gemini-1.5-flash", // Default to Gemini
            }.to_string();
            
            // Send streaming request to backend
            eprintln!("[AI CHAT] Sending ProviderChatStream to backend...");
            eprintln!("[AI CHAT] UI Model: {}, Backend Model: {}, Messages: {}", model, backend_model, provider_messages.len());
            if let Err(e) = bridge.send(OutboundMessage::ProviderChatStream {
                model: backend_model,
                messages: provider_messages,
                max_tokens: Some(2048),
                temperature: Some(0.7),
            }) {
                eprintln!("[AI CHAT] ❌ Failed to send message: {}", e);
            } else {
                eprintln!("[AI CHAT] ✅ Message sent successfully!");
            }
            
            input_value.set(String::new());
        }
    });
    
    // Poll for incoming messages (including streaming chunks)
    // TODO: Implement proper async polling or callback-based message handling
    // For now, polling would need to be triggered manually or via IPC callbacks
    // Floem's reactive signals (RwSignal) are not Send, so we can't use tokio::spawn
    // In production, the IPC layer would trigger updates directly via callbacks
    
    // Convert state messages to chat row messages
    let messages_signal = ai_state.messages;
    let streaming_signal = ai_state.streaming_text;
    
    // Main chat area (no sidebar)
    v_stack((
            // Chat view (includes messages area and input - integrated)
            container(
                chat_view(
                    ChatViewProps {
                        input_value,
                        sending_disabled,
                        on_send,
                        messages_signal,
                        streaming_signal,
                        selected_model,
                        selected_mode,
                    },
                    move || config.get_untracked(),
                )
            )
            .style(|s| s.flex_grow(1.0).width_full()),
            
            // Clean: No toolbar buttons (Windsurf style)
    ))
    .style(move |s| {
        let cfg = config.get_untracked();
        s.width_full()
            .height_full()
            .flex_col()
            .background(cfg.color("panel.background"))
    })
}
