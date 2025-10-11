// ChatView - Main chat interface container
// Ported from ChatView.tsx (simplified)

use std::rc::Rc;
use std::sync::Arc;

use floem::{
    reactive::{RwSignal, SignalGet, create_rw_signal},
    views::{Decorators, container, dyn_stack, label, scroll, v_stack},
    View,
};

use crate::{
    ai_bridge::messages::{MessageType as BridgeMessageType, SayType as BridgeSayType, AskType as BridgeAskType},
    config::LapceConfig,
    panel::ai_chat::components::{
        chat_text_area::{ChatTextAreaProps, chat_text_area},
        chat_row::{ChatMessage, ChatRowProps, MessageType, SayType, AskType, chat_row},
        welcome_screen::welcome_screen,
    },
};

pub struct ChatViewProps {
    pub input_value: RwSignal<String>,
    pub sending_disabled: bool,
    pub on_send: Rc<dyn Fn()>,
    pub messages_signal: RwSignal<Vec<crate::ai_state::ChatMessage>>,
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
    let messages_signal = props.messages_signal;
    
    v_stack((
        // Messages area (scrollable)
        container(
            scroll(
                container(
                    dyn_stack(
                        move || messages_signal.get(),
                        |msg| msg.ts,
                        move |msg| {
                            let is_expanded = create_rw_signal(false);
                            let is_last = false; // TODO: track properly
                            
                            // Convert bridge message to chat row message
                            let chat_msg = ChatMessage {
                                ts: msg.ts,
                                message_type: convert_message_type(&msg.message_type, msg.content.contains("tool")),
                                text: msg.content.clone(),
                                partial: msg.partial,
                            };
                            
                            chat_row(
                                ChatRowProps {
                                    message: chat_msg,
                                    is_last,
                                    is_expanded,
                                },
                                config,
                            )
                        },
                    )
                )
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

/// Convert bridge message type to chat row message type
fn convert_message_type(msg_type: &BridgeMessageType, has_tool_content: bool) -> MessageType {
    match msg_type {
        BridgeMessageType::Say => {
            if has_tool_content {
                MessageType::Say(SayType::Tool)
            } else {
                MessageType::Say(SayType::Text)
            }
        }
        BridgeMessageType::Ask => {
            if has_tool_content {
                MessageType::Ask(AskType::Tool)
            } else {
                MessageType::Ask(AskType::Followup)
            }
        }
    }
}
