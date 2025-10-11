// AI Chat Panel View
// Phase 0-4: Full chat UI integrated

use std::rc::Rc;

use floem::{
    reactive::{create_rw_signal, SignalGet, SignalUpdate},
    views::{container, Decorators},
    IntoView,
};

use crate::{
    panel::ai_chat::components::{
        chat_view::{ChatViewProps, chat_view},
        chat_row::{ChatMessage, MessageType, SayType},
    },
    window_tab::WindowTabData,
};

/// Create the AI Chat panel view
pub fn ai_chat_panel(
    window_tab_data: Rc<WindowTabData>,
) -> impl IntoView {
    let config = window_tab_data.common.config;
    
    // Chat state
    let input_value = create_rw_signal(String::new());
    let messages = create_rw_signal(Vec::new());
    let sending_disabled = false;
    
    // Message handler
    let on_send = Rc::new(move || {
        let msg = input_value.get();
        if !msg.trim().is_empty() {
            println!("[AI Chat] Sending: {}", msg);
            
            // Add user message
            messages.update(|msgs| {
                msgs.push(ChatMessage {
                    ts: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    message_type: MessageType::Say(SayType::User),
                    text: msg.clone(),
                    partial: false,
                });
            });
            
            // TODO: Wire to IPC bridge when ready
            input_value.set(String::new());
        }
    });
    
    container(
        chat_view(
            ChatViewProps {
                input_value,
                sending_disabled,
                on_send,
                messages: messages.get_untracked(),
            },
            move || config.get_untracked(),
        )
    )
    .style(move |s| {
        let cfg = config.get_untracked();
        s.width_full()
            .height_full()
            .background(cfg.color("panel.background"))
    })
}
