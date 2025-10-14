// Search Panel - Search within chat messages
// Find and navigate through chat history

use std::sync::Arc;
use floem::{
    reactive::{RwSignal, SignalGet},
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub message_id: String,
    pub excerpt: String,
    pub match_index: usize,
    pub total_matches: usize,
}

/// Search panel component
/// Search bar with navigation controls
pub fn search_panel(
    query: RwSignal<String>,
    current_match: RwSignal<usize>,
    total_matches: RwSignal<usize>,
    on_search: impl Fn(String) + 'static + Copy,
    on_next: impl Fn() + 'static + Copy,
    on_prev: impl Fn() + 'static + Copy,
    on_close: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    h_stack((
        // Search icon
        label(|| "🔍".to_string())
            .style(|s| s.margin_right(8.0)),
        
        // Search input (simplified - will be replaced with actual text input)
        container(
            label(move || {
                let q = query.get();
                if q.is_empty() {
                    "Search messages...".to_string()
                } else {
                    q
                }
            })
        )
        .on_click_stop(move |_| {
            // Focus search input
            // TODO: Wire to actual text editor when integrated
        })
        .style(move |s| {
            let cfg = config();
            let q = query.get();
            s.padding(6.0)
                .padding_horiz(10.0)
                .flex_grow(1.0)
                .border(1.0)
                .border_radius(4.0)
                .border_color(cfg.color("input.border"))
                .background(cfg.color("input.background"))
                .color(if q.is_empty() {
                    cfg.color("input.placeholderForeground")
                } else {
                    cfg.color("input.foreground")
                })
                .font_size(12.0)
                .cursor(floem::style::CursorStyle::Text)
                .margin_right(12.0)
        }),
        
        // Match counter
        label(move || {
            let current = current_match.get();
            let total = total_matches.get();
            if total > 0 {
                format!("{}/{}", current, total)
            } else {
                "No matches".to_string()
            }
        })
        .style(move |s| {
            let cfg = config();
            s.color(cfg.color("editor.dim"))
                .font_size(11.0)
                .margin_right(12.0)
        }),
        
        // Previous match button
        container(
            label(|| "▲".to_string())
        )
        .on_click_stop(move |_| {
            if total_matches.get() > 0 {
                on_prev();
            }
        })
        .style(move |s| {
            let cfg = config();
            let has_matches = total_matches.get() > 0;
            s.padding(4.0)
                .padding_horiz(8.0)
                .border_radius(3.0)
                .background(cfg.color("panel.current.background"))
                .color(if has_matches {
                    cfg.color("editor.foreground")
                } else {
                    cfg.color("editor.dim")
                })
                .font_size(10.0)
                .cursor(if has_matches {
                    floem::style::CursorStyle::Pointer
                } else {
                    floem::style::CursorStyle::Default
                })
                .margin_right(4.0)
        }),
        
        // Next match button
        container(
            label(|| "▼".to_string())
        )
        .on_click_stop(move |_| {
            if total_matches.get() > 0 {
                on_next();
            }
        })
        .style(move |s| {
            let cfg = config();
            let has_matches = total_matches.get() > 0;
            s.padding(4.0)
                .padding_horiz(8.0)
                .border_radius(3.0)
                .background(cfg.color("panel.current.background"))
                .color(if has_matches {
                    cfg.color("editor.foreground")
                } else {
                    cfg.color("editor.dim")
                })
                .font_size(10.0)
                .cursor(if has_matches {
                    floem::style::CursorStyle::Pointer
                } else {
                    floem::style::CursorStyle::Default
                })
                .margin_right(12.0)
        }),
        
        // Close button
        container(
            label(|| "✕".to_string())
        )
        .on_click_stop(move |_| {
            on_close();
        })
        .style(move |s| {
            let cfg = config();
            s.padding(4.0)
                .padding_horiz(8.0)
                .border_radius(3.0)
                .color(cfg.color("editor.foreground"))
                .font_size(12.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| s.background(cfg.color("list.hoverBackground")))
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.padding(8.0)
            .border_bottom(1.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background"))
            .items_center()
    })
}

/// Search result item
/// Individual search result in results list
pub fn search_result_item(
    result: SearchResult,
    is_selected: bool,
    on_click: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        v_stack((
            // Match indicator
            label(move || format!("Match {}/{}", result.match_index, result.total_matches))
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.dim"))
                        .font_size(10.0)
                        .margin_bottom(4.0)
                }),
            
            // Excerpt
            label(move || result.excerpt.clone())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(11.0)
                        .line_height(1.5)
                }),
        ))
    )
    .on_click_stop(move |_| {
        on_click();
    })
    .style(move |s| {
        let cfg = config();
        s.padding(8.0)
            .width_full()
            .border_radius(4.0)
            .background(if is_selected {
                cfg.color("list.activeSelectionBackground")
            } else {
                cfg.color("editor.background")
            })
            .cursor(floem::style::CursorStyle::Pointer)
            .hover(|s| s.background(cfg.color("list.hoverBackground")))
            .margin_bottom(4.0)
    })
}
