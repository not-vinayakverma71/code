// Welcome Screen - Enhanced empty state for AI chat
// Shown when no messages exist yet

use std::sync::Arc;
use floem::{
    peniko::Color,
    views::{Decorators, container, h_stack, label, svg, v_stack},
    View,
};

use crate::{
    config::LapceConfig,
    panel::ai_chat::icons::*,
};

/// Enhanced welcome screen matching Windsurf style
pub fn welcome_screen_v2(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        // Logo/Icon area
        welcome_logo(config),
        
        // Welcome message
        label(|| "Welcome to AI Chat".to_string())
            .style(move |s| {
                let cfg = config();
                s.font_size(24.0)
                    .font_weight(floem::text::Weight::SEMIBOLD)
                    .color(cfg.color("editor.foreground"))
                    .margin_bottom(8.0)
            }),
        
        // Subtitle
        label(|| "Ask me anything to get started".to_string())
            .style(move |s| {
                let cfg = config();
                s.font_size(14.0)
                    .color(cfg.color("editor.foreground").multiply_alpha(0.7))
                    .margin_bottom(32.0)
            }),
        
        // Suggested prompts
        suggested_prompts(config),
    ))
    .style(|s| {
        s.width_full()
            .height_full()
            .flex_col()
            .justify_center()
            .items_center()
            .padding(24.0)
    })
}

/// Welcome logo/icon
fn welcome_logo(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        label(|| "🤖".to_string())
            .style(|s| s.font_size(64.0))
    )
    .style(move |s| {
        let cfg = config();
        s.width(120.0)
            .height(120.0)
            .border_radius(60.0)
            .justify_center()
            .items_center()
            .background(cfg.color("editor.background").multiply_alpha(0.3))
            .margin_bottom(24.0)
    })
}

/// Grid of suggested prompts
fn suggested_prompts(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        label(|| "Try asking:".to_string())
            .style(move |s| {
                let cfg = config();
                s.font_size(13.0)
                    .font_weight(floem::text::Weight::SEMIBOLD)
                    .color(cfg.color("editor.foreground"))
                    .margin_bottom(12.0)
            }),
        
        // Prompt suggestions
        v_stack((
            prompt_card("Help me write a function", ICON_CODE, config),
            prompt_card("Explain this code", ICON_SEARCH, config),
            prompt_card("Find and fix bugs", ICON_TERMINAL, config),
            prompt_card("Refactor for better performance", ICON_CHART, config),
        ))
        .style(|s| s.gap(8.0)),
    ))
    .style(|s| {
        s.max_width(500.0)
            .width_full()
    })
}

/// Individual prompt suggestion card
fn prompt_card(
    text: &'static str,
    icon: &'static str,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        h_stack((
            // Icon
            svg(move || icon.to_string())
                .style(move |s| {
                    let cfg = config();
                    s.width(16.0)
                        .height(16.0)
                        .color(cfg.color("editor.foreground").multiply_alpha(0.7))
                }),
            
            // Text
            label(move || text.to_string())
                .style(move |s| {
                    let cfg = config();
                    s.font_size(14.0)
                        .color(cfg.color("editor.foreground"))
                        .margin_left(12.0)
                }),
        ))
        .style(|s| s.items_center())
    )
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .padding(16.0)
            .border_radius(8.0)
            .border(1.0)
            .border_color(cfg.color("lapce.border"))
            .cursor(floem::style::CursorStyle::Pointer)
            .hover(|s| {
                let cfg = config();
                s.background(cfg.color("editor.foreground").multiply_alpha(0.05))
                    .border_color(cfg.color("editor.foreground").multiply_alpha(0.3))
            })
    })
}
