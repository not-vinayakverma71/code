// Welcome Screen
// Pre-IPC Phase 8: Initial view when no messages

use std::sync::Arc;

use floem::{
    reactive::SignalGet,
    views::{Decorators, container, label, v_stack},
    View,
};

use crate::config::LapceConfig;

/// Welcome screen shown when chat is empty
pub fn welcome_screen(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        v_stack((
            // Title
            label(|| "AI Assistant".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.font_size(24.0)
                        .padding_bottom(12.0)
                        .color(cfg.color("editor.foreground"))
                }),
            
            // Subtitle
            label(|| "Ask me anything about your code".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.font_size(14.0)
                        .padding_bottom(24.0)
                        .color(cfg.color("editor.dim"))
                }),
            
            // Features list
            v_stack((
                feature_item("💻 Execute commands", config),
                feature_item("📝 Edit files", config),
                feature_item("🔍 Search codebase", config),
                feature_item("🔧 Use tools", config),
            ))
            .style(|s| s.gap(12.0)),
        ))
        .style(|s| s.items_center())
    )
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .height_full()
            .items_center()
            .justify_center()
            .background(cfg.color("editor.background"))
    })
}

fn feature_item(
    text: &'static str,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    label(move || text.to_string())
        .style(move |s| {
            let cfg = config();
            s.font_size(13.0)
                .color(cfg.color("editor.foreground"))
        })
}
