// Badge primitive - ported from components/ui/badge.tsx
use std::sync::Arc;

use floem::{
    views::{Decorators, label, stack},
    IntoView, View,
};

use crate::config::LapceConfig;

#[derive(Clone, Copy, Debug)]
pub enum BadgeVariant {
    Default,
    Secondary,
    Destructive,
    Outline,
}

pub fn badge<V: IntoView + 'static>(
    child: V,
    variant: BadgeVariant,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    stack((child,))
        .style(move |s| {
            let cfg = config();
            let s = s
                .border_radius(12.0)
                .padding_horiz(8.0)
                .padding_vert(2.0)
                .font_size(11.0);

            match variant {
                BadgeVariant::Default => s
                    .background(cfg.color("lapce.button.primary.background"))
                    .color(cfg.color("lapce.button.primary.foreground")),
                BadgeVariant::Secondary => s
                    .background(cfg.color("panel.background"))
                    .color(cfg.color("editor.foreground")),
                BadgeVariant::Destructive => s
                    .background(cfg.color("editor.errorForeground"))
                    .color(cfg.color("editor.background")),
                BadgeVariant::Outline => s
                    .border(1.0)
                    .border_color(cfg.color("lapce.border"))
                    .color(cfg.color("editor.dim")),
            }
        })
}

pub fn text_badge(
    text: impl Fn() -> String + 'static,
    variant: BadgeVariant,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    badge(label(text), variant, config)
}
