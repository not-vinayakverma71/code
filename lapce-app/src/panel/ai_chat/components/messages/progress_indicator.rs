// Progress Indicator - Streaming/loading animation
// Floem equivalent of Codex's ProgressIndicator component

use std::sync::Arc;
use floem::{
    reactive::SignalGet,
    views::{container, label, Decorators},
    View,
};
use crate::config::LapceConfig;

/// Simple spinning/pulsing progress indicator
pub fn progress_indicator(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        label(|| "⋯".to_string()) // Animated dots
    )
    .style(move |s| {
        let cfg = config();
        s.width(16.0)
            .height(16.0)
            .display(floem::style::Display::Flex)
            .items_center()
            .justify_center()
            .color(cfg.color("progressBar.background"))
            .font_size(12.0)
    })
}

/// Thinking indicator with animated dots
pub fn thinking_indicator(
    elapsed_secs: impl Fn() -> u64 + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    label(move || {
        let secs = elapsed_secs();
        let dots = match secs % 4 {
            0 => "",
            1 => ".",
            2 => "..",
            _ => "...",
        };
        format!("Thinking{}", dots)
    })
    .style(move |s| {
        let cfg = config();
        s.color(cfg.color("editor.dim"))
            .font_size(12.0)
    })
}
