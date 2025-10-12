// Inline Controls - Message-level action buttons
// Regenerate, copy, edit, delete controls for messages

use std::sync::Arc;
use floem::{
    views::{container, h_stack, label, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
}

/// Inline message controls
/// Shows action buttons for individual messages
pub fn inline_controls(
    role: MessageRole,
    on_copy: impl Fn() + 'static + Copy,
    on_edit: impl Fn() + 'static + Copy,
    on_regenerate: impl Fn() + 'static + Copy,
    on_delete: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    h_stack((
        // Copy button (always available)
        container(
            label(|| "📋".to_string())
        )
        .on_click_stop(move |_| {
            on_copy();
        })
        .style(move |s| {
            let cfg = config();
            s.padding(4.0)
                .padding_horiz(8.0)
                .border_radius(3.0)
                .background(cfg.color("panel.current.background"))
                .color(cfg.color("editor.foreground"))
                .font_size(12.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| s.background(cfg.color("panel.hovered.background")))
                .margin_right(4.0)
        }),
        
        // Edit button (user messages only)
        if role == MessageRole::User {
            container(
                label(|| "✏️".to_string())
            )
            .on_click_stop(move |_| {
                on_edit();
            })
            .style(move |s| {
                let cfg = config();
                s.padding(4.0)
                    .padding_horiz(8.0)
                    .border_radius(3.0)
                    .background(cfg.color("panel.current.background"))
                    .color(cfg.color("editor.foreground"))
                    .font_size(12.0)
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(cfg.color("panel.hovered.background")))
                    .margin_right(4.0)
            })
        } else {
            container(floem::views::empty())
                .style(|s| s.display(floem::style::Display::None))
        },
        
        // Regenerate button (assistant messages only)
        if role == MessageRole::Assistant {
            container(
                label(|| "🔄".to_string())
            )
            .on_click_stop(move |_| {
                on_regenerate();
            })
            .style(move |s| {
                let cfg = config();
                s.padding(4.0)
                    .padding_horiz(8.0)
                    .border_radius(3.0)
                    .background(cfg.color("panel.current.background"))
                    .color(cfg.color("editor.foreground"))
                    .font_size(12.0)
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(cfg.color("panel.hovered.background")))
                    .margin_right(4.0)
            })
        } else {
            container(floem::views::empty())
                .style(|s| s.display(floem::style::Display::None))
        },
        
        // Delete button (always available)
        container(
            label(|| "🗑️".to_string())
        )
        .on_click_stop(move |_| {
            on_delete();
        })
        .style(move |s| {
            let cfg = config();
            s.padding(4.0)
                .padding_horiz(8.0)
                .border_radius(3.0)
                .background(cfg.color("list.errorForeground"))
                .color(cfg.color("editor.background"))
                .font_size(12.0)
                .cursor(floem::style::CursorStyle::Pointer)
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.padding(4.0)
            .border_radius(4.0)
            .background(cfg.color("editor.background"))
    })
}

/// Mini inline controls for hover state
/// Compact version that appears on message hover
pub fn mini_inline_controls(
    on_copy: impl Fn() + 'static + Copy,
    on_more: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    h_stack((
        container(
            label(|| "📋".to_string())
        )
        .on_click_stop(move |_| {
            on_copy();
        })
        .style(move |s| {
            let cfg = config();
            s.padding(3.0)
                .padding_horiz(6.0)
                .border_radius(3.0)
                .background(cfg.color("editor.background"))
                .color(cfg.color("editor.foreground"))
                .font_size(11.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| s.background(cfg.color("list.hoverBackground")))
                .margin_right(4.0)
        }),
        
        container(
            label(|| "⋯".to_string())
        )
        .on_click_stop(move |_| {
            on_more();
        })
        .style(move |s| {
            let cfg = config();
            s.padding(3.0)
                .padding_horiz(6.0)
                .border_radius(3.0)
                .background(cfg.color("editor.background"))
                .color(cfg.color("editor.foreground"))
                .font_size(11.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| s.background(cfg.color("list.hoverBackground")))
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.padding(2.0)
            .border_radius(3.0)
            .background(cfg.color("editor.background"))
            .border(1.0)
            .border_color(cfg.color("lapce.border"))
    })
}
