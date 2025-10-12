// Diff Hunk - A single section of changes
// Represents a contiguous block of additions/deletions

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone)]
pub struct DiffHunkData {
    pub header: String, // e.g., "@@ -10,7 +10,8 @@ function_name"
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub additions: Vec<String>,
    pub deletions: Vec<String>,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

impl DiffHunkData {
    pub fn total_changes(&self) -> usize {
        self.additions.len() + self.deletions.len()
    }
}

/// Diff hunk component
/// Shows a single change section with collapsible context
pub fn diff_hunk(
    data: DiffHunkData,
    on_accept: impl Fn() + 'static + Copy,
    on_reject: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let is_expanded = create_rw_signal(true);
    
    v_stack((
        // Hunk header
        h_stack((
            // Expand/collapse chevron
            label(move || if is_expanded.get() { "▼" } else { "▶" })
                .style(|s| s.margin_right(8.0)),
            
            // Header info
            label({
                let header = data.header.clone();
                move || header.clone()
            })
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.dim"))
                    .font_family("monospace".to_string())
                    .font_size(11.0)
                    .flex_grow(1.0)
            }),
            
            // Change count
            label({
                let additions = data.additions.len();
                let deletions = data.deletions.len();
                move || format!("+{} -{}", additions, deletions)
            })
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.dim"))
                    .font_family("monospace".to_string())
                    .font_size(10.0)
                    .margin_right(12.0)
            }),
            
            // Quick actions
            h_stack((
                container(
                    label(|| "✓".to_string())
                )
                .on_click_stop(move |_| {
                    on_accept();
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(4.0)
                        .padding_horiz(8.0)
                        .border_radius(3.0)
                        .background(cfg.color("testing.iconPassed"))
                        .color(cfg.color("editor.background"))
                        .font_size(12.0)
                        .font_bold()
                        .cursor(floem::style::CursorStyle::Pointer)
                        .margin_right(4.0)
                }),
                
                container(
                    label(|| "✗".to_string())
                )
                .on_click_stop(move |_| {
                    on_reject();
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(4.0)
                        .padding_horiz(8.0)
                        .border_radius(3.0)
                        .background(cfg.color("list.errorForeground"))
                        .color(cfg.color("editor.background"))
                        .font_size(12.0)
                        .font_bold()
                        .cursor(floem::style::CursorStyle::Pointer)
                }),
            )),
        ))
        .on_click_stop(move |_| {
            is_expanded.update(|v| *v = !*v);
        })
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .background(cfg.color("titleBar.inactiveBackground"))
                .border_radius(4.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .items_center()
                .hover(|s| s.background(cfg.color("list.hoverBackground")))
        }),
        
        // Hunk content (collapsible)
        container(
            v_stack((
                // Context before (if any)
                if !data.context_before.is_empty() {
                    container(
                        label({
                            let lines = data.context_before.clone();
                            move || lines.iter()
                                .map(|l| format!("  {}", l))
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.dim"))
                                .font_family("monospace".to_string())
                                .font_size(11.0)
                                .line_height(1.4)
                        })
                    )
                    .style(|s| s.padding(4.0).margin_bottom(4.0))
                } else {
                    container(floem::views::empty())
                        .style(|s| s.display(floem::style::Display::None))
                },
                
                // Deletions
                if !data.deletions.is_empty() {
                    container(
                        label({
                            let lines = data.deletions.clone();
                            move || lines.iter()
                                .map(|l| format!("- {}", l))
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("list.errorForeground"))
                                .font_family("monospace".to_string())
                                .font_size(11.0)
                                .line_height(1.4)
                        })
                    )
                    .style(move |s| {
                        let cfg = config();
                        s.padding(4.0)
                            .background(cfg.color("diffEditor.removedTextBackground"))
                            .margin_bottom(4.0)
                    })
                } else {
                    container(floem::views::empty())
                        .style(|s| s.display(floem::style::Display::None))
                },
                
                // Additions
                if !data.additions.is_empty() {
                    container(
                        label({
                            let lines = data.additions.clone();
                            move || lines.iter()
                                .map(|l| format!("+ {}", l))
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("testing.iconPassed"))
                                .font_family("monospace".to_string())
                                .font_size(11.0)
                                .line_height(1.4)
                        })
                    )
                    .style(move |s| {
                        let cfg = config();
                        s.padding(4.0)
                            .background(cfg.color("diffEditor.insertedTextBackground"))
                            .margin_bottom(4.0)
                    })
                } else {
                    container(floem::views::empty())
                        .style(|s| s.display(floem::style::Display::None))
                },
                
                // Context after (if any)
                if !data.context_after.is_empty() {
                    container(
                        label({
                            let lines = data.context_after.clone();
                            move || lines.iter()
                                .map(|l| format!("  {}", l))
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.dim"))
                                .font_family("monospace".to_string())
                                .font_size(11.0)
                                .line_height(1.4)
                        })
                    )
                    .style(|s| s.padding(4.0))
                } else {
                    container(floem::views::empty())
                        .style(|s| s.display(floem::style::Display::None))
                },
            ))
        )
        .style(move |s| {
            let cfg = config();
            let base = s
                .padding(8.0)
                .margin_top(4.0)
                .border_left(2.0)
                .border_color(cfg.color("lapce.border"));
            
            if is_expanded.get() {
                base
            } else {
                base.display(floem::style::Display::None)
            }
        }),
    ))
    .style(|s| s.margin_bottom(8.0))
}
