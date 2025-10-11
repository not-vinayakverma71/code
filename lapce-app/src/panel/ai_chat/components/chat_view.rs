// ChatView - Main chat interface container
// Ported from ChatView.tsx (simplified)

use std::rc::Rc;
use std::sync::Arc;

use floem::{
    reactive::{RwSignal, SignalGet},
    views::{Decorators, container, label, scroll, v_stack},
    View,
};

use crate::{
    config::LapceConfig,
    panel::ai_chat::components::{
        chat_text_area::{ChatTextAreaProps, chat_text_area},
        chat_row::{ChatMessage, ChatRowProps, chat_row},
        welcome_screen::welcome_screen,
    },
};

pub struct ChatViewProps {
    pub input_value: RwSignal<String>,
    pub sending_disabled: bool,
    pub on_send: Rc<dyn Fn()>,
    pub messages: Vec<ChatMessage>,
}

/// Simplified ChatView for Phase 4
/// TODO: Add full features:
/// - Message list with virtualization
/// - ChatRow rendering
/// - Auto-scroll behavior
/// - Task history/checkpoints
/// - Welcome screen
/// - Announcement banners
pub fn chat_view(
    props: ChatViewProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let input_value = props.input_value;
    let messages = props.messages.clone();
    
    v_stack((
        // Messages area (scrollable)
        container(
            scroll(
                if messages.is_empty() {
                    container(welcome_screen(config))
                } else {
                    // TODO Phase 6: Render actual message list with tool integration
                    container(
                        label(move || format!("{} messages", messages.len()))
                    )
                    .style(move |s| {
                        let cfg = config();
                        s.padding(20.0)
                            .color(cfg.color("editor.dim"))
                    })
                }
                .style(|s| s.padding(12.0).width_full())
            )
            .style(|s| s.flex_grow(1.0).width_full())
        )
        .style(move |s| {
            let cfg = config();
            s.flex_grow(1.0)
                .width_full()
                .background(cfg.color("editor.background"))
        }),
        
        // Input area
        container(
            chat_text_area(
                ChatTextAreaProps {
                    input_value,
                    sending_disabled: props.sending_disabled,
                    placeholder_text: "Ask AI...".to_string(),
                    on_send: props.on_send.clone(),
                },
                config,
            )
        )
        .style(move |s| {
            let cfg = config();
            s.width_full()
                .border_top(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("panel.background"))
        }),
    ))
    .style(|s| s.width_full().height_full().flex_col())
}
