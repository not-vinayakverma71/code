// ChatTextArea - EXACT Windsurf input area replica
// Extracted from real Windsurf HTML (small.html)
// Key measurements: 20x20px send button, gap-1.5 (6px), text-[12px] buttons

use std::rc::Rc;
use std::sync::Arc;

use floem::{
    event::{EventListener, EventPropagation},
    keyboard::{Key, NamedKey},
    peniko::Color,
    reactive::RwSignal,
    style::CursorStyle,
    text::Weight,
    views::{Decorators, container, empty, h_stack, label, svg, text, text_input, v_stack},
    View, ViewId,
};

use crate::config::LapceConfig;

pub struct ChatTextAreaProps {
    pub input_value: RwSignal<String>,
    pub sending_disabled: bool,
    pub placeholder_text: String,
    pub on_send: Rc<dyn Fn()>,
}

/// EXACT Windsurf chat input area
/// Based on real HTML from small.html:
/// - Main container: rounded-[15px], p-[6px]
/// - Input: min-h-[2rem], max-h-[300px]
/// - Send button: h-[20px] w-[20px] rounded-full
/// - Button row: gap-1.5 (6px)
/// - Buttons: text-[12px], opacity-70, hover:opacity-100
pub fn chat_text_area(
    props: ChatTextAreaProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let input_value = props.input_value;
    let on_send = props.on_send.clone();
    let on_send_btn = props.on_send.clone();
    let disabled = props.sending_disabled;
    let placeholder = props.placeholder_text;
    
    // Main container with input and button row
    v_stack((
        // Input area with placeholder overlay
        container(
            v_stack((
                // Multi-line text input
                text_input(input_value)
                    .keyboard_navigable()
                    .on_event(EventListener::KeyDown, move |event| {
                        if let floem::event::Event::KeyDown(key_event) = event {
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
                            .min_height(32.0)  // 2rem = 32px
                            .max_height(300.0)
                            .padding(0.0)
                            .background(Color::TRANSPARENT)
                            .border(0.0)
                            .color(cfg.color("input.foreground"))
                            .font_size(14.0)
                    }),
                
                // Placeholder overlay (shown when input is empty)
                // Positioned absolutely over input
                label(move || {
                    if input_value.get().is_empty() {
                        placeholder.clone()
                    } else {
                        String::new()
                    }
                })
                .style(move |s| {
                    let cfg = config();
                    let visible = input_value.get().is_empty();
                    s.position_absolute()
                        .inset_left(0.0)
                        .inset_right(0.0)
                        .inset_top(0.0)
                        .color(cfg.color("input.foreground").multiply_alpha(0.5))
                        .font_size(14.0)
                        .cursor(CursorStyle::Text)
                        .apply_if(!visible, |s| s.display_none())
                }),
            ))
            .style(|s| {
                s.position_relative()
                    .width_full()
                    .min_height(32.0)
            })
        )
        .style(|s| {
            s.width_full()
                .padding_left(3.0)
                .padding_top(1.0)
                .padding_bottom(4.0)
        }),
        
        // Button row (bottom)
        h_stack((
            // Left buttons group
            h_stack((
                // Add files button (+)
                add_files_button(config),
                
                // Code button
                code_button(config),
                
                // Model selector button
                model_selector_button(config),
            ))
            .style(|s| s.gap(6.0)),  // gap-1.5 = 6px
            
            // Spacer (pushes right buttons to the right)
            empty().style(|s| s.flex_grow(1.0)),
            
            // Right buttons group
            h_stack((
                // Microphone button
                mic_button(config),
                
                // Send button (20x20px circular)
                send_button(disabled, on_send_btn, config),
            ))
            .style(|s| s.gap(6.0)),  // gap-1.5 = 6px
        ))
        .style(|s| {
            s.width_full()
                .items_center()
                .gap(6.0)
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .padding(6.0)  // p-[6px]
            .border_radius(15.0)  // rounded-[15px]
            .background(cfg.color("panel.background"))
            .border(1.0)
            .border_color(cfg.color("lapce.border"))
            .box_shadow_blur(3.0)
            .box_shadow_color(Color::BLACK.multiply_alpha(0.1))
    })
}

/// Add files button (+) - text-[12px], opacity-70
fn add_files_button(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        label(|| "+".to_string())
            .style(move |s| {
                let cfg = config();
                s.font_size(12.0)
                    .font_weight(Weight::BOLD)
                    .color(cfg.color("editor.foreground"))
            })
    )
    .style(move |s| {
        let cfg = config();
        s.padding_horiz(4.0)
            .padding_vert(2.0)
            .border_radius(4.0)
            .cursor(CursorStyle::Pointer)
            .color(cfg.color("editor.foreground").multiply_alpha(0.7))
            .hover(|s| {
                let cfg = config();
                s.background(cfg.color("editor.foreground").multiply_alpha(0.1))
                    .color(cfg.color("editor.foreground"))
            })
    })
}

/// Code button with icon
fn code_button(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        label(|| "</>".to_string())
            .style(move |s| {
                let cfg = config();
                s.font_size(12.0)
                    .color(cfg.color("editor.foreground"))
            })
    )
    .style(move |s| {
        let cfg = config();
        s.padding_horiz(4.0)
            .padding_vert(2.0)
            .border_radius(4.0)
            .cursor(CursorStyle::Pointer)
            .color(cfg.color("editor.foreground").multiply_alpha(0.7))
            .hover(|s| {
                let cfg = config();
                s.background(cfg.color("editor.foreground").multiply_alpha(0.1))
                    .color(cfg.color("editor.foreground"))
            })
    })
}

/// Model selector button
fn model_selector_button(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        label(|| "GPT-5-Codex".to_string())
            .style(move |s| {
                let cfg = config();
                s.font_size(12.0)
                    .color(cfg.color("editor.foreground"))
            })
    )
    .style(move |s| {
        let cfg = config();
        s.padding_horiz(4.0)
            .padding_vert(2.0)
            .border_radius(4.0)
            .cursor(CursorStyle::Pointer)
            .color(cfg.color("editor.foreground").multiply_alpha(0.7))
            .hover(|s| {
                let cfg = config();
                s.background(cfg.color("editor.foreground").multiply_alpha(0.1))
                    .color(cfg.color("editor.foreground"))
            })
    })
}

/// Microphone button - h-3.5 w-3.5 (14px icon)
fn mic_button(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        label(|| "🎤".to_string())
            .style(move |s| {
                let cfg = config();
                s.font_size(14.0)
                    .color(cfg.color("editor.foreground"))
            })
    )
    .style(move |s| {
        let cfg = config();
        s.padding(4.0)
            .border_radius(4.0)
            .cursor(CursorStyle::Pointer)
            .color(cfg.color("editor.foreground").multiply_alpha(0.7))
            .hover(|s| {
                let cfg = config();
                s.background(cfg.color("editor.foreground").multiply_alpha(0.2))
                    .color(cfg.color("editor.foreground"))
            })
    })
}

/// Send button - EXACT: h-[20px] w-[20px] rounded-full
/// Arrow icon: h-3 w-3 (12px)
fn send_button(
    disabled: bool,
    on_click: Rc<dyn Fn()>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        label(|| "↑".to_string())
            .style(move |s| {
                let cfg = config();
                s.font_size(12.0)  // h-3 w-3 = 12px
                    .font_weight(Weight::BOLD)
                    .color(cfg.color("input.background"))
            })
    )
    .on_click_stop(move |_| {
        if !disabled {
            on_click();
        }
    })
    .style(move |s| {
        let cfg = config();
        s.width(20.0)      // EXACT: w-[20px]
            .height(20.0)   // EXACT: h-[20px]
            .border_radius(10.0)  // rounded-full = 50%
            .flex_shrink(0.0)
            .justify_center()
            .items_center()
            .background(if disabled {
                cfg.color("input.foreground").multiply_alpha(0.5)
            } else {
                cfg.color("input.foreground")
            })
            .cursor(if disabled {
                CursorStyle::NotAllowed
            } else {
                CursorStyle::Pointer
            })
            .apply_if(disabled, |s| s.apply_if(disabled, |s| s.opacity(0.5)))
    })
}
