// Streaming Text - Character-by-character reveal animation
// Used for AI responses that stream in real-time

use std::sync::Arc;
use floem::{
    reactive::{RwSignal, SignalGet},
    views::{label, Decorators},
    View,
};
use crate::config::LapceConfig;

/// Displays text with streaming animation (cursor at end while streaming)
pub fn streaming_text(
    text: RwSignal<String>,
    is_streaming: impl Fn() -> bool + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    label(move || {
        let content = text.get();
        if is_streaming() {
            format!("{}▊", content) // Block cursor while streaming
        } else {
            content
        }
    })
    .style(move |s| {
        let cfg = config();
        s.color(cfg.color("editor.foreground"))
            .font_size(13.0)
            .line_height(1.6)
    })
}
