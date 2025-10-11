// Button primitive - ported from components/ui/button.tsx
use std::sync::Arc;

use floem::{
    event::EventListener,
    reactive::SignalGet,
    style::{CursorStyle, Style},
    views::{Decorators, label, stack},
    IntoView, View,
};

use crate::config::LapceConfig;

#[derive(Clone, Copy, Debug)]
pub enum ButtonVariant {
    Default,
    Destructive,
    Outline,
    Secondary,
    Ghost,
}

#[derive(Clone, Copy, Debug)]
pub enum ButtonSize {
    Default,
    Small,
    Large,
    Icon,
}

pub fn button<V: IntoView + 'static>(
    child: V,
    variant: ButtonVariant,
    size: ButtonSize,
    on_click: impl Fn() + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    stack((child,))
        .on_click_stop(move |_| on_click())
        .style(move |s| {
            let cfg = config();
            let s = s
                .cursor(CursorStyle::Pointer)
                .border_radius(3.0)
                .justify_center()
                .items_center()
                .apply_if(matches!(variant, ButtonVariant::Outline | ButtonVariant::Default | ButtonVariant::Secondary), |s| {
                    s.border(1.0).border_color(cfg.color("lapce.border"))
                });

            let s = match size {
                ButtonSize::Default => s.height(28.0).padding_horiz(12.0),
                ButtonSize::Small => s.height(24.0).padding_horiz(8.0),
                ButtonSize::Large => s.height(32.0).padding_horiz(16.0),
                ButtonSize::Icon => s.width(28.0).height(28.0),
            };

            match variant {
                ButtonVariant::Default => s
                    .background(cfg.color("lapce.button.primary.background"))
                    .color(cfg.color("lapce.button.primary.foreground")),
                ButtonVariant::Destructive => s
                    .background(cfg.color("editor.errorForeground"))
                    .color(cfg.color("editor.background")),
                ButtonVariant::Outline => s
                    .background(cfg.color("editor.background"))
                    .color(cfg.color("editor.foreground")),
                ButtonVariant::Secondary => s
                    .background(cfg.color("panel.background"))
                    .color(cfg.color("editor.foreground")),
                ButtonVariant::Ghost => s
                    .color(cfg.color("editor.foreground")),
            }
        })
}

pub fn text_button(
    text: impl Fn() -> String + 'static,
    variant: ButtonVariant,
    on_click: impl Fn() + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    button(
        label(text),
        variant,
        ButtonSize::Default,
        on_click,
        config,
    )
}
