// ChatTextArea - Input area for AI chat
// Ported from ChatTextArea.tsx
// Phase 4: Simplified version without mention system, file attachments, mode selectors

use std::rc::Rc;
use std::sync::Arc;

use floem::{
    event::{EventListener, EventPropagation},
    keyboard::{Key, NamedKey},
    reactive::{RwSignal, SignalGet, SignalUpdate},
    views::{Decorators, container, h_stack, label, text_input, v_stack},
    View,
};

use crate::config::LapceConfig;

pub struct ChatTextAreaProps {
    pub input_value: RwSignal<String>,
    pub sending_disabled: bool,
    pub placeholder_text: String,
    pub on_send: Rc<dyn Fn()>,
}

/// Simplified chat text area for Phase 4
/// TODO: Add full features in later phases:
/// - File attachments
/// - Image selection
/// - Mention system (@file, @folder, @git, @url, etc.)
/// - Slash commands
/// - Mode selector
/// - API config selector
/// - Prompt history (up/down arrows)
pub fn chat_text_area(
    props: ChatTextAreaProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let input_value = props.input_value;
    let placeholder = props.placeholder_text.clone();
    let on_send = props.on_send.clone();
    let on_send2 = props.on_send.clone();
    
    v_stack((
        // Text input area
        container(
            text_input(input_value)
                .keyboard_navigable()
                .on_event(EventListener::KeyDown, move |event| {
                    if let floem::event::Event::KeyDown(key_event) = event {
                        // Send on Enter (without Shift)
                        if key_event.key.logical_key == Key::Named(NamedKey::Enter)
                            && !key_event.modifiers.shift()
                        {
                            on_send();
                            return EventPropagation::Stop;
                        }
                    }
                    EventPropagation::Continue
                })
                .style(move |s| {
                    let cfg = config();
                    s.width_full()
                        .min_height(60.0)
                        .padding(12.0)
                        .background(cfg.color("editor.background"))
                        .border(1.0)
                        .border_color(cfg.color("lapce.border"))
                        .border_radius(4.0)
                        .color(cfg.color("editor.foreground"))
                })
        )
        .style(|s| s.padding(8.0).width_full()),
        
        // Send button row
        container(
            h_stack((
                // Status info
                label(|| "".to_string())
                    .style(|s| s.flex_grow(1.0)),
                
                // Send button
                {
                    let disabled = props.sending_disabled;
                    container(
                        label(|| "Send".to_string())
                            .on_click_stop(move |_| {
                                if !disabled {
                                    on_send2();
                                }
                            })
                            .style(move |s| {
                                let cfg = config();
                                s.padding_horiz(16.0)
                                    .padding_vert(8.0)
                                    .background(if disabled {
                                        cfg.color("editor.dim")
                                    } else {
                                        cfg.color("lapce.button_primary")
                                    })
                                    .border_radius(4.0)
                                    .color(cfg.color("editor.foreground"))
                                    .cursor(if disabled {
                                        floem::style::CursorStyle::Default
                                    } else {
                                        floem::style::CursorStyle::Pointer
                                    })
                            })
                    )
                    .style(|s| s)
                },
            ))
        )
        .style(|s| s.padding(8.0).padding_top(0.0).width_full()),
    ))
    .style(|s| s.width_full())
}
