// File Picker - File/folder selection dialog
// Browse and select files to attach to context

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, scroll, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone)]
pub struct FilePickerItem {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub is_selected: bool,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct FilePickerData {
    pub title: String,
    pub current_path: String,
    pub items: Vec<FilePickerItem>,
    pub multi_select: bool,
}

/// File picker component
/// Dialog for browsing and selecting files/folders
pub fn file_picker(
    data: FilePickerData,
    on_select: impl Fn(Vec<String>) + 'static + Copy,
    on_cancel: impl Fn() + 'static + Copy,
    on_navigate: impl Fn(String) + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let selected_items = create_rw_signal::<Vec<String>>(vec![]);
    let search_query = create_rw_signal(String::new());
    
    v_stack((
        // Header
        h_stack((
            label({
                let title = data.title.clone();
                move || title.clone()
            })
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .font_size(14.0)
                    .font_bold()
                    .flex_grow(1.0)
            }),
            
            // Close button
            container(
                label(|| "✕".to_string())
            )
            .on_click_stop(move |_| {
                on_cancel();
            })
            .style(move |s| {
                let cfg = config();
                s.padding(6.0)
                    .padding_horiz(10.0)
                    .border_radius(3.0)
                    .color(cfg.color("editor.foreground"))
                    .font_size(14.0)
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(cfg.color("list.hoverBackground")))
            }),
        ))
        .style(move |s| {
            let cfg = config();
            s.padding(16.0)
                .border_bottom(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("titleBar.activeBackground"))
                .items_center()
        }),
        
        // Current path
        container(
            h_stack((
                label(|| "📁".to_string())
                    .style(|s| s.margin_right(8.0)),
                
                label({
                    let path = data.current_path.clone();
                    move || path.clone()
                })
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_family("monospace".to_string())
                        .font_size(11.0)
                        .flex_grow(1.0)
                }),
                
                // Up directory button
                container(
                    label(|| "↑ Up".to_string())
                )
                .on_click_stop({
                    let current = data.current_path.clone();
                    move |_| {
                        if let Some(parent) = std::path::Path::new(&current).parent() {
                            on_navigate(parent.to_string_lossy().to_string());
                        }
                    }
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(4.0)
                        .padding_horiz(10.0)
                        .border_radius(3.0)
                        .background(cfg.color("panel.current.background"))
                        .color(cfg.color("editor.foreground"))
                        .font_size(11.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                }),
            ))
        )
        .style(move |s| {
            let cfg = config();
            s.padding(12.0)
                .border_bottom(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("editor.background"))
        }),
        
        // File list
        scroll(
            container(
                label({
                    let items = data.items.clone();
                    let query = search_query;
                    move || {
                        let q = query.get();
                        items.iter()
                            .filter(|item| {
                                if q.is_empty() {
                                    true
                                } else {
                                    item.name.to_lowercase().contains(&q.to_lowercase())
                                }
                            })
                            .map(|item| {
                                let indent = "  ".repeat(item.depth);
                                let icon = if item.is_dir { "📁" } else { "📄" };
                                let marker = if item.is_selected { "☑" } else { "☐" };
                                
                                format!("{} {} {} {}", marker, indent, icon, item.name)
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                })
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_family("monospace".to_string())
                        .font_size(11.0)
                        .line_height(1.8)
                        .width_full()
                })
            )
            .style(|s| s.padding(12.0))
        )
        .style(|s| s.flex_grow(1.0).width_full()),
        
        // Footer with actions
        h_stack((
            // Selection count
            label({
                let count = selected_items.get().len();
                move || format!("{} selected", count)
            })
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.dim"))
                    .font_size(11.0)
                    .flex_grow(1.0)
            }),
            
            // Cancel button
            container(
                label(|| "Cancel".to_string())
            )
            .on_click_stop(move |_| {
                on_cancel();
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
                    .margin_right(8.0)
            }),
            
            // Select button
            container(
                label(|| "Select".to_string())
            )
            .on_click_stop(move |_| {
                let selected = selected_items.get();
                if !selected.is_empty() {
                    on_select(selected);
                }
            })
            .style(move |s| {
                let cfg = config();
                let has_selection = !selected_items.get().is_empty();
                s.padding(8.0)
                    .padding_horiz(16.0)
                    .border_radius(4.0)
                    .background(if has_selection {
                        cfg.color("lapce.button.primary.background")
                    } else {
                        cfg.color("panel.current.background")
                    })
                    .color(if has_selection {
                        cfg.color("lapce.button.primary.foreground")
                    } else {
                        cfg.color("editor.dim")
                    })
                    .font_size(12.0)
                    .font_bold()
                    .cursor(if has_selection {
                        floem::style::CursorStyle::Pointer
                    } else {
                        floem::style::CursorStyle::Default
                    })
            }),
        ))
        .style(move |s| {
            let cfg = config();
            s.padding(16.0)
                .border_top(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("editor.background"))
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.width(600.0)
            .height(500.0)
            .border(1.0)
            .border_radius(8.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background"))
    })
}
