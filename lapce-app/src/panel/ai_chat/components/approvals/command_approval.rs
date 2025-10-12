// Command Approval - Command execution approval dialog
// Shows command, safety warnings, pattern allow/deny options

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::{
    config::LapceConfig,
    panel::ai_chat::components::approvals::{approval_request, ApprovalRequestProps, RiskLevel},
};

#[derive(Debug, Clone)]
pub struct CommandSafetyWarning {
    pub warning_type: String, // "destructive", "network", "sudo", "env_modification"
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CommandApprovalData {
    pub command: String,
    pub working_dir: String,
    pub warnings: Vec<CommandSafetyWarning>,
    pub suggested_alternative: Option<String>,
    pub extractable_patterns: Vec<String>,
}

/// Command-specific approval dialog
/// Includes safety warnings and pattern management
pub fn command_approval(
    data: CommandApprovalData,
    on_approve: impl Fn(bool) + 'static + Copy, // bool = always_allow
    on_reject: impl Fn() + 'static + Copy,
    on_allow_pattern: impl Fn(String) + 'static + Copy,
    on_deny_pattern: impl Fn(String) + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let show_patterns = create_rw_signal(false);
    
    // Determine risk level based on warnings
    let risk_level = if data.warnings.iter().any(|w| w.warning_type == "destructive" || w.warning_type == "sudo") {
        RiskLevel::High
    } else if !data.warnings.is_empty() {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };
    
    v_stack((
        // Main approval dialog
        approval_request(
            ApprovalRequestProps {
                title: "Command Execution Approval Required".to_string(),
                description: format!(
                    "The AI wants to execute the following command in {}:",
                    data.working_dir
                ),
                risk_level,
                show_auto_approve: true,
                timeout_secs: None,
            },
            on_approve,
            on_reject,
            config,
        ),
        
        // Command preview
        container(
            v_stack((
                label(|| "Command:".to_string())
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.dim"))
                            .font_size(11.0)
                            .margin_bottom(6.0)
                    }),
                
                container(
                    label({
                        let cmd = data.command.clone();
                        move || cmd.clone()
                    })
                )
                .style(move |s| {
                    let cfg = config();
                    s.padding(12.0)
                        .border(1.0)
                        .border_radius(4.0)
                        .border_color(cfg.color("terminal.border"))
                        .background(cfg.color("terminal.background"))
                        .color(cfg.color("terminal.ansiBrightWhite"))
                        .font_family("monospace".to_string())
                        .font_size(12.0)
                        .width_full()
                }),
            ))
        )
        .style(|s| s.margin_bottom(16.0)),
        
        // Safety warnings
        if !data.warnings.is_empty() {
            container(
                v_stack((
                    label(|| "⚠️ Safety Warnings:".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("list.errorForeground"))
                                .font_size(12.0)
                                .font_bold()
                                .margin_bottom(8.0)
                        }),
                    
                    label({
                        let warnings = data.warnings.clone();
                        move || warnings.iter()
                            .map(|w| format!("• {}", w.message))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.foreground"))
                            .font_size(12.0)
                            .line_height(1.6)
                    }),
                ))
            )
            .style(move |s| {
                let cfg = config();
                s.padding(12.0)
                    .border(1.0)
                    .border_radius(4.0)
                    .border_color(cfg.color("list.errorForeground"))
                    .background(cfg.color("inputValidation.errorBackground"))
                    .margin_bottom(16.0)
            })
        } else {
            container(floem::views::empty())
                .style(|s| s.display(floem::style::Display::None))
        },
        
        // Suggested alternative
        if let Some(alt) = data.suggested_alternative {
            container(
                v_stack((
                    label(|| "💡 Safer Alternative:".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("testing.iconPassed"))
                                .font_size(12.0)
                                .font_bold()
                                .margin_bottom(8.0)
                        }),
                    
                    container(
                        label(move || alt.clone())
                    )
                    .style(move |s| {
                        let cfg = config();
                        s.padding(8.0)
                            .border(1.0)
                            .border_radius(4.0)
                            .border_color(cfg.color("input.border"))
                            .background(cfg.color("input.background"))
                            .font_family("monospace".to_string())
                            .font_size(11.0)
                    }),
                ))
            )
            .style(|s| s.margin_bottom(16.0))
        } else {
            container(floem::views::empty())
                .style(|s| s.display(floem::style::Display::None))
        },
        
        // Pattern management section
        if !data.extractable_patterns.is_empty() {
            container(
                v_stack((
                    // Toggle header
                    h_stack((
                        label(move || if show_patterns.get() { "▼" } else { "▶" })
                            .style(|s| s.margin_right(8.0)),
                        label(|| "Command Pattern Rules".to_string()),
                    ))
                    .on_click_stop(move |_| {
                        show_patterns.update(|v| *v = !*v);
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.foreground"))
                            .font_size(12.0)
                            .font_bold()
                            .cursor(floem::style::CursorStyle::Pointer)
                            .margin_bottom(8.0)
                    }),
                    
                    // Patterns list (visible when expanded) - simplified
                    container(
                        v_stack((
                            label({
                                let patterns = data.extractable_patterns.clone();
                                move || patterns.iter()
                                    .map(|p| format!("  • {}", p))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            })
                            .style(move |s| {
                                let cfg = config();
                                s.color(cfg.color("editor.foreground"))
                                    .font_family("monospace".to_string())
                                    .font_size(11.0)
                                    .line_height(1.6)
                            }),
                            
                            label(|| "💡 Configure allowed/denied patterns in Settings → AI".to_string())
                                .style(move |s| {
                                    let cfg = config();
                                    s.color(cfg.color("editor.dim"))
                                        .font_size(10.0)
                                        .font_style(floem::text::Style::Italic)
                                        .margin_top(8.0)
                                }),
                        ))
                    )
                    .style(move |s| {
                        if show_patterns.get() {
                            s
                        } else {
                            s.display(floem::style::Display::None)
                        }
                    }),
                ))
            )
        } else {
            container(floem::views::empty())
                .style(|s| s.display(floem::style::Display::None))
        },
    ))
}
