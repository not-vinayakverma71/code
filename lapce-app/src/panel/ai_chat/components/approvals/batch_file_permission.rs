// Batch File Permission - Approval for multiple file operations
// Shows list of files to read/write with workspace boundary warnings

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOperation {
    Read,
    Write,
}

impl FileOperation {
    pub fn label(&self) -> &'static str {
        match self {
            FileOperation::Read => "Read",
            FileOperation::Write => "Write",
        }
    }
    
    pub fn icon(&self) -> &'static str {
        match self {
            FileOperation::Read => "📖",
            FileOperation::Write => "✏️",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FilePermissionItem {
    pub path: String,
    pub operation: FileOperation,
    pub outside_workspace: bool,
    pub is_readonly: bool,
    pub is_protected: bool, // system files, .git, etc.
}

#[derive(Debug, Clone)]
pub struct BatchFilePermissionData {
    pub files: Vec<FilePermissionItem>,
}

/// Batch file permission approval
/// Shows list of files with individual or bulk approve/reject
pub fn batch_file_permission(
    data: BatchFilePermissionData,
    on_approve_all: impl Fn() + 'static + Copy,
    on_reject_all: impl Fn() + 'static + Copy,
    on_approve_file: impl Fn(usize) + 'static + Copy,
    on_reject_file: impl Fn(usize) + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let selected_files = create_rw_signal(vec![true; data.files.len()]);
    
    // Determine risk level
    let has_outside_workspace = data.files.iter().any(|f| f.outside_workspace);
    let has_protected = data.files.iter().any(|f| f.is_protected);
    let has_write = data.files.iter().any(|f| f.operation == FileOperation::Write);
    
    let risk_level = if has_protected {
        RiskLevel::High
    } else if has_outside_workspace && has_write {
        RiskLevel::Medium
    } else if has_write {
        RiskLevel::Low
    } else {
        RiskLevel::Low
    };
    
    v_stack((
        // Main approval dialog
        approval_request(
            ApprovalRequestProps {
                title: "File Operations Approval Required".to_string(),
                description: format!(
                    "The AI wants to perform operations on {} file(s):",
                    data.files.len()
                ),
                risk_level,
                show_auto_approve: true,
                timeout_secs: None,
            },
            move |always| {
                if always {
                    on_approve_all();
                } else {
                    // Approve only selected files
                    for (i, &selected) in selected_files.get().iter().enumerate() {
                        if selected {
                            on_approve_file(i);
                        }
                    }
                }
            },
            on_reject_all,
            config,
        ),
        
        // Files list
        container(
            v_stack((
                // List header
                h_stack((
                    label(|| "Select files to approve:".to_string())
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
                        selected_files.update(|files| {
                            for f in files.iter_mut() {
                                *f = true;
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
                        selected_files.update(|files| {
                            for f in files.iter_mut() {
                                *f = false;
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
                
                // Scrollable file list - simplified text list
                scroll(
                    label({
                        let files = data.files.clone();
                        move || files.iter().map(|file| {
                            let mut tags = Vec::new();
                            if file.outside_workspace { tags.push("OUTSIDE_WS"); }
                            if file.is_readonly && file.operation == FileOperation::Write { tags.push("READONLY"); }
                            if file.is_protected { tags.push("PROTECTED"); }
                            
                            let tags_str = if tags.is_empty() {
                                String::new()
                            } else {
                                format!(" [{}]", tags.join(", "))
                            };
                            
                            format!("{} {} {}{}", 
                                file.operation.icon(),
                                file.operation.label(),
                                file.path,
                                tags_str
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
                
                label(|| "💡 Full interactive file selection coming soon".to_string())
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
// SIMPLIFIED VERSION - Full interactive checkboxes will be added in future iteration
// The complex v_stack with mapped containers was causing ViewTuple issues
// For now, showing simple text list with operation icons and warning tags
