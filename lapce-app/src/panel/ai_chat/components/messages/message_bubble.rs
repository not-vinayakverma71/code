// Message Bubble - Container for user/assistant messages
// Styled differently based on sender

use std::sync::Arc;
use floem::{
    reactive::SignalGet,
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

pub struct MessageBubbleProps {
    pub role: MessageRole,
    pub show_avatar: bool,
}

/// Main message bubble container
/// Wraps message content with appropriate styling based on role
pub fn message_bubble(
    props: MessageBubbleProps,
    content: impl View + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let MessageBubbleProps { role, show_avatar } = props;
    
    h_stack((
        // Avatar (optional)
        if show_avatar {
            container(
                label(move || match role {
                    MessageRole::User => "👤".to_string(),
                    MessageRole::Assistant => "🤖".to_string(),
                    MessageRole::System => "ℹ️".to_string(),
                })
            )
            .style(move |s| {
                let cfg = config();
                s.width(32.0)
                    .height(32.0)
                    .border_radius(16.0)
                    .display(floem::style::Display::Flex)
                    .items_center()
                    .justify_center()
                    .background(cfg.color("panel.current.background"))
                    .margin_right(12.0)
                    .flex_shrink(0.0)
            })
        } else {
            container(floem::views::empty())
                .style(|s| s.display(floem::style::Display::None))
        },
        
        // Message content
        container(content)
            .style(move |s| {
                let cfg = config();
                let base = s
                    .padding(12.0)
                    .border_radius(8.0)
                    .max_width_pct(80.0);
                
                match role {
                    MessageRole::User => {
                        base.background(cfg.color("lapce.button.primary.background"))
                            .color(cfg.color("lapce.button.primary.foreground"))
                    }
                    MessageRole::Assistant => {
                        base.background(cfg.color("editor.background"))
                            .border(1.0)
                            .border_color(cfg.color("lapce.border"))
                            .color(cfg.color("editor.foreground"))
                    }
                    MessageRole::System => {
                        base.background(cfg.color("editorInfo.background"))
                            .color(cfg.color("editorInfo.foreground"))
                    }
                }
            }),
    ))
    .style(move |s| {
        let base = s.margin_bottom(16.0);
        match role {
            MessageRole::User => base.justify_end(),
            _ => base.justify_start(),
        }
    })
}

/// Timestamp label for messages
pub fn message_timestamp(
    timestamp: impl Fn() -> String + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    label(timestamp)
        .style(move |s| {
            let cfg = config();
            s.font_size(10.0)
                .color(cfg.color("editor.dim"))
                .margin_top(4.0)
        })
}
