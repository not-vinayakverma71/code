// Chat Input - Main text input area for user messages
// Multi-line text entry with controls

use std::sync::Arc;
use floem::{
    reactive::{RwSignal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone)]
pub struct ChatInputData {
    pub text: String,
    pub is_generating: bool,
    pub can_send: bool,
    pub placeholder: String,
}

/// Chat input component
/// Multi-line text input with send/stop controls and attachment buttons
pub fn chat_input(
    text: RwSignal<String>,
    is_generating: RwSignal<bool>,
    on_send: impl Fn(String) + 'static + Copy,
    on_stop: impl Fn() + 'static + Copy,
    on_attach_file: impl Fn() + 'static + Copy,
    on_attach_folder: impl Fn() + 'static + Copy,
    on_attach_selection: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        // Main input area (simplified - will be replaced with actual text editor)
        container(
            label(move || {
                let t = text.get();
                if t.is_empty() {
                    "Type your message... (Shift+Enter for new line, Enter to send)".to_string()
                } else {
                    t
                }
            })
        )
        .on_click_stop(move |_| {
            // Focus text input
            // TODO: Wire to actual text editor when integrated
        })
        .style(move |s| {
            let cfg = config();
            let t = text.get();
            s.padding(12.0)
                .width_full()
                .min_height(80.0)
                .max_height(300.0)
                .border(1.0)
                .border_radius(6.0)
                .border_color(cfg.color("input.border"))
                .background(cfg.color("input.background"))
                .color(if t.is_empty() {
                    cfg.color("input.placeholderForeground")
                } else {
                    cfg.color("input.foreground")
                })
                .font_size(13.0)
                .line_height(1.5)
                .cursor(floem::style::CursorStyle::Text)
        }),
        
        // Controls bar
        h_stack((
            // Left side - attachment buttons
            h_stack((
                container(
                    label(|| "📎 File".to_string())
                )
                .on_click_stop(move |_| {
                    on_attach_file();
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(6.0)
                        .padding_horiz(10.0)
                        .border_radius(4.0)
                        .background(cfg.color("panel.current.background"))
                        .color(cfg.color("editor.foreground"))
                        .font_size(11.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(cfg.color("panel.hovered.background")))
                        .margin_right(6.0)
                }),
                
                container(
                    label(|| "📁 Folder".to_string())
                )
                .on_click_stop(move |_| {
                    on_attach_folder();
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(6.0)
                        .padding_horiz(10.0)
                        .border_radius(4.0)
                        .background(cfg.color("panel.current.background"))
                        .color(cfg.color("editor.foreground"))
                        .font_size(11.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(cfg.color("panel.hovered.background")))
                        .margin_right(6.0)
                }),
                
                container(
                    label(|| "📋 Selection".to_string())
                )
                .on_click_stop(move |_| {
                    on_attach_selection();
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(6.0)
                        .padding_horiz(10.0)
                        .border_radius(4.0)
                        .background(cfg.color("panel.current.background"))
                        .color(cfg.color("editor.foreground"))
                        .font_size(11.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(cfg.color("panel.hovered.background")))
                }),
            )),
            
            // Spacer
            container(floem::views::empty())
                .style(|s| s.flex_grow(1.0)),
            
            // Right side - send/stop button
            container(
                if is_generating.get() {
                    container(
                        label(|| "⏹ Stop".to_string())
                    )
                    .on_click_stop(move |_| {
                        on_stop();
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.padding(8.0)
                            .padding_horiz(16.0)
                            .border_radius(6.0)
                            .background(cfg.color("list.errorForeground"))
                            .color(cfg.color("editor.background"))
                            .font_size(12.0)
                            .font_bold()
                            .cursor(floem::style::CursorStyle::Pointer)
                    })
                } else {
                    container(
                        label(|| "↑ Send".to_string())
                    )
                    .on_click_stop(move |_| {
                        let message = text.get();
                        if !message.trim().is_empty() {
                            on_send(message);
                            text.set(String::new());
                        }
                    })
                    .style(move |s| {
                        let cfg = config();
                        let can_send = !text.get().trim().is_empty();
                        s.padding(8.0)
                            .padding_horiz(16.0)
                            .border_radius(6.0)
                            .background(if can_send {
                                cfg.color("lapce.button.primary.background")
                            } else {
                                cfg.color("panel.current.background")
                            })
                            .color(if can_send {
                                cfg.color("lapce.button.primary.foreground")
                            } else {
                                cfg.color("editor.dim")
                            })
                            .font_size(12.0)
                            .font_bold()
                            .cursor(if can_send {
                                floem::style::CursorStyle::Pointer
                            } else {
                                floem::style::CursorStyle::Default
                            })
                            .hover(move |s| {
                                if can_send {
                                    s.background(cfg.color("panel.hovered.background"))
                                } else {
                                    s
                                }
                            })
                    })
                }
            ),
        ))
        .style(|s| s.margin_top(8.0)),
    ))
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .padding(16.0)
            .border_top(1.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background"))
    })
}
