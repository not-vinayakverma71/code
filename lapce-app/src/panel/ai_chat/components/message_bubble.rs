// Message Bubble - Complete Windsurf-style message container
// Handles user and assistant messages with full styling and actions

use std::sync::Arc;
use floem::{
    peniko::Color,
    reactive::{RwSignal, SignalGet},
    views::{Decorators, container, h_stack, label, svg, v_stack},
    AnyView, IntoView, View,
};

use crate::{
    config::LapceConfig,
    panel::ai_chat::icons::*,
};

#[derive(Clone, Debug)]
pub enum MessageRole {
    User,
    Assistant,
}

pub struct MessageBubbleProps {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
    pub is_streaming: bool,
}

/// Complete message bubble matching Windsurf design
pub fn message_bubble(
    props: MessageBubbleProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let role = props.role.clone();
    let content = props.content.clone();
    let timestamp = props.timestamp;
    let is_streaming = props.is_streaming;
    
    let show_actions = matches!(role, MessageRole::Assistant) && !is_streaming;
    let role_for_style = role.clone();
    
    v_stack((
        // Message header (avatar + role)
        message_header(role.clone(), config),
        
        // Message content  
        message_content(content, role.clone(), config),
        
        // Action bar (copy, like, bookmark, etc.) - only for assistant messages
        container(
            if show_actions {
                action_bar(config).into_any()
            } else {
                label(|| "".to_string()).style(|s| s.width(0.0).height(0.0)).into_any()
            }
        ),
        
        // Timestamp
        label(move || timestamp.clone())
            .style(move |s| {
                let cfg = config();
                s.font_size(11.0)
                    .color(cfg.color("editor.foreground").multiply_alpha(0.5))
                    .margin_top(4.0)
            }),
    ))
    .style(move |s| {
        let cfg = config();
        let bg_color = match role_for_style {
            MessageRole::User => cfg.color("editor.background").multiply_alpha(0.5),
            MessageRole::Assistant => Color::TRANSPARENT,
        };
        
        s.width_full()
            .padding(12.0)
            .border_radius(8.0)
            .background(bg_color)
            .margin_bottom(12.0)
    })
}

/// Message header with avatar and role indicator
fn message_header(
    role: MessageRole,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let role_for_avatar = role.clone();
    let role_for_label = role.clone();
    
    h_stack((
        // Avatar/icon
        container(
            label(move || match role_for_avatar {
                MessageRole::User => "👤".to_string(),
                MessageRole::Assistant => "🤖".to_string(),
            })
            .style(|s| s.font_size(16.0))
        )
        .style(move |s| {
            let cfg = config();
            s.width(32.0)
                .height(32.0)
                .border_radius(16.0)
                .justify_center()
                .items_center()
                .background(cfg.color("editor.background").multiply_alpha(0.3))
        }),
        
        // Role label
        label(move || match role_for_label {
            MessageRole::User => "You".to_string(),
            MessageRole::Assistant => "Assistant".to_string(),
        })
        .style(move |s| {
            let cfg = config();
            s.font_size(13.0)
                .font_weight(floem::text::Weight::SEMIBOLD)
                .color(cfg.color("editor.foreground"))
                .margin_left(8.0)
        }),
    ))
    .style(|s| s.items_center().margin_bottom(8.0))
}

/// Message content area
fn message_content(
    content: String,
    _role: MessageRole,  // Reserved for future use (different styling per role)
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    // TODO: Replace with full markdown renderer
    // For now, simple text display
    label(move || content.clone())
        .style(move |s| {
            let cfg = config();
            s.font_size(14.0)
                .line_height(1.6)
                .color(cfg.color("editor.foreground"))
                .width_full()
                .padding_vert(4.0)
        })
}

/// Action bar with copy, like, bookmark buttons
fn action_bar(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    h_stack((
        // Copy button
        action_button(ICON_COPY, "Copy", config),
        
        // Thumbs up
        action_button(ICON_THUMBS_UP, "Like", config),
        
        // Thumbs down
        action_button(ICON_THUMBS_DOWN, "Dislike", config),
        
        // Bookmark
        action_button(ICON_BOOKMARK, "Save", config),
        
        // More options
        action_button(ICON_ELLIPSIS, "More", config),
    ))
    .style(|s| {
        s.gap(6.0)  // gap-1.5 = 6px
            .margin_top(8.0)
    })
}

/// Individual action button
fn action_button(
    icon: &'static str,
    tooltip: &'static str,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        svg(move || icon.to_string())
            .style(move |s| {
                let cfg = config();
                s.width(12.0)  // h-3 w-3 = 12px
                    .height(12.0)
                    .color(cfg.color("editor.foreground"))
            })
    )
    .style(move |s| {
        let cfg = config();
        s.padding(4.0)
            .border_radius(4.0)
            .cursor(floem::style::CursorStyle::Pointer)
            .color(cfg.color("editor.foreground").multiply_alpha(0.7))
            .hover(|s| {
                let cfg = config();
                s.background(cfg.color("editor.foreground").multiply_alpha(0.1))
                    .color(cfg.color("editor.foreground"))
            })
    })
}

/// Streaming cursor indicator
pub fn streaming_cursor(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    label(|| "▌".to_string())
        .style(move |s| {
            let cfg = config();
            s.font_size(14.0)
                .color(cfg.color("editor.foreground"))
                // TODO: Add blinking animation
        })
}
