// Status Badge - Visual status indicators
// Success, Error, Pending, Running states

use std::sync::Arc;
use floem::{
    views::{container, label, h_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Success,
    Error,
    Pending,
    Running,
    Rejected,
}

impl Status {
    pub fn icon(&self) -> &'static str {
        match self {
            Status::Success => "✓",
            Status::Error => "✗",
            Status::Pending => "⏳",
            Status::Running => "▶",
            Status::Rejected => "🚫",
        }
    }
    
    pub fn color(&self, config: &Arc<LapceConfig>) -> floem::peniko::Color {
        match self {
            Status::Success => config.color("testing.iconPassed"),
            Status::Error => config.color("testing.iconFailed"),
            Status::Pending => config.color("list.warningForeground"),
            Status::Running => config.color("progressBar.background"),
            Status::Rejected => config.color("disabledForeground"),
        }
    }
}

pub fn status_badge(
    status: Status,
    text: impl Fn() -> String + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    h_stack((
        label(move || status.icon().to_string())
            .style(move |s| {
                let cfg = config();
                s.color(status.color(&cfg))
                    .margin_right(4.0)
                    .font_size(12.0)
            }),
        
        label(text)
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .font_size(12.0)
            }),
    ))
    .style(|s| {
        s.items_center()
            .padding(4.0)
            .padding_horiz(8.0)
            .border_radius(4.0)
    })
}
