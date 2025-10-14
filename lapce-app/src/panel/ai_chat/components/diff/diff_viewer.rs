// Diff Viewer - Displays file diffs in unified format
// Shows additions, deletions, and context lines

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, scroll, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineType {
    Addition,
    Deletion,
    Context,
    Separator,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub old_line_num: Option<usize>,
    pub new_line_num: Option<usize>,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct DiffViewerData {
    pub file_path: String,
    pub old_path: Option<String>, // For renamed files
    pub lines: Vec<DiffLine>,
    pub total_additions: usize,
    pub total_deletions: usize,
}

/// Unified diff viewer
/// Displays file changes with line numbers and syntax highlighting
pub fn diff_viewer(
    data: DiffViewerData,
    on_apply: impl Fn() + 'static + Copy,
    on_reject: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let show_context = create_rw_signal(true);
    
    v_stack((
        // File header
        container(
            v_stack((
                h_stack((
                    label({
                        let path = data.file_path.clone();
                        move || path.clone()
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.foreground"))
                            .font_family("monospace".to_string())
                            .font_size(13.0)
                            .font_bold()
                            .flex_grow(1.0)
                    }),
                    
                    // Stats
                    label({
                        let added = data.total_additions;
                        let deleted = data.total_deletions;
                        move || format!("+{} -{}", added, deleted)
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.dim"))
                            .font_family("monospace".to_string())
                            .font_size(11.0)
                    }),
                ))
                .style(|s| s.margin_bottom(6.0)),
                
                // Renamed file indicator
                container(
                    if let Some(old_path) = data.old_path.clone() {
                        label(move || format!("renamed from: {}", old_path))
                            .style(move |s| {
                                let cfg = config();
                                s.color(cfg.color("editor.dim"))
                                    .font_size(11.0)
                                    .font_style(floem::text::Style::Italic)
                            })
                    } else {
                        label(|| String::new())
                            .style(|s| s.display(floem::style::Display::None))
                    }
                ),
            ))
        )
        .style(move |s| {
            let cfg = config();
            s.padding(12.0)
                .border_bottom(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("editor.background"))
        }),
        
        // Diff content
        scroll(
            container(
                label({
                    let lines = data.lines.clone();
                    let show_ctx = show_context;
                    move || {
                        lines.iter()
                            .filter(|line| {
                                if show_ctx.get() {
                                    true
                                } else {
                                    line.line_type != DiffLineType::Context
                                }
                            })
                            .map(|line| {
                                let prefix = match line.line_type {
                                    DiffLineType::Addition => "+",
                                    DiffLineType::Deletion => "-",
                                    DiffLineType::Context => " ",
                                    DiffLineType::Separator => "@",
                                };
                                
                                let old_num = line.old_line_num
                                    .map(|n| format!("{:4}", n))
                                    .unwrap_or_else(|| "    ".to_string());
                                
                                let new_num = line.new_line_num
                                    .map(|n| format!("{:4}", n))
                                    .unwrap_or_else(|| "    ".to_string());
                                
                                format!("{} {} {} {}", old_num, new_num, prefix, line.content)
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                })
                .style(move |s| {
                    let cfg = config();
                    s.font_family("monospace".to_string())
                        .font_size(12.0)
                        .line_height(1.4)
                        .color(cfg.color("editor.foreground"))
                        .width_full()
                })
            )
            .style(move |s| {
                let cfg = config();
                s.padding(12.0)
                    .background(cfg.color("terminal.background"))
                    .width_full()
            })
        )
        .style(|s| s.flex_grow(1.0).width_full()),
        
        // Action bar
        container(
            h_stack((
                // Toggle context
                container(
                    label(move || {
                        if show_context.get() {
                            "Hide Context"
                        } else {
                            "Show Context"
                        }
                    })
                )
                .on_click_stop(move |_| {
                    show_context.update(|v| *v = !*v);
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(6.0)
                        .padding_horiz(12.0)
                        .border_radius(4.0)
                        .background(cfg.color("panel.current.background"))
                        .color(cfg.color("editor.foreground"))
                        .font_size(11.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(cfg.color("panel.hovered.background")))
                        .margin_right(12.0)
                }),
                
                // Spacer
                container(floem::views::empty())
                    .style(|s| s.flex_grow(1.0)),
                
                // Reject button
                container(
                    label(|| "Reject".to_string())
                )
                .on_click_stop(move |_| {
                    on_reject();
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(8.0)
                        .padding_horiz(16.0)
                        .border_radius(4.0)
                        .background(cfg.color("panel.current.background"))
                        .color(cfg.color("editor.foreground"))
                        .font_size(12.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(cfg.color("panel.hovered.background")))
                        .margin_right(8.0)
                }),
                
                // Apply button
                container(
                    label(|| "Apply Change".to_string())
                )
                .on_click_stop(move |_| {
                    on_apply();
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(8.0)
                        .padding_horiz(16.0)
                        .border_radius(4.0)
                        .background(cfg.color("lapce.button.primary.background"))
                        .color(cfg.color("lapce.button.primary.foreground"))
                        .font_size(12.0)
                        .font_bold()
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(cfg.color("panel.hovered.background")))
                }),
            ))
        )
        .style(move |s| {
            let cfg = config();
            s.padding(12.0)
                .border_top(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("editor.background"))
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .height_full()
            .flex_col()
            .border(1.0)
            .border_radius(6.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background"))
    })
}
