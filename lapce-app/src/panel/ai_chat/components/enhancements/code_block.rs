// Code Block - Syntax highlighted code display
// Show code with language-specific highlighting and copy button

use std::sync::Arc;
use floem::{
    views::{container, h_stack, label, scroll, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone)]
pub struct CodeBlockData {
    pub language: String,
    pub code: String,
    pub line_numbers: bool,
    pub filename: Option<String>,
}

/// Code block component with syntax highlighting
/// Full highlighting will be integrated with Lapce's existing syntax engine
pub fn code_block(
    data: CodeBlockData,
    on_copy: impl Fn(String) + 'static + Copy,
    on_insert: impl Fn(String) + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        // Header with language and actions
        h_stack((
            // Language badge
            container(
                label({
                    let lang = data.language.clone();
                    move || if lang.is_empty() { "text".to_string() } else { lang.clone() }
                })
            )
            .style(move |s| {
                let cfg = config();
                s.padding(4.0)
                    .padding_horiz(8.0)
                    .border_radius(3.0)
                    .background(cfg.color("badge.background"))
                    .color(cfg.color("badge.foreground"))
                    .font_size(10.0)
                    .font_bold()
                    .margin_right(8.0)
            }),
            
            // Filename (if provided)
            if let Some(filename) = data.filename.clone() {
                label(move || filename.clone())
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.foreground"))
                            .font_family("monospace".to_string())
                            .font_size(11.0)
                    })
            } else {
                label(|| String::new())
                    .style(|s| s.display(floem::style::Display::None))
            },
            
            // Spacer
            container(floem::views::empty())
                .style(|s| s.flex_grow(1.0)),
            
            // Copy button
            container(
                label(|| "📋 Copy".to_string())
            )
            .on_click_stop({
                let code = data.code.clone();
                move |_| {
                    on_copy(code.clone());
                }
            })
            .style(move |s| {
                let cfg = config();
                s.padding(4.0)
                    .padding_horiz(8.0)
                    .border_radius(3.0)
                    .background(cfg.color("panel.current.background"))
                    .color(cfg.color("editor.foreground"))
                    .font_size(10.0)
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(cfg.color("panel.hovered.background")))
                    .margin_right(6.0)
            }),
            
            // Insert button
            container(
                label(|| "+ Insert".to_string())
            )
            .on_click_stop({
                let code = data.code.clone();
                move |_| {
                    on_insert(code.clone());
                }
            })
            .style(move |s| {
                let cfg = config();
                s.padding(4.0)
                    .padding_horiz(8.0)
                    .border_radius(3.0)
                    .background(cfg.color("lapce.button.primary.background"))
                    .color(cfg.color("lapce.button.primary.foreground"))
                    .font_size(10.0)
                    .cursor(floem::style::CursorStyle::Pointer)
            }),
        ))
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .border_bottom(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("titleBar.inactiveBackground"))
                .items_center()
        }),
        
        // Code content
        scroll(
            container(
                if data.line_numbers {
                    h_stack((
                        // Line numbers
                        label({
                            let code = data.code.clone();
                            move || {
                                let line_count = code.lines().count();
                                (1..=line_count)
                                    .map(|n| format!("{:>4}", n))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            }
                        })
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editorLineNumber.foreground"))
                                .font_family("monospace".to_string())
                                .font_size(12.0)
                                .line_height(1.5)
                                .padding_right(12.0)
                                .border_right(1.0)
                                .border_color(cfg.color("lapce.border"))
                                .margin_right(12.0)
                        }),
                        
                        // Code
                        label({
                            let code = data.code.clone();
                            move || code.clone()
                        })
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_family("monospace".to_string())
                                .font_size(12.0)
                                .line_height(1.5)
                        }),
                    ))
                } else {
                    h_stack((
                        label({
                            let code = data.code.clone();
                            move || code.clone()
                        })
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_family("monospace".to_string())
                                .font_size(12.0)
                                .line_height(1.5)
                        }),
                    ))
                }
            )
            .style(move |s| {
                let cfg = config();
                s.padding(12.0)
                    .background(cfg.color("terminal.background"))
                    .width_full()
            })
        )
        .style(|s| s.max_height(400.0)),
    ))
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .border(1.0)
            .border_radius(6.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background"))
            .margin_bottom(8.0)
    })
}

/// Inline code snippet
/// Small inline code style
pub fn inline_code(
    code: String,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        label(move || code.clone())
    )
    .style(move |s| {
        let cfg = config();
        s.padding(2.0)
            .padding_horiz(6.0)
            .border_radius(3.0)
            .background(cfg.color("terminal.background"))
            .color(cfg.color("editor.foreground"))
            .font_family("monospace".to_string())
            .font_size(12.0)
    })
}
