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
    ai_bridge::messages::{MessageType as BridgeMessageType},
    config::LapceConfig,
    panel::ai_chat::components::{
        windsurf_ui,
        chat_row::{ChatMessage, ChatRowProps, MessageType, SayType, AskType, chat_row},
        welcome_screen::welcome_screen,
    },
};

pub struct ChatViewProps {
    pub input_value: RwSignal<String>,
    pub sending_disabled: bool,
    pub on_send: Rc<dyn Fn()>,
    pub messages_signal: RwSignal<Vec<crate::ai_state::ChatMessage>>,
    pub streaming_signal: RwSignal<String>,  // Live streaming text
    pub selected_model: RwSignal<String>,
    pub selected_mode: RwSignal<String>,
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
    let streaming_signal = props.streaming_signal;
    
    // Windsurf-style model selector state
    let current_model = create_rw_signal("GPT-4".to_string());
    let is_model_open = create_rw_signal(false);
    
    v_stack((
        // Messages area (scrollable)
        container(
            scroll(
                v_stack((
                    // Welcome screen (shows when empty)
                    container(welcome_screen(config))
                        .style(move |s| {
                            let msgs = messages_signal.get();
                            if msgs.is_empty() {
                                s.width_full().flex_grow(1.0)
                            } else {
                                s.width_full().height(0.0)
                            }
                        }),
                    
                    // Message list
                    dyn_stack(
                        move || messages_signal.get(),
                        |msg| msg.ts,
                        move |msg| {
                            let is_expanded = create_rw_signal(false);
                            let is_last = false;
                            
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
                    .style(|s| s.width_full().flex_col().gap(12.0)),
                    
                    // Streaming text display (live assistant response)
                    container({
                        let text = streaming_signal.get();
                        if text.is_empty() {
                            container(label(|| "".to_string()))
                                .style(|s| s.height(0.0))
                        } else {
                            container(windsurf_ui::ai_message(text, false))
                                .style(|s| s.width_full())
                        }
                    })
                    .style(|s| s.width_full().padding(8.0))
                ))
                .style(|s| s.padding(12.0).width_full().flex_col().gap(12.0))
            )
            .style(|s| s.flex_grow(1.0).width_full())
        )
        .style(move |s| {
            s.flex_grow(1.0)
                .width_full()
                .background(floem::peniko::Color::from_rgb8(0x1a, 0x1a, 0x1a))
        }),
        
        // Clean Windsurf input bar with model/mode selection
        {
            let on_send_clone = props.on_send.clone();
            windsurf_ui::input_bar(
                input_value,
                move || (on_send_clone)(),
                props.sending_disabled,
                props.selected_model,
                props.selected_mode,
            )
        },
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
