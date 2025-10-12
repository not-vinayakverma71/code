// Welcome Screen - Windsurf Style
// Minimalist, centered design with quick actions

use std::sync::Arc;

use floem::{
    views::{Decorators, container, h_stack, label, v_stack},
    View,
};

use crate::config::LapceConfig;

/// Welcome screen shown when chat is empty - Clean centered style
pub fn welcome_screen(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        // Just the title, centered
        label(|| "TESTING - BUILD WORKS!".to_string())
            .style(move |s| {
                let cfg = config();
                s.font_size(32.0)
                    .color(cfg.color("editor.foreground"))
            })
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

// Removed action_item - not needed anymore
