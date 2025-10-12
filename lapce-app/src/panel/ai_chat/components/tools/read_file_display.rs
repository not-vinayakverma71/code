// Read File Display - Shows file read operations
// File path, line range, preview, metadata

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, RwSignal, SignalGet},
    views::{container, dyn_stack, h_stack, label, v_stack, Decorators},
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
pub struct ReadFileData {
    pub path: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub total_lines: usize,
    pub size_bytes: usize,
    pub encoding: String,
    pub preview_lines: Vec<String>,
    pub status: Status,
}

pub fn read_file_display(
    data: ReadFileData,
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
            || "Lines:".to_string(),
            {
                let line_start = data.line_start;
                let line_end = data.line_end;
                let total = data.total_lines;
                move || {
                    if let (Some(start), Some(end)) = (line_start, line_end) {
                        format!("{}-{} of {}", start, end, total)
                    } else {
                        format!("All {} lines", total)
                    }
                }
            },
            config
        ),
        
        tool_metadata_row(
            || "Size:".to_string(),
            {
                let size = data.size_bytes;
                move || format_bytes(size)
            },
            config
        ),
        
        tool_metadata_row(
            || "Encoding:".to_string(),
            {
                let encoding = data.encoding.clone();
                move || encoding.clone()
            },
            config
        ),
        
        // Preview (first few lines)
        if !data.preview_lines.is_empty() {
            container(
                v_stack((
                    label(|| "Preview:".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.dim"))
                                .font_size(11.0)
                                .margin_top(8.0)
                                .margin_bottom(4.0)
                        }),
                    
                    container(
                        label({
                            let lines = data.preview_lines.clone();
                            move || lines.iter().enumerate()
                                .map(|(i, line)| format!("{:4} {}", i + 1, line))
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_family("monospace".to_string())
                                .font_size(11.0)
                        })
                    )
                    .style(move |s| {
                        let cfg = config();
                        s.padding(8.0)
                            .border(1.0)
                            .border_radius(4.0)
                            .border_color(cfg.color("input.border"))
                            .background(cfg.color("input.background"))
                            .max_height(200.0)
                    }),
                ))
            )
        } else {
            container(floem::views::empty())
                .style(|s| s.display(floem::style::Display::None))
        },
        
        // Action buttons
        h_stack((
            container(
                label(|| "Open in Editor".to_string())
            )
            .on_click_stop({
                let path = data.path.clone();
                move |_| {
                    println!("[ReadFile] Open in editor: {}", path);
                    // TODO: Wire to workspace.open_file when IPC ready
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
            }),
        )),
    ));
    
    tool_use_block(
        ToolUseBlockProps {
            tool_name: "Read File".to_string(),
            icon: "📄".to_string(),
            status: data.status,
            is_expanded,
        },
        content,
        config
    )
}

fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    
    if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
