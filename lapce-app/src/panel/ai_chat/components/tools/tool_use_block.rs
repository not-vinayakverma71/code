// Tool Use Block - Generic container for tool execution display
// Collapsible header with status, content area for tool-specific details

use std::sync::Arc;
use floem::{
    reactive::{RwSignal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::{
    config::LapceConfig,
    panel::ai_chat::components::messages::{Status, status_badge},
};

#[derive(Debug, Clone)]
pub struct ToolUseBlockProps {
    pub tool_name: String,
    pub icon: String,
    pub status: Status,
    pub is_expanded: RwSignal<bool>,
}

/// Generic tool use block container
/// Header shows tool name + status, body shows tool-specific content
pub fn tool_use_block(
    props: ToolUseBlockProps,
    content: impl View + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let ToolUseBlockProps { tool_name, icon, status, is_expanded } = props;
    
    v_stack((
        // Header (clickable to expand/collapse)
        h_stack((
            // Expand/collapse chevron
            label(move || if is_expanded.get() { "▼" } else { "▶" })
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.dim"))
                        .font_size(10.0)
                        .margin_right(8.0)
                }),
            
            // Tool icon
            label(move || icon.clone())
                .style(|s| s.font_size(14.0).margin_right(8.0)),
            
            // Tool name
            label(move || tool_name.clone())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(13.0)
                        .font_bold()
                        .margin_right(12.0)
                }),
            
            // Status badge
            status_badge(
                status,
                move || match status {
                    Status::Success => "Success".to_string(),
                    Status::Error => "Error".to_string(),
                    Status::Pending => "Pending".to_string(),
                    Status::Running => "Running".to_string(),
                    Status::Rejected => "Rejected".to_string(),
                },
                config
            ),
        ))
        .on_click_stop(move |_| {
            is_expanded.update(|v| *v = !*v);
        })
        .style(move |s| {
            let cfg = config();
            s.padding(12.0)
                .items_center()
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(move |s| s.background(cfg.color("list.hoverBackground")))
        }),
        
        // Content area (visible when expanded)
        container(content)
            .style(move |s| {
                let cfg = config();
                let base = s
                    .padding(12.0)
                    .border_left(2.0)
                    .border_color(cfg.color("lapce.border"))
                    .margin_left(20.0);
                
                if is_expanded.get() {
                    base
                } else {
                    base.display(floem::style::Display::None)
                }
            }),
    ))
    .style(move |s| {
        let cfg = config();
        s.margin_bottom(8.0)
            .border(1.0)
            .border_radius(6.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background"))
    })
}

/// Simple text content for tool blocks
pub fn tool_text_content(
    text: impl Fn() -> String + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    label(text)
        .style(move |s| {
            let cfg = config();
            s.color(cfg.color("editor.foreground"))
                .font_size(12.0)
                .line_height(1.5)
        })
}

/// Metadata row (key: value pairs)
pub fn tool_metadata_row(
    key: impl Fn() -> String + 'static,
    value: impl Fn() -> String + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    h_stack((
        label(key)
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.dim"))
                    .font_size(11.0)
                    .margin_right(8.0)
                    .min_width(80.0)
            }),
        
        label(value)
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .font_size(11.0)
            }),
    ))
    .style(|s| s.margin_bottom(6.0))
}
