// Command Execution Display - Shows terminal command execution
// Command text, output, exit code, duration

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, SignalGet},
    views::{container, h_stack, label, v_stack, scroll, Decorators},
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
pub struct CommandExecutionData {
    pub command: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub working_dir: String,
    pub status: Status,
}

pub fn command_execution_display(
    data: CommandExecutionData,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let is_expanded = create_rw_signal(true);
    
    let content = v_stack((
        // Command text
        container(
            h_stack((
                label(|| "Command:".to_string())
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.dim"))
                            .font_size(11.0)
                            .margin_right(8.0)
                            .min_width(80.0)
                    }),
                
                label({
                    let cmd = data.command.clone();
                    move || cmd.clone()
                })
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_family("monospace".to_string())
                        .font_size(12.0)
                        .font_bold()
                }),
            ))
        )
        .style(|s| s.margin_bottom(8.0)),
        
        // Metadata
        tool_metadata_row(
            || "Working Dir:".to_string(),
            {
                let wd = data.working_dir.clone();
                move || wd.clone()
            },
            config
        ),
        
        // Exit code (only if available)
        container(
            tool_metadata_row(
                || "Exit Code:".to_string(),
                {
                    let exit_code = data.exit_code;
                    move || {
                        if let Some(code) = exit_code {
                            if code == 0 {
                                format!("{} (success)", code)
                            } else {
                                format!("{} (error)", code)
                            }
                        } else {
                            "N/A".to_string()
                        }
                    }
                },
                config
            )
        )
        .style(move |s| {
            if data.exit_code.is_some() {
                s
            } else {
                s.display(floem::style::Display::None)
            }
        }),
        
        tool_metadata_row(
            || "Duration:".to_string(),
            {
                let duration = data.duration_ms;
                move || format_duration(duration)
            },
            config
        ),
        
        // Output
        if !data.output.is_empty() {
            container(
                v_stack((
                    label(|| "Output:".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.dim"))
                                .font_size(11.0)
                                .margin_top(8.0)
                                .margin_bottom(4.0)
                        }),
                    
                    scroll(
                        container(
                            label({
                                let output = data.output.clone();
                                move || output.clone()
                            })
                            .style(move |s| {
                                let cfg = config();
                                let is_error = data.exit_code.map(|c| c != 0).unwrap_or(false);
                                s.font_family("monospace".to_string())
                                    .font_size(11.0)
                                    .line_height(1.4)
                                    .color(if is_error {
                                        cfg.color("errorForeground")
                                    } else {
                                        cfg.color("terminal.ansiGreen")
                                    })
                            })
                        )
                        .style(move |s| {
                            let cfg = config();
                            s.padding(8.0)
                                .border(1.0)
                                .border_radius(4.0)
                                .border_color(cfg.color("terminal.border"))
                                .background(cfg.color("terminal.background"))
                                .width_full()
                        })
                    )
                    .style(|s| s.max_height(300.0).width_full()),
                ))
            )
        } else {
            container(
                label(|| "(no output)".to_string())
            )
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.dim"))
                    .font_size(11.0)
                    .font_style(floem::text::Style::Italic)
                    .margin_top(8.0)
            })
        },
        
        // Action buttons
        h_stack((
            container(
                label(|| "Open in Terminal".to_string())
            )
            .on_click_stop({
                let cmd = data.command.clone();
                move |_| {
                    println!("[CommandExecution] Open in terminal: {}", cmd);
                    // TODO: Wire to terminal panel when IPC ready
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
            
            // Retry button (only for failed commands)
            container(
                label(|| "Retry".to_string())
            )
            .on_click_stop({
                let cmd = data.command.clone();
                move |_| {
                    println!("[CommandExecution] Retry: {}", cmd);
                    // TODO: Wire to retry mechanism when IPC ready
                }
            })
            .style(move |s| {
                let cfg = config();
                let base = s
                    .padding(6.0)
                    .padding_horiz(12.0)
                    .border_radius(4.0)
                    .background(cfg.color("panel.current.background"))
                    .color(cfg.color("editor.foreground"))
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(cfg.color("panel.hovered.background")))
                    .margin_top(8.0);
                
                if data.exit_code.map(|c| c != 0).unwrap_or(false) {
                    base
                } else {
                    base.display(floem::style::Display::None)
                }
            }),
        )),
    ));
    
    tool_use_block(
        ToolUseBlockProps {
            tool_name: "Execute Command".to_string(),
            icon: "💻".to_string(),
            status: data.status,
            is_expanded,
        },
        content,
        config
    )
}

fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{:.2}s", ms as f64 / 1000.0)
    } else {
        let secs = ms / 1000;
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{}m {}s", mins, secs)
    }
}
