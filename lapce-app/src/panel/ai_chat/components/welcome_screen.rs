// Welcome Screen - Windsurf Style
// Minimalist, centered design with quick actions

use std::sync::Arc;

use floem::{
    views::{Decorators, container, h_stack, label, v_stack},
    View,
};

use crate::config::LapceConfig;

/// Welcome screen shown when chat is empty - Completely clean
pub fn welcome_screen(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(label(|| "".to_string()))
    .style(|s| {
        s.width_full()
            .height_full()
            .background(floem::peniko::Color::from_rgb8(0x1a, 0x1a, 0x1a))
    })
}

// Removed action_item - not needed anymore
