// AI Chat Panel View
// Phase 0-6: Full chat UI with state integration

use std::{rc::Rc, sync::Arc};

use floem::{
    reactive::{create_rw_signal, SignalGet, SignalUpdate},
    views::{container, v_stack, Decorators},
    IntoView,
};

use crate::{
    ai_bridge::{BridgeClient, NoTransport},
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
    
    // Initialize AI state with NoTransport (pre-IPC)
    let bridge = Arc::new(BridgeClient::new(Box::new(NoTransport::new())));
    let ai_state = Arc::new(AIChatState::new(bridge));
    
    // Local input state
    let input_value = create_rw_signal(String::new());
    let sending_disabled = false;
    
    // Message handler
    let ai_state_clone = ai_state.clone();
    let on_send = Rc::new(move || {
        let msg = input_value.get();
        if !msg.trim().is_empty() {
            println!("[AI Chat] Sending: {}", msg);
            
            // Add user message to state
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
            
            // TODO: Send via IPC bridge when ready
            // ai_state.bridge.send(OutboundMessage::NewTask { text: msg, images: vec![] });
            
            input_value.set(String::new());
        }
    });
    
    // Convert state messages to chat row messages
    let messages_signal = ai_state.messages;
    
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
