// MCP Execution Display - Shows Model Context Protocol tool calls
// MCP server name, tool name, parameters, result

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
pub struct McpExecutionData {
    pub server_name: String,
    pub tool_name: String,
    pub parameters: Vec<(String, String)>, // key-value pairs
    pub result: String,
    pub duration_ms: u64,
    pub status: Status,
}

pub fn mcp_execution_display(
    data: McpExecutionData,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let is_expanded = create_rw_signal(true);
    
    let content = v_stack((
        // Server and tool
        tool_metadata_row(
            || "MCP Server:".to_string(),
            {
                let server = data.server_name.clone();
                move || server.clone()
            },
            config
        ),
        
        tool_metadata_row(
            || "Tool:".to_string(),
            {
                let tool = data.tool_name.clone();
                move || tool.clone()
            },
            config
        ),
        
        tool_metadata_row(
            || "Duration:".to_string(),
            {
                let duration = data.duration_ms;
                move || format_duration(duration)
            },
            config
        ),
        
        // Parameters
        if !data.parameters.is_empty() {
            container(
                v_stack((
                    label(|| "Parameters:".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.dim"))
                                .font_size(11.0)
                                .margin_top(8.0)
                                .margin_bottom(4.0)
                        }),
                    
                    label({
                        let params = data.parameters.clone();
                        move || params.iter()
                            .map(|(k, v)| format!("  {}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.foreground"))
                            .font_family("monospace".to_string())
                            .font_size(11.0)
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.padding(8.0)
                            .border(1.0)
                            .border_radius(4.0)
                            .border_color(cfg.color("input.border"))
                            .background(cfg.color("input.background"))
                    }),
                ))
            )
        } else {
            container(floem::views::empty())
                .style(|s| s.display(floem::style::Display::None))
        },
        
        // Result
        if !data.result.is_empty() {
            container(
                v_stack((
                    label(|| "Result:".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.dim"))
                                .font_size(11.0)
                                .margin_top(8.0)
                                .margin_bottom(4.0)
                        }),
                    
                    container(
                        label({
                            let result = data.result.clone();
                            move || result.clone()
                        })
                        .style(move |s| {
                            let cfg = config();
                            s.font_family("monospace".to_string())
                                .font_size(11.0)
                                .line_height(1.4)
                                .color(cfg.color("terminal.ansiGreen"))
                        })
                    )
                    .style(move |s| {
                        let cfg = config();
                        s.padding(8.0)
                            .border(1.0)
                            .border_radius(4.0)
                            .border_color(cfg.color("terminal.border"))
                            .background(cfg.color("terminal.background"))
                            .max_height(200.0)
                    }),
                ))
            )
        } else {
            container(
                label(|| "(no result)".to_string())
            )
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.dim"))
                    .font_size(11.0)
                    .font_style(floem::text::Style::Italic)
                    .margin_top(8.0)
            })
        },
    ));
    
    tool_use_block(
        ToolUseBlockProps {
            tool_name: format!("MCP: {}", data.tool_name),
            icon: "🔌".to_string(),
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
    } else {
        format!("{:.2}s", ms as f64 / 1000.0)
    }
}
