// Toast Notifications - Temporary notification messages
// Display success, error, info, and warning toasts

use std::sync::Arc;
use floem::{
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

impl ToastType {
    pub fn icon(&self) -> &'static str {
        match self {
            ToastType::Success => "✅",
            ToastType::Error => "❌",
            ToastType::Warning => "⚠️",
            ToastType::Info => "ℹ️",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToastData {
    pub toast_type: ToastType,
    pub title: String,
    pub message: Option<String>,
    pub duration_ms: Option<u64>,
}

/// Toast notification component
/// Temporary notification that appears at top/bottom of screen
pub fn toast_notification(
    data: ToastData,
    on_close: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    h_stack((
        // Icon
        label({
            let icon = data.toast_type.icon().to_string();
            move || icon.clone()
        })
        .style(|s| s.font_size(18.0).margin_right(12.0)),
        
        // Content
        v_stack((
            // Title
            label({
                let title = data.title.clone();
                move || title.clone()
            })
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .font_size(13.0)
                    .font_bold()
            }),
            
            // Message (optional)
            if let Some(message) = data.message {
                label(move || message.clone())
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.dim"))
                            .font_size(12.0)
                            .margin_top(4.0)
                    })
            } else {
                label(|| String::new())
                    .style(|s| s.display(floem::style::Display::None))
            },
        ))
        .style(|s| s.flex_grow(1.0)),
        
        // Close button
        container(
            label(|| "✕".to_string())
        )
        .on_click_stop(move |_| {
            on_close();
        })
        .style(move |s| {
            let cfg = config();
            s.padding(4.0)
                .padding_horiz(8.0)
                .border_radius(3.0)
                .color(cfg.color("editor.foreground"))
                .font_size(14.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| s.background(cfg.color("list.hoverBackground")))
                .margin_left(12.0)
        }),
    ))
    .style(move |s| {
        let cfg = config();
        let bg_color = match data.toast_type {
            ToastType::Success => cfg.color("testing.iconPassed"),
            ToastType::Error => cfg.color("list.errorForeground"),
            ToastType::Warning => cfg.color("list.warningForeground"),
            ToastType::Info => cfg.color("lapce.button.primary.background"),
        };
        
        s.padding(12.0)
            .padding_horiz(16.0)
            .border_radius(6.0)
            .background(bg_color)
            .border(1.0)
            .border_color(cfg.color("lapce.border"))
            .min_width(300.0)
            .max_width(500.0)
            .items_center()
    })
}

/// Success toast
pub fn success_toast(
    title: String,
    message: Option<String>,
    on_close: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    toast_notification(
        ToastData {
            toast_type: ToastType::Success,
            title,
            message,
            duration_ms: Some(3000),
        },
        on_close,
        config,
    )
}

/// Error toast
pub fn error_toast(
    title: String,
    message: Option<String>,
    on_close: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    toast_notification(
        ToastData {
            toast_type: ToastType::Error,
            title,
            message,
            duration_ms: None, // Errors stay until dismissed
        },
        on_close,
        config,
    )
}

/// Warning toast
pub fn warning_toast(
    title: String,
    message: Option<String>,
    on_close: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    toast_notification(
        ToastData {
            toast_type: ToastType::Warning,
            title,
            message,
            duration_ms: Some(5000),
        },
        on_close,
        config,
    )
}

/// Info toast
pub fn info_toast(
    title: String,
    message: Option<String>,
    on_close: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    toast_notification(
        ToastData {
            toast_type: ToastType::Info,
            title,
            message,
            duration_ms: Some(4000),
        },
        on_close,
        config,
    )
}

// Note: Toast container removed - dynamic view lists not supported in current Floem API
// Toasts will be managed individually via state management when IPC bridge is integrated

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastPosition {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
    TopCenter,
    BottomCenter,
}

/// Quick action toast
/// Toast with action button
pub fn action_toast(
    title: String,
    action_label: String,
    on_action: impl Fn() + 'static + Copy,
    on_close: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    h_stack((
        label(|| "ℹ️".to_string())
            .style(|s| s.font_size(18.0).margin_right(12.0)),
        
        label({
            let t = title.clone();
            move || t.clone()
        })
        .style(move |s| {
            let cfg = config();
            s.color(cfg.color("editor.foreground"))
                .font_size(13.0)
                .flex_grow(1.0)
        }),
        
        container(
            label({
                let al = action_label.clone();
                move || al.clone()
            })
        )
        .on_click_stop(move |_| {
            on_action();
        })
        .style(move |s| {
            let cfg = config();
            s.padding(6.0)
                .padding_horiz(12.0)
                .border_radius(4.0)
                .background(cfg.color("panel.current.background"))
                .color(cfg.color("editor.foreground"))
                .font_size(11.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .margin_right(12.0)
        }),
        
        container(
            label(|| "✕".to_string())
        )
        .on_click_stop(move |_| {
            on_close();
        })
        .style(move |s| {
            let cfg = config();
            s.padding(4.0)
                .padding_horiz(8.0)
                .border_radius(3.0)
                .color(cfg.color("editor.foreground"))
                .font_size(14.0)
                .cursor(floem::style::CursorStyle::Pointer)
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.padding(12.0)
            .padding_horiz(16.0)
            .border_radius(6.0)
            .background(cfg.color("lapce.button.primary.background"))
            .border(1.0)
            .border_color(cfg.color("lapce.border"))
            .min_width(300.0)
            .max_width(500.0)
            .items_center()
    })
}
