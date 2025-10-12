// ToolUseBlock container - ported from components/common/ToolUseBlock.tsx
use std::sync::Arc;

use floem::{
    style::CursorStyle,
    views::{Decorators, container, h_stack},
    IntoView, View,
};

use crate::config::LapceConfig;

pub fn tool_use_block<V: IntoView + 'static>(
    content: V,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(content).style(move |s| {
        let cfg = config();
        s.border(1.0)
            .border_color(cfg.color("lapce.border"))
            .border_radius(4.0)
            .background(cfg.color("editor.background"))
            .width_full()
    })
}

pub fn tool_use_block_header<V: IntoView + 'static>(
    content: V,
    on_click: Option<impl Fn() + 'static>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let header = h_stack((content,)).style(move |s| {
        let cfg = config();
        s.padding_horiz(12.0)
            .padding_vert(8.0)
            .width_full()
            .items_center()
            .background(cfg.color("panel.background"))
            .border_bottom(1.0)
            .border_color(cfg.color("lapce.border"))
    });
    
    if let Some(on_click) = on_click {
        header
            .on_click_stop(move |_| on_click())
            .style(|s| s.cursor(CursorStyle::Pointer))
    } else {
        header.style(|s| s)
    }
}
