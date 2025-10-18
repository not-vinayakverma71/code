// Batch Diff Approval - Approval for multiple file changes
// Shows list of diffs with summaries, individual or bulk approve/reject

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, scroll, v_stack, Decorators},
    View,
};
use crate::{
    config::LapceConfig,
    panel::ai_chat::components::approvals::{approval_request, ApprovalRequestProps, RiskLevel},
};

#[derive(Debug, Clone)]
pub struct DiffItem {
    pub file_path: String,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub is_new_file: bool,
    pub is_deleted: bool,
    pub preview: String, // First few lines of diff
}

impl DiffItem {
    pub fn total_changes(&self) -> usize {
        self.lines_added + self.lines_removed
    }
}

#[derive(Debug, Clone)]
pub struct BatchDiffApprovalData {
    pub diffs: Vec<DiffItem>,
}

impl BatchDiffApprovalData {
    pub fn total_files(&self) -> usize {
        self.diffs.len()
    }
    
    pub fn total_lines_added(&self) -> usize {
        self.diffs.iter().map(|d| d.lines_added).sum()
    }
    
    pub fn total_lines_removed(&self) -> usize {
        self.diffs.iter().map(|d| d.lines_removed).sum()
    }
}

/// Batch diff approval
/// Shows list of file changes with individual or bulk approve/reject
pub fn batch_diff_approval(
    data: BatchDiffApprovalData,
    on_approve_all: impl Fn() + 'static + Copy,
    on_reject_all: impl Fn() + 'static + Copy,
    on_approve_file: impl Fn(usize) + 'static + Copy,
    on_reject_file: impl Fn(usize) + 'static + Copy,
    on_view_diff: impl Fn(usize) + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let selected_diffs = create_rw_signal(vec![true; data.diffs.len()]);
    
    // Determine risk level based on scope
    let total_changes = data.total_lines_added() + data.total_lines_removed();
    let risk_level = if total_changes > 500 {
        RiskLevel::High
    } else if total_changes > 100 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };
    
    // Extract values before moving data into closures
    let total_files = data.total_files();
    let total_added = data.total_lines_added();
    let total_removed = data.total_lines_removed();
    let diffs_clone = data.diffs.clone();
    
    v_stack((
        // Main approval dialog
        approval_request(
            ApprovalRequestProps {
                title: "Apply Changes Approval Required".to_string(),
                description: format!(
                    "The AI wants to modify {} file(s) with {} changes:",
                    total_files,
                    total_changes,
                ),
                risk_level,
                show_auto_approve: false, // Diffs are usually reviewed manually
                timeout_secs: None,
            },
            move |_always| {
                // Approve selected diffs
                for (i, &selected) in selected_diffs.get().iter().enumerate() {
                    if selected {
                        on_approve_file(i);
                    }
                }
            },
            on_reject_all,
            config,
        ),
        
        // Summary stats
        container(
            h_stack((
                v_stack((
                    label(move || total_files.to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_size(20.0)
                                .font_bold()
                        }),
                    label(|| "Files".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.dim"))
                                .font_size(10.0)
                        }),
                ))
                .style(|s| s.items_center().margin_right(24.0)),
                
                v_stack((
                    label(move || format!("+{}", total_added))
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("testing.iconPassed"))
                                .font_size(20.0)
                                .font_bold()
                        }),
                    label(|| "Added".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.dim"))
                                .font_size(10.0)
                        }),
                ))
                .style(|s| s.items_center().margin_right(24.0)),
                
                v_stack((
                    label(move || format!("-{}", total_removed))
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("list.errorForeground"))
                                .font_size(20.0)
                                .font_bold()
                        }),
                    label(|| "Removed".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.dim"))
                                .font_size(10.0)
                        }),
                ))
                .style(|s| s.items_center()),
            ))
        )
        .style(move |s| {
            let cfg = config();
            s.padding(16.0)
                .border(1.0)
                .border_radius(4.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("editor.background"))
                .margin_bottom(16.0)
                .justify_center()
        }),
        
        // Diffs list
        container(
            v_stack((
                // List header
                h_stack((
                    label(|| "Select changes to apply:".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_size(12.0)
                                .font_bold()
                                .flex_grow(1.0)
                        }),
                    
                    // Select All / Deselect All
                    container(
                        label(|| "Select All".to_string())
                    )
                    .on_click_stop(move |_| {
                        selected_diffs.update(|diffs| {
                            for d in diffs.iter_mut() {
                                *d = true;
                            }
                        });
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.padding(4.0)
                            .padding_horiz(8.0)
                            .border_radius(3.0)
                            .background(cfg.color("panel.current.background"))
                            .color(cfg.color("editor.foreground"))
                            .font_size(10.0)
                            .cursor(floem::style::CursorStyle::Pointer)
                            .margin_right(6.0)
                    }),
                    
                    container(
                        label(|| "Deselect All".to_string())
                    )
                    .on_click_stop(move |_| {
                        selected_diffs.update(|diffs| {
                            for d in diffs.iter_mut() {
                                *d = false;
                            }
                        });
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.padding(4.0)
                            .padding_horiz(8.0)
                            .border_radius(3.0)
                            .background(cfg.color("panel.current.background"))
                            .color(cfg.color("editor.foreground"))
                            .font_size(10.0)
                            .cursor(floem::style::CursorStyle::Pointer)
                    }),
                ))
                .style(|s| s.margin_bottom(12.0)),
                
                // Scrollable diff list - simplified
                scroll(
                    label({
                        let diffs = diffs_clone.clone();
                        move || diffs.iter().map(|diff| {
                            let mut status = Vec::new();
                            if diff.is_new_file { status.push("NEW"); }
                            if diff.is_deleted { status.push("DELETED"); }
                            
                            let status_str = if status.is_empty() {
                                String::new()
                            } else {
                                format!(" [{}]", status.join(", "))
                            };
                            
                            format!("📄 {} (+{} -{}){}",
                                diff.file_path,
                                diff.lines_added,
                                diff.lines_removed,
                                status_str
                            )
                        }).collect::<Vec<_>>().join("\n")
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.foreground"))
                            .font_family("monospace".to_string())
                            .font_size(11.0)
                            .line_height(1.6)
                    })
                )
                .style(|s| s.max_height(400.0)),
                
                label(|| "💡 Full interactive diff selection coming soon".to_string())
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
            let cfg = config();
            s.padding(12.0)
                .border(1.0)
                .border_radius(4.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("editor.background"))
        }),
    ))
}
// SIMPLIFIED VERSION - Full interactive diff approval UI coming in future iteration
