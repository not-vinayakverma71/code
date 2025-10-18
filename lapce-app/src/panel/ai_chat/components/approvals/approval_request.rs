// Approval Request - Generic approval dialog
// Shows action, risk level, approve/reject buttons, auto-approve option

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn label(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Low Risk",
            RiskLevel::Medium => "Medium Risk",
            RiskLevel::High => "High Risk",
            RiskLevel::Critical => "Critical Risk",
        }
    }
    
    pub fn color(&self, config: &Arc<LapceConfig>) -> floem::peniko::Color {
        match self {
            RiskLevel::Low => config.color("testing.iconPassed"),
            RiskLevel::Medium => config.color("list.warningForeground"),
            RiskLevel::High => config.color("list.errorForeground"),
            RiskLevel::Critical => config.color("errorForeground"),
        }
    }
    
    pub fn icon(&self) -> &'static str {
        match self {
            RiskLevel::Low => "ℹ️",
            RiskLevel::Medium => "⚠️",
            RiskLevel::High => "⚠️",
            RiskLevel::Critical => "🚨",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApprovalRequestProps {
    pub title: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub show_auto_approve: bool,
    pub timeout_secs: Option<u64>,
}

/// Generic approval dialog
/// Used for various agent actions requiring user consent
pub fn approval_request(
    props: ApprovalRequestProps,
    on_approve: impl Fn(bool) + 'static + Copy, // bool = always_allow
    on_reject: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let ApprovalRequestProps { 
        title, 
        description, 
        risk_level,
        show_auto_approve,
        timeout_secs,
    } = props;
    
    let always_allow = create_rw_signal(false);
    let remaining_secs = create_rw_signal(timeout_secs);
    
    container(
        v_stack((
            // Header with icon and risk level
            h_stack((
                label(move || risk_level.icon().to_string())
                    .style(|s| s.font_size(24.0).margin_right(12.0)),
                
                v_stack((
                    label(move || title.clone())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_size(14.0)
                                .font_bold()
                        }),
                    
                    container(
                        label(move || risk_level.label().to_string())
                    )
                    .style(move |s| {
                        let cfg = config();
                        s.padding(3.0)
                            .padding_horiz(8.0)
                            .border_radius(3.0)
                            .background(risk_level.color(&cfg))
                            .color(cfg.color("editor.background"))
                            .font_size(10.0)
                            .font_bold()
                            .margin_top(4.0)
                    }),
                )),
            ))
            .style(|s| s.items_start().margin_bottom(12.0)),
            
            // Description
            label(move || description.clone())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(13.0)
                        .line_height(1.5)
                        .margin_bottom(16.0)
                }),
            
            // Auto-approve checkbox
            container(
                h_stack((
                    container(
                        label(move || if always_allow.get() { "☑" } else { "☐" })
                    )
                    .on_click_stop(move |_| {
                        always_allow.update(|v| *v = !*v);
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.width(16.0)
                            .height(16.0)
                            .display(floem::style::Display::Flex)
                            .items_center()
                            .justify_center()
                            .border(1.0)
                            .border_radius(3.0)
                            .border_color(cfg.color("input.border"))
                            .cursor(floem::style::CursorStyle::Pointer)
                            .margin_right(8.0)
                    }),
                    
                    label(|| "Always allow this action".to_string())
                        .on_click_stop(move |_| {
                            always_allow.update(|v| *v = !*v);
                        })
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_size(12.0)
                                .cursor(floem::style::CursorStyle::Pointer)
                        }),
                ))
            )
            .style(move |s| {
                let base = s.margin_bottom(16.0);
                if show_auto_approve {
                    base
                } else {
                    base.display(floem::style::Display::None)
                }
            }),
            
            // Timeout countdown
            container(
                label(move || {
                    if let Some(secs) = remaining_secs.get() {
                        format!("Auto-rejecting in {} seconds...", secs)
                    } else {
                        String::new()
                    }
                })
            )
            .style(move |s| {
                let cfg = config();
                let base = s
                    .color(cfg.color("list.warningForeground"))
                    .font_size(11.0)
                    .margin_bottom(12.0);
                
                if timeout_secs.is_some() && remaining_secs.get().is_some() {
                    base
                } else {
                    base.display(floem::style::Display::None)
                }
            }),
            
            // Action buttons
            h_stack((
                container(
                    label(|| "Approve".to_string())
                )
                .on_click_stop(move |_| {
                    on_approve(always_allow.get());
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(10.0)
                        .padding_horiz(24.0)
                        .border_radius(6.0)
                        .background(cfg.color("lapce.button.primary.background"))
                        .color(cfg.color("lapce.button.primary.foreground"))
                        .font_size(13.0)
                        .font_bold()
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(cfg.color("panel.hovered.background")))
                        .margin_right(12.0)
                }),
                
                container(
                    label(|| "Reject".to_string())
                )
                .on_click_stop(move |_| {
                    on_reject();
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(10.0)
                        .padding_horiz(24.0)
                        .border_radius(6.0)
                        .background(cfg.color("panel.current.background"))
                        .color(cfg.color("editor.foreground"))
                        .font_size(13.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(cfg.color("panel.hovered.background")))
                }),
            ))
            .style(|s| s.justify_end()),
        ))
    )
    .style(move |s| {
        let cfg = config();
        s.padding(20.0)
            .border(2.0)
            .border_radius(8.0)
            .border_color(risk_level.color(&cfg))
            .background(cfg.color("editor.background"))
            .box_shadow_blur(8.0)
            .margin_bottom(16.0)
    })
}
