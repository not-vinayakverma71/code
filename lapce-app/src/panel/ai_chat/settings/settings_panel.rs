// Settings Panel
// Pre-IPC Phase 7: Settings UI (read-only for now)

use std::sync::Arc;

use floem::{
    reactive::SignalGet,
    views::{Decorators, container, label, scroll, v_stack},
    View,
};

use crate::config::LapceConfig;

/// Settings panel for AI chat configuration
/// TODO: Wire to actual settings when IPC ready
pub fn settings_panel(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        scroll(
            v_stack((
                // Header
                label(|| "AI Chat Settings".to_string())
                    .style(move |s| {
                        let cfg = config();
                        s.font_size(18.0)
                            .padding_bottom(16.0)
                            .color(cfg.color("editor.foreground"))
                    }),
                
                // Auto-approval section
                settings_section("Auto-Approval", config),
                settings_item("Auto-approve read-only operations", "false", config),
                settings_item("Auto-approve file writes", "false", config),
                settings_item("Auto-approve commands", "false", config),
                settings_item("Auto-approve MCP tools", "false", config),
                
                // Display section
                settings_section("Display", config),
                settings_item("Show tool details", "true", config),
                settings_item("Show timestamps", "false", config),
                settings_item("Syntax highlighting", "true", config),
                
                // Notifications section
                settings_section("Notifications", config),
                settings_item("Sound enabled", "true", config),
                settings_item("Desktop notifications", "true", config),
            ))
            .style(|s| s.gap(8.0).padding(16.0))
        )
    )
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .height_full()
            .background(cfg.color("panel.background"))
    })
}

fn settings_section(
    title: &'static str,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    label(move || title.to_string())
        .style(move |s| {
            let cfg = config();
            s.font_size(14.0)
                .padding_top(12.0)
                .padding_bottom(8.0)
                .color(cfg.color("editor.foreground"))
        })
}

fn settings_item(
    name: &'static str,
    value: &'static str,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        v_stack((
            label(move || name.to_string())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                }),
            label(move || format!("Value: {}", value))
                .style(move |s| {
                    let cfg = config();
                    s.font_size(11.0)
                        .color(cfg.color("editor.dim"))
                }),
        ))
    )
    .style(|s| s.padding(8.0))
}
