// Plan Breakdown - Task planning and breakdown display
// Shows AI's plan with steps and progress tracking

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, scroll, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

impl StepStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            StepStatus::Pending => "⏸",
            StepStatus::InProgress => "⏳",
            StepStatus::Completed => "✅",
            StepStatus::Failed => "❌",
            StepStatus::Skipped => "⏭",
        }
    }
    
    pub fn label(&self) -> &'static str {
        match self {
            StepStatus::Pending => "Pending",
            StepStatus::InProgress => "In Progress",
            StepStatus::Completed => "Completed",
            StepStatus::Failed => "Failed",
            StepStatus::Skipped => "Skipped",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanStep {
    pub id: usize,
    pub title: String,
    pub description: String,
    pub status: StepStatus,
    pub substeps: Vec<String>,
    pub estimated_duration: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlanBreakdownData {
    pub title: String,
    pub description: String,
    pub steps: Vec<PlanStep>,
    pub current_step: Option<usize>,
}

impl PlanBreakdownData {
    pub fn progress_percentage(&self) -> f64 {
        if self.steps.is_empty() {
            return 0.0;
        }
        
        let completed = self.steps.iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count();
        
        (completed as f64 / self.steps.len() as f64) * 100.0
    }
    
    pub fn completed_count(&self) -> usize {
        self.steps.iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count()
    }
}

/// Plan breakdown component
/// Displays task plan with step-by-step progress
pub fn plan_breakdown(
    data: PlanBreakdownData,
    on_retry_step: impl Fn(usize) + 'static + Copy,
    on_skip_step: impl Fn(usize) + 'static + Copy,
    on_cancel_plan: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let is_expanded = create_rw_signal(true);
    
    // Clone values before moving into closures
    let title = data.title.clone();
    let description = data.description.clone();
    let steps = data.steps.clone();
    let completed = data.completed_count();
    let total = data.steps.len();
    let percentage = data.progress_percentage();
    
    v_stack((
        // Header
        h_stack((
            label(move || if is_expanded.get() { "▼" } else { "▶" })
                .on_click_stop(move |_| {
                    is_expanded.update(|v| *v = !*v);
                })
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(12.0)
                        .margin_right(8.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                }),
            
            label({
                let t = title.clone();
                move || t.clone()
            })
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .font_size(13.0)
                    .font_bold()
                    .flex_grow(1.0)
            }),
            
            // Progress
            label(move || format!("{}/{} steps ({:.0}%)", completed, total, percentage))
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.dim"))
                    .font_size(10.0)
                    .margin_right(12.0)
            }),
            
            // Cancel button
            container(
                label(|| "✕".to_string())
            )
            .on_click_stop(move |_| {
                on_cancel_plan();
            })
            .style(move |s| {
                let cfg = config();
                s.padding(4.0)
                    .padding_horiz(8.0)
                    .border_radius(3.0)
                    .background(cfg.color("list.errorForeground"))
                    .color(cfg.color("editor.background"))
                    .font_size(11.0)
                    .font_bold()
                    .cursor(floem::style::CursorStyle::Pointer)
            }),
        ))
        .style(move |s| {
            let cfg = config();
            s.padding(12.0)
                .border_bottom(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("panel.background"))
                .items_center()
        }),
        
        // Content
        container(
            v_stack((
                // Description
                label({
                    let desc = description.clone();
                    move || desc.clone()
                })
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(12.0)
                        .line_height(1.5)
                        .margin_bottom(16.0)
                }),
                
                // Progress bar
                container(
                    container(floem::views::empty())
                        .style(move |s| {
                            let cfg = config();
                            s.width_pct(percentage)
                                .height(4.0)
                                .background(cfg.color("testing.iconPassed"))
                                .border_radius(2.0)
                        })
                )
                .style(move |s| {
                    let cfg = config();
                    s.width_full()
                        .height(4.0)
                        .background(cfg.color("input.border"))
                        .border_radius(2.0)
                        .margin_bottom(16.0)
                }),
                
                // Steps list
                scroll(
                    label({
                        let step_list = steps.clone();
                        move || step_list.iter().map(|step| {
                            let substeps_str = if !step.substeps.is_empty() {
                                format!("\n{}", 
                                    step.substeps.iter()
                                        .map(|s| format!("    • {}", s))
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                )
                            } else {
                                String::new()
                            };
                            
                            let duration_str = if let Some(dur) = &step.estimated_duration {
                                format!(" (~{})", dur)
                            } else {
                                String::new()
                            };
                            
                            format!("{}. {} {} {}{}\n   {}{}",
                                step.id,
                                step.status.icon(),
                                step.title,
                                step.status.label(),
                                duration_str,
                                step.description,
                                substeps_str
                            )
                        }).collect::<Vec<_>>().join("\n\n")
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.foreground"))
                            .font_size(11.0)
                            .line_height(1.6)
                    })
                )
                .style(|s| s.max_height(400.0).width_full()),
            ))
            .style(|s| s.padding(12.0))
        )
        .style(move |s| {
            if is_expanded.get() {
                s
            } else {
                s.display(floem::style::Display::None)
            }
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .border(1.0)
            .border_radius(6.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background"))
            .margin_bottom(16.0)
    })
}
