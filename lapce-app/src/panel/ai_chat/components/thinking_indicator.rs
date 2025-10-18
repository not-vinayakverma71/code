// Thinking Indicator - "Diving..." animation
// Shimmer effect matching Windsurf's gradient animation

use std::sync::Arc;
use floem::{
    peniko::Color,
    views::{Decorators, container, label},
    View,
};

use crate::config::LapceConfig;

/// Windsurf-style thinking indicator with shimmer animation
/// Shows "Diving..." or custom text with animated gradient
pub fn thinking_indicator(
    text: Option<String>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let display_text = text.unwrap_or_else(|| "Diving...".to_string());
    
    container(
        label(move || display_text.clone())
            .style(move |s| {
                let cfg = config();
                s.font_size(14.0)
                    .color(cfg.color("editor.foreground").multiply_alpha(0.7))
                    .font_weight(floem::text::Weight::MEDIUM)
                    // TODO: Add shimmer animation
                    // CSS equivalent:
                    // animation: shine 1s linear infinite;
                    // background-image: linear-gradient(120deg,
                    //     color-mix(in srgb, currentColor 100%, transparent) 35%,
                    //     color-mix(in srgb, var(--c) var(--o), transparent) 50%,
                    //     color-mix(in srgb, currentColor 100%, transparent) 65%
                    // );
                    // background-size: 200% 100%;
            })
    )
    .style(move |s| {
        let cfg = config();
        s.padding(12.0)
            .border_radius(8.0)
            .background(cfg.color("editor.background").multiply_alpha(0.3))
            .min_width(120.0)
    })
}

/// Compact thinking indicator (smaller, for inline use)
pub fn thinking_indicator_compact(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    // Three animated dots
    label(|| "...".to_string())
        .style(move |s| {
            let cfg = config();
            s.font_size(16.0)
                .color(cfg.color("editor.foreground").multiply_alpha(0.7))
                // TODO: Add dot animation (bounce/fade)
        })
}

/// Shimmer text effect (for custom text with animation)
pub fn shimmer_text(
    text: String,
    min_width: f64,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        label(move || text.clone())
            .style(move |s| {
                let cfg = config();
                s.font_size(14.0)
                    .color(cfg.color("editor.foreground").multiply_alpha(0.6))
            })
    )
    .style(move |s| {
        s.min_width(min_width)
            .padding_horiz(8.0)
            .padding_vert(4.0)
    })
}
