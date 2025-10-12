// Settings Panel
// Pre-IPC Phase 7: Settings UI bound to AIChatState

use std::sync::Arc;

use floem::{
    reactive::SignalGet,
    views::{Decorators, container, label, scroll, v_stack},
    View,
};

use crate::{
    ai_state::AIChatState,
    config::LapceConfig,
};

/// Settings panel for AI chat configuration
/// Displays current state values (read-only pre-IPC)
pub fn settings_panel(
    ai_state: Arc<AIChatState>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let state = ai_state.clone();
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
                {
                    let s = state.clone();
                    settings_item_dynamic("Auto-approval enabled", move || s.auto_approval_enabled.get().to_string(), config)
                },
                {
                    let s = state.clone();
                    settings_item_dynamic("Auto-approve read-only", move || s.always_allow_read_only.get().to_string(), config)
                },
                {
                    let s = state.clone();
                    settings_item_dynamic("Auto-approve writes", move || s.always_allow_write.get().to_string(), config)
                },
                {
                    let s = state.clone();
                    settings_item_dynamic("Auto-approve commands", move || s.always_allow_execute.get().to_string(), config)
                },
                {
                    let s = state.clone();
                    settings_item_dynamic("Auto-approve MCP", move || s.always_allow_mcp.get().to_string(), config)
                },
                {
                    let s = state.clone();
                    settings_item_dynamic("Auto-approve browser", move || s.always_allow_browser.get().to_string(), config)
                },
                
                // Display section  
                settings_section("Display", config),
                {
                    let s = state.clone();
                    settings_item_dynamic("Show timestamps", move || s.show_timestamps.get().to_string(), config)
                },
                {
                    let s = state.clone();
                    settings_item_dynamic("Show task timeline", move || s.show_task_timeline.get().to_string(), config)
                },
                {
                    let s = state.clone();
                    settings_item_dynamic("Reasoning collapsed", move || s.reasoning_block_collapsed.get().to_string(), config)
                },
                
                // Notifications section
                settings_section("Notifications", config),
                {
                    let s = state.clone();
                    settings_item_dynamic("Sound enabled", move || s.sound_enabled.get().to_string(), config)
                },
                {
                    let s = state.clone();
                    settings_item_dynamic("Sound volume", move || format!("{:.0}%", s.sound_volume.get() * 100.0), config)
                },
                {
                    let s = state.clone();
                    settings_item_dynamic("System notifications", move || s.system_notifications_enabled.get().to_string(), config)
                },
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

fn settings_item_dynamic(
    name: &'static str,
    value_fn: impl Fn() -> String + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        v_stack((
            label(move || name.to_string())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                }),
            label(move || format!("Value: {}", value_fn()))
                .style(move |s| {
                    let cfg = config();
                    s.font_size(11.0)
                        .color(cfg.color("editor.dim"))
                }),
        ))
    )
    .style(|s| s.padding(8.0))
}
