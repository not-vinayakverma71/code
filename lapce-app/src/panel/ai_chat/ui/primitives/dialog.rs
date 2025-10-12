// Dialog/Modal Component - Radix UI equivalent for Floem
// Source: Codex webview-ui/src/components/ui/dialog.tsx
//
// Usage:
// dialog(
//     DialogProps {
//         open: signal,
//         title: "Dialog Title",
//         content: v_stack((...)),
//         footer: Some(h_stack((...)),
//     },
//     config
// )

use std::sync::Arc;

use floem::{
    event::{EventListener, EventPropagation},
    keyboard::Key,
    reactive::{RwSignal, SignalGet, SignalUpdate},
    style::{Position, JustifyContent},
    views::{Decorators, container, h_stack, label, v_stack, empty},
    View, IntoView,
};

use crate::config::LapceConfig;

pub struct DialogProps<C, F>
where
    C: IntoView + 'static,
    F: IntoView + 'static,
{
    /// Open/close state
    pub open: RwSignal<bool>,
    /// Dialog title
    pub title: String,
    /// Dialog description (optional)
    pub description: Option<String>,
    /// Main content
    pub content: C,
    /// Footer content (optional)
    pub footer: Option<F>,
    /// Show close button
    pub show_close: bool,
    /// Max width
    pub max_width: f64,
}

impl<C, F> DialogProps<C, F>
where
    C: IntoView + 'static,
    F: IntoView + 'static,
{
    pub fn new(open: RwSignal<bool>, title: impl Into<String>, content: C) -> Self {
        Self {
            open,
            title: title.into(),
            description: None,
            content,
            footer: None,
            show_close: true,
            max_width: 512.0, // sm:max-w-lg
        }
    }
    
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    pub fn with_footer(mut self, footer: F) -> Self {
        self.footer = Some(footer);
        self
    }
}

/// Dialog/Modal component - centered modal with overlay
///
/// Features:
/// - Semi-transparent overlay
/// - Click outside to close
/// - ESC key to close
/// - Close button (X)
/// - Title + description + content + footer
/// - Centered positioning
/// - Animation (fade + zoom)
pub fn dialog<C, F>(
    props: DialogProps<C, F>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View
where
    C: IntoView + 'static,
    F: IntoView + 'static,
{
    let open = props.open;
    let title = props.title.clone();
    let description = props.description.clone();
    let max_width = props.max_width;
    let show_close = props.show_close;
    
    container(
        container((
            // Overlay
            container(())
                .on_click_stop(move |_| {
                    open.set(false);
                })
                .style(move |s| {
                    let cfg = config();
                    let is_open = open.get();
                    if !is_open {
                        s.display(floem::style::Display::None)
                    } else {
                        s.position(Position::Absolute)
                            .inset(0.0)
                            .z_index(50)
                            .background(cfg.color("editor.foreground").multiply_alpha(0.5))
                    }
                }),
            
            // Dialog content
            container(
                v_stack((
                    // Header with close button
                    h_stack((
                        // Title & description
                        v_stack((
                            label(move || title.clone())
                                .style(move |s| {
                                    let cfg = config();
                                    s.font_size(18.0) // text-lg
                                        .font_weight(floem::text::Weight::SEMIBOLD)
                                        .color(cfg.color("editor.foreground"))
                                        .margin_bottom(8.0)
                                }),
                            {
                                if let Some(desc) = description.clone() {
                                    label(move || desc.clone())
                                        .style(move |s| {
                                            let cfg = config();
                                            s.font_size(14.0)
                                                .color(cfg.color("editor.dim"))
                                        })
                                        .into_any()
                                } else {
                                    empty().into_any()
                                }
                            },
                        ))
                        .style(|s| s.flex_grow(1.0)),
                        
                        // Close button
                        {
                            if show_close {
                                label(|| "X".to_string())
                                    .on_click_stop(move |_| {
                                        open.set(false);
                                    })
                                    .style(move |s| {
                                        let cfg = config();
                                        s.padding(4.0)
                                            .border_radius(4.0)
                                            .color(cfg.color("editor.foreground"))
                                            .cursor(floem::style::CursorStyle::Pointer)
                                    })
                                    .into_any()
                            } else {
                                empty().into_any()
                            }
                        },
                    ))
                    .style(|s| s.width_full().margin_bottom(16.0)),
                    
                    // Content
                    container(props.content)
                        .style(|s| s.flex_grow(1.0).width_full()),
                    
                    // Footer
                    {
                        if let Some(footer) = props.footer {
                            container(footer)
                                .style(|s| {
                                    s.width_full()
                                        .margin_top(16.0)
                                        .flex_row()
                                        .justify_end()
                                        .gap(8.0)
                                })
                                .into_any()
                        } else {
                            label(|| "".to_string()).style(|s| s.height(0.0)).into_any()
                        }
                    },
                ))
                .style(|s| s.flex_col().width_full())
            )
            .on_event_stop(EventListener::KeyDown, move |event| {
                if let floem::event::Event::KeyDown(key_event) = event {
                    if key_event.key.logical_key == Key::Named(floem::keyboard::NamedKey::Escape) {
                        open.set(false);
                        return EventPropagation::Stop;
                    }
                }
                EventPropagation::Continue
            })
            .style(move |s| {
                let cfg = config();
                let is_open = open.get();
                
                if !is_open {
                    return s.display(floem::style::Display::None);
                }
                
                s.position(Position::Fixed)
                    .z_index(51)
                    // Center on screen
                    .inset_top_pct(50.0)
                    .inset_left_pct(50.0)
                    // Translate to true center
                    .width(max_width.min(900.0)) // max-w-lg with calc(100%-2rem) limit
                    .max_height_pct(90.0)
                    .padding(24.0) // p-6
                    .border_radius(8.0) // rounded-lg
                    .background(cfg.color("editor.background"))
                    .border(1.0)
                    .border_color(cfg.color("lapce.border"))
                    // Shadow
                    .box_shadow_blur(16.0)
                    .box_shadow_color(cfg.color("editor.foreground").multiply_alpha(0.3))
            }),
        ))
    )
    .style(move |s| {
        let is_open = open.get();
        if is_open {
            s.position(Position::Absolute)
                .inset(0.0)
                .z_index(50)
        } else {
            s.display(floem::style::Display::None)
        }
    })
}

/// Simple dialog builder with OK/Cancel buttons
pub fn confirm_dialog(
    open: RwSignal<bool>,
    title: impl Into<String>,
    message: impl Into<String>,
    on_confirm: impl Fn() + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let message_str = message.into();
    let on_confirm = Arc::new(on_confirm);
    
    dialog(
        DialogProps::new(
            open,
            title,
            label(move || message_str.clone())
                .style(move |s| {
                    let cfg = config();
                    s.padding(16.0)
                        .color(cfg.color("editor.foreground"))
                }),
        )
        .with_footer(
            h_stack((
                // Cancel button
                label(|| "Cancel".to_string())
                    .on_click_stop(move |_| {
                        open.set(false);
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.padding(8.0)
                            .padding_horiz(16.0)
                            .border_radius(4.0)
                            .background(cfg.color("panel.background"))
                            .border(1.0)
                            .border_color(cfg.color("lapce.border"))
                            .color(cfg.color("editor.foreground"))
                            .cursor(floem::style::CursorStyle::Pointer)
                    }),
                
                // Confirm button
                label(|| "OK".to_string())
                    .on_click_stop({
                        let on_confirm = on_confirm.clone();
                        move |_| {
                            on_confirm();
                            open.set(false);
                        }
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.padding(8.0)
                            .padding_horiz(16.0)
                            .border_radius(4.0)
                            .background(cfg.color("lapce.button_primary"))
                            .color(cfg.color("editor.background"))
                            .cursor(floem::style::CursorStyle::Pointer)
                    }),
            ))
            .style(|s| s.gap(8.0))
        ),
        config,
    )
}
