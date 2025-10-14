// ChatTextArea - EXACT Windsurf input area replica
// Key measurements: 20x20px send button, gap-1.5 (6px), text-[12px] buttons

use std::rc::Rc;
use std::sync::Arc;

use floem::{
    event::{EventListener, EventPropagation},
    keyboard::{Key, NamedKey},
    peniko::Color,
    reactive::{RwSignal, SignalGet, SignalUpdate},
    style::CursorStyle,
    views::{Decorators, container, empty, h_stack, svg, text_input, v_stack},
    View,
};

use crate::{
    config::LapceConfig,
    panel::ai_chat::{
        icons::*,
    },
};

pub struct ChatTextAreaProps {
    pub input_value: RwSignal<String>,
    pub sending_disabled: bool,
    pub placeholder_text: String,
    pub on_send: Rc<dyn Fn()>,
    pub current_model: RwSignal<String>,
    pub is_model_open: RwSignal<bool>,
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
        // Input area
        container(
            text_input(input_value)
                .keyboard_navigable()
                .placeholder(placeholder)
                .on_event(EventListener::KeyDown, move |event| {
                    if let floem::event::Event::KeyDown(key_event) = event {
                        if key_event.key.logical_key == Key::Named(NamedKey::Enter)
                            && !key_event.modifiers.shift()
                        {
                            if !input_value.get().trim().is_empty() {
                                on_send();
                                input_value.set(String::new());
                            }
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
            ))
            .style(|s| s.gap(6.0)),  // gap-1.5 = 6px
            
            // Spacer (pushes right buttons to the right)
            empty().style(|s| s.flex_grow(1.0)),
            
            // Right buttons group
            h_stack((
                // Microphone button
                mic_button(config),
                
                // Send button (20x20px circular)
                send_button(disabled, input_value, on_send_btn, config),
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
            .border_radius(8.0)  // Reduced to avoid clipping dropdown
            .background(cfg.color("panel.background"))
            .border(1.0)
            .border_color(cfg.color("lapce.border"))
            .box_shadow_blur(3.0)
            .box_shadow_color(Color::BLACK.multiply_alpha(0.1))
    })
}

/// Add files button (+) - h-3 w-3 (12px), stroke-[2]
fn add_files_button(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        svg(|| ICON_PLUS.to_string())
            .style(move |s| {
                let cfg = config();
                s.width(12.0)
                    .height(12.0)
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

/// Code button - size-[12px] (12x12px), stroke-[2.5px]
fn code_button(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        svg(|| ICON_CODE.to_string())
            .style(move |s| {
                let cfg = config();
                s.width(12.0)
                    .height(12.0)
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
        svg(|| ICON_MIC.to_string())
            .style(move |s| {
                let cfg = config();
                s.width(14.0)  // h-3.5 w-3.5 = 14px
                    .height(14.0)
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

/// Send button - Windsurf style: 32x32px blue circle with white arrow
/// Blue: #0078d4, Hover: #026ec1, Icon: white
fn send_button(
    disabled: bool,
    input_value: RwSignal<String>,
    on_click: Rc<dyn Fn()>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        svg(|| ICON_ARROW_UP.to_string())
            .style(move |s| {
                s.width(14.0)  // Slightly larger for 32px button
                    .height(14.0)
                    .color(Color::WHITE)  // White arrow
            })
    )
    .on_click_stop(move |_| {
        if !disabled && !input_value.get().trim().is_empty() {
            on_click();
            input_value.set(String::new());
        }
    })
    .style(move |s| {
        s.width(32.0)      // Windsurf size: 32x32px
            .height(32.0)
            .border_radius(16.0)  // Fully circular
            .flex_shrink(0.0)
            .justify_center()
            .items_center()
            .background(if disabled {
                Color::from_rgb8(0x00, 0x78, 0xd4).multiply_alpha(0.5)  // Dimmed blue
            } else {
                Color::from_rgb8(0x00, 0x78, 0xd4)  // #0078d4 Windsurf blue
            })
            .cursor(if disabled {
                CursorStyle::Default
            } else {
                CursorStyle::Pointer
            })
            .hover(move |s| {
                if !disabled {
                    s.background(Color::from_rgb8(0x02, 0x6e, 0xc1))  // #026ec1 darker blue on hover
                } else {
                    s
                }
            })
    })
}
