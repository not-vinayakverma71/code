// Loading States - Various loading and skeleton screens
// Provide visual feedback during data loading

use std::sync::Arc;
use floem::{
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

/// Spinner loading indicator
/// Animated spinner for loading states
pub fn spinner(
    size: f64,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        label(|| "⏳".to_string())
    )
    .style(move |s| {
        let cfg = config();
        s.width(size)
            .height(size)
            .display(floem::style::Display::Flex)
            .items_center()
            .justify_center()
            .color(cfg.color("editor.foreground"))
            .font_size(size * 0.7)
    })
}

/// Loading message with spinner
pub fn loading_message(
    message: String,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    h_stack((
        spinner(24.0, config),
        
        label(move || message.clone())
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .font_size(13.0)
                    .margin_left(12.0)
            }),
    ))
    .style(|s| s.items_center())
}

/// Skeleton loader for message
/// Placeholder animation while content loads
pub fn message_skeleton(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        // Avatar skeleton
        h_stack((
            container(floem::views::empty())
                .style(move |s| {
                    let cfg = config();
                    s.width(32.0)
                        .height(32.0)
                        .border_radius(16.0)
                        .background(cfg.color("input.border"))
                        .margin_right(12.0)
                }),
            
            v_stack((
                // Line 1
                container(floem::views::empty())
                    .style(move |s| {
                        let cfg = config();
                        s.width(200.0)
                            .height(16.0)
                            .border_radius(4.0)
                            .background(cfg.color("input.border"))
                            .margin_bottom(8.0)
                    }),
                
                // Line 2
                container(floem::views::empty())
                    .style(move |s| {
                        let cfg = config();
                        s.width(300.0)
                            .height(16.0)
                            .border_radius(4.0)
                            .background(cfg.color("input.border"))
                            .margin_bottom(8.0)
                    }),
                
                // Line 3
                container(floem::views::empty())
                    .style(move |s| {
                        let cfg = config();
                        s.width(250.0)
                            .height(16.0)
                            .border_radius(4.0)
                            .background(cfg.color("input.border"))
                    }),
            )),
        )),
    ))
    .style(move |s| {
        let cfg = config();
        s.padding(16.0)
            .width_full()
            .background(cfg.color("editor.background"))
    })
}

/// Empty state component
/// Displayed when there's no content
pub fn empty_state(
    icon: String,
    title: String,
    description: String,
    action_label: Option<String>,
    on_action: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        // Icon
        label(move || icon.clone())
            .style(|s| s.font_size(48.0).margin_bottom(16.0)),
        
        // Title
        label(move || title.clone())
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .font_size(16.0)
                    .font_bold()
                    .margin_bottom(8.0)
            }),
        
        // Description
        label(move || description.clone())
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.dim"))
                    .font_size(13.0)
                    .margin_bottom(24.0)
                    .line_height(1.5)
            }),
        
        // Action button (optional)
        if let Some(action_text) = action_label {
            container(
                label(move || action_text.clone())
            )
            .on_click_stop(move |_| {
                on_action();
            })
            .style(move |s| {
                let cfg = config();
                s.padding(10.0)
                    .padding_horiz(20.0)
                    .border_radius(6.0)
                    .background(cfg.color("lapce.button.primary.background"))
                    .color(cfg.color("lapce.button.primary.foreground"))
                    .font_size(13.0)
                    .font_bold()
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(cfg.color("panel.hovered.background")))
            })
        } else {
            container(floem::views::empty())
                .style(|s| s.display(floem::style::Display::None))
        },
    ))
    .style(move |s| {
        s.padding(48.0)
            .width_full()
            .items_center()
            .justify_center()
    })
}

/// Error state component
/// Displayed when something goes wrong
pub fn error_state(
    error_message: String,
    on_retry: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        label(|| "⚠️".to_string())
            .style(|s| s.font_size(48.0).margin_bottom(16.0)),
        
        label(|| "Something went wrong".to_string())
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("list.errorForeground"))
                    .font_size(16.0)
                    .font_bold()
                    .margin_bottom(8.0)
            }),
        
        label(move || error_message.clone())
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .font_size(13.0)
                    .margin_bottom(24.0)
                    .line_height(1.5)
            }),
        
        container(
            label(|| "Retry".to_string())
        )
        .on_click_stop(move |_| {
            on_retry();
        })
        .style(move |s| {
            let cfg = config();
            s.padding(10.0)
                .padding_horiz(20.0)
                .border_radius(6.0)
                .background(cfg.color("lapce.button.primary.background"))
                .color(cfg.color("lapce.button.primary.foreground"))
                .font_size(13.0)
                .font_bold()
                .cursor(floem::style::CursorStyle::Pointer)
        }),
    ))
    .style(|s| {
        s.padding(48.0)
            .width_full()
            .items_center()
            .justify_center()
    })
}

/// Progress bar
/// Shows progress for long operations
pub fn progress_bar(
    progress: f64, // 0.0 to 100.0
    label_text: Option<String>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        // Label
        if let Some(text) = label_text {
            label(move || text.clone())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(11.0)
                        .margin_bottom(6.0)
                })
        } else {
            label(|| String::new())
                .style(|s| s.display(floem::style::Display::None))
        },
        
        // Progress bar
        container(
            container(floem::views::empty())
                .style(move |s| {
                    let cfg = config();
                    s.width_pct(progress.clamp(0.0, 100.0))
                        .height(6.0)
                        .background(cfg.color("lapce.button.primary.background"))
                        .border_radius(3.0)
                })
        )
        .style(move |s| {
            let cfg = config();
            s.width_full()
                .height(6.0)
                .background(cfg.color("input.border"))
                .border_radius(3.0)
        }),
    ))
}
