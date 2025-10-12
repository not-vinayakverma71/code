// Write File Display - Shows file write operations
// File path, new/modified badge, diff preview

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, SignalGet},
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::{
    config::LapceConfig,
    panel::ai_chat::components::{
        messages::Status,
        tools::{tool_use_block, tool_metadata_row, ToolUseBlockProps},
    },
};

#[derive(Debug, Clone)]
pub struct WriteFileData {
    pub path: String,
    pub is_new_file: bool,
    pub backup_created: bool,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub encoding: String,
    pub status: Status,
}

pub fn write_file_display(
    data: WriteFileData,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let is_expanded = create_rw_signal(true);
    
    let content = v_stack((
        // Metadata
        tool_metadata_row(
            || "Path:".to_string(),
            {
                let path = data.path.clone();
                move || path.clone()
            },
            config
        ),
        
        tool_metadata_row(
            || "Type:".to_string(),
            {
                let is_new = data.is_new_file;
                move || if is_new { "New file".to_string() } else { "Modified".to_string() }
            },
            config
        ),
        
        // Changes row (only for modified files)
        container(
            tool_metadata_row(
                || "Changes:".to_string(),
                {
                    let added = data.lines_added;
                    let removed = data.lines_removed;
                    move || format!("+{} -{} lines", added, removed)
                },
                config
            )
        )
        .style(move |s| {
            if data.is_new_file {
                s.display(floem::style::Display::None)
            } else {
                s
            }
        }),
        
        // Backup row (only if backup created)
        container(
            tool_metadata_row(
                || "Backup:".to_string(),
                || "Created".to_string(),
                config
            )
        )
        .style(move |s| {
            if data.backup_created {
                s
            } else {
                s.display(floem::style::Display::None)
            }
        }),
        
        // New file badge
        container(
            label(|| "NEW FILE".to_string())
        )
        .style(move |s| {
            let cfg = config();
            let base = s
                .padding(4.0)
                .padding_horiz(8.0)
                .border_radius(3.0)
                .background(cfg.color("testing.iconPassed"))
                .color(cfg.color("editor.background"))
                .font_size(10.0)
                .font_bold()
                .margin_top(8.0);
            
            if data.is_new_file {
                base
            } else {
                base.display(floem::style::Display::None)
            }
        }),
        
        // Action buttons
        h_stack((
            container(
                label(|| "View Diff".to_string())
            )
            .on_click_stop({
                let path = data.path.clone();
                move |_| {
                    println!("[WriteFile] View diff: {}", path);
                    // TODO: Open diff view when IPC ready
                }
            })
            .style(move |s| {
                let cfg = config();
                s.padding(6.0)
                    .padding_horiz(12.0)
                    .border_radius(4.0)
                    .background(cfg.color("lapce.button.primary.background"))
                    .color(cfg.color("lapce.button.primary.foreground"))
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(cfg.color("panel.hovered.background")))
                    .margin_top(8.0)
                    .margin_right(8.0)
            }),
            
            container(
                label(|| "Open File".to_string())
            )
            .on_click_stop({
                let path = data.path.clone();
                move |_| {
                    println!("[WriteFile] Open file: {}", path);
                    // TODO: Wire to workspace.open_file when IPC ready
                }
            })
            .style(move |s| {
                let cfg = config();
                s.padding(6.0)
                    .padding_horiz(12.0)
                    .border_radius(4.0)
                    .background(cfg.color("panel.current.background"))
                    .color(cfg.color("editor.foreground"))
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(cfg.color("panel.hovered.background")))
                    .margin_top(8.0)
            }),
        )),
    ));
    
    tool_use_block(
        ToolUseBlockProps {
            tool_name: "Write File".to_string(),
            icon: "📝".to_string(),
            status: data.status,
            is_expanded,
        },
        content,
        config
    )
}
