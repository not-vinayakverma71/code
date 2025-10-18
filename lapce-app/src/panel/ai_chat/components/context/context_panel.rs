// Context Panel - Shows attached files, folders, and context items
// Display active context for the current conversation

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, scroll, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextItemType {
    File,
    Folder,
    Selection,
    Symbol,
    Url,
}

impl ContextItemType {
    pub fn icon(&self) -> &'static str {
        match self {
            ContextItemType::File => "📄",
            ContextItemType::Folder => "📁",
            ContextItemType::Selection => "📋",
            ContextItemType::Symbol => "🔤",
            ContextItemType::Url => "🔗",
        }
    }
    
    pub fn label(&self) -> &'static str {
        match self {
            ContextItemType::File => "File",
            ContextItemType::Folder => "Folder",
            ContextItemType::Selection => "Selection",
            ContextItemType::Symbol => "Symbol",
            ContextItemType::Url => "URL",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextItem {
    pub item_type: ContextItemType,
    pub name: String,
    pub path: String,
    pub size_kb: Option<usize>,
    pub line_count: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ContextPanelData {
    pub items: Vec<ContextItem>,
    pub total_size_kb: usize,
    pub total_tokens: usize,
}

/// Context panel component
/// Shows all attached context items with remove functionality
pub fn context_panel(
    data: ContextPanelData,
    on_remove_item: impl Fn(usize) + 'static + Copy,
    on_clear_all: impl Fn() + 'static + Copy,
    on_add_file: impl Fn() + 'static + Copy,
    on_add_folder: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let is_expanded = create_rw_signal(true);
    
    v_stack((
        // Header
        h_stack((
            // Expand/collapse
            label(move || if is_expanded.get() { "▼" } else { "▶" })
                .on_click_stop(move |_| {
                    is_expanded.update(|v| *v = !*v);
                })
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(12.0)
                        .margin_right(8.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                }),
            
            label(|| "Context".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(13.0)
                        .font_bold()
                        .flex_grow(1.0)
                }),
            
            // Stats
            label({
                let items = data.items.len();
                let tokens = data.total_tokens;
                move || format!("{} items • {} tokens", items, format_number(tokens))
            })
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.dim"))
                    .font_size(10.0)
                    .margin_right(12.0)
            }),
            
            // Add buttons
            h_stack((
                container(
                    label(|| "+📄".to_string())
                )
                .on_click_stop(move |_| {
                    on_add_file();
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(4.0)
                        .padding_horiz(8.0)
                        .border_radius(3.0)
                        .background(cfg.color("panel.current.background"))
                        .color(cfg.color("editor.foreground"))
                        .font_size(11.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(cfg.color("panel.hovered.background")))
                        .margin_right(6.0)
                }),
                
                container(
                    label(|| "+📁".to_string())
                )
                .on_click_stop(move |_| {
                    on_add_folder();
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(4.0)
                        .padding_horiz(8.0)
                        .border_radius(3.0)
                        .background(cfg.color("panel.current.background"))
                        .color(cfg.color("editor.foreground"))
                        .font_size(11.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(cfg.color("panel.hovered.background")))
                }),
            )),
        ))
        .style(move |s| {
            let cfg = config();
            s.padding(12.0)
                .border_bottom(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("panel.background"))
                .items_center()
        }),
        
        // Context items list
        container(
            container(
                if !data.items.is_empty() {
                    scroll(
                    v_stack((
                        // Items
                        label({
                            let items = data.items.clone();
                            move || items.iter().map(|item| {
                                let size_info = if let Some(size) = item.size_kb {
                                    format!(" ({} KB)", size)
                                } else if let Some(lines) = item.line_count {
                                    format!(" ({} lines)", lines)
                                } else {
                                    String::new()
                                };
                                
                                format!("{} {} {}{}", 
                                    item.item_type.icon(),
                                    item.item_type.label(),
                                    item.name,
                                    size_info
                                )
                            }).collect::<Vec<_>>().join("\n")
                        })
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_size(11.0)
                                .line_height(1.8)
                                .margin_bottom(12.0)
                        }),
                        
                        // Clear all button
                        container(
                            label(|| "Clear All".to_string())
                        )
                        .on_click_stop(move |_| {
                            on_clear_all();
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
                        }),
                    ))
                    .style(|s| s.padding(12.0))
                )
                .style(|s| s.max_height(400.0).width_full())
            } else {
                scroll(
                    container(
                        label(|| "No context attached\n\nClick +📄 to add files or +📁 to add folders".to_string())
                    )
                    .style(move |s| {
                        let cfg = config();
                        s.padding(24.0)
                            .color(cfg.color("editor.dim"))
                            .font_size(11.0)
                    })
                )
                .style(|s| s.max_height(400.0).width_full())
            }
            )
        )
        .style(move |s| {
            if is_expanded.get() {
                s
            } else {
                s.display(floem::style::Display::None)
            }
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .border(1.0)
            .border_radius(6.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background"))
            .margin_bottom(16.0)
    })
}

fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
