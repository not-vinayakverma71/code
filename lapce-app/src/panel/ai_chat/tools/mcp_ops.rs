// MCP (Model Context Protocol) Tool Renderers
// Ported from McpExecution.tsx and MCP-related ChatRow cases

use std::sync::Arc;

use floem::{
    reactive::{SignalGet, SignalUpdate},
    views::{Decorators, container, h_stack, label, v_stack},
    View,
};

use crate::{
    config::LapceConfig,
    panel::ai_chat::shared::code_accordion::{CodeAccordionProps, code_accordion},
};

/// MCP Tool Execution renderer
pub struct McpToolExecutionProps {
    pub server_name: String,
    pub tool_name: String,
    pub arguments: Option<String>,
    pub result: Option<String>,
    pub is_executing: bool,
}

pub fn mcp_tool_execution(
    props: McpToolExecutionProps,
    is_expanded: floem::reactive::RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let server_name = props.server_name.clone();
    let tool_name = props.tool_name.clone();
    let arguments = props.arguments.clone();
    let result = props.result.clone();
    
    v_stack((
        // Header
        container(
            h_stack((
                label(|| "🔧".to_string())
                    .style(|s| s.margin_right(8.0)),
                label(move || {
                    if props.is_executing {
                        format!("executing {} on {}", tool_name, server_name)
                    } else {
                        format!("wants to use tool {} on {}", tool_name, server_name)
                    }
                })
                .style(|s| s),
            ))
        )
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .color(cfg.color("editor.foreground"))
        }),
        
        // Arguments display
        if let Some(args) = arguments {
            container(
                code_accordion(
                    CodeAccordionProps {
                        path: Some("Arguments".to_string()),
                        code: args.clone(),
                        language: Some("json".to_string()),
                        is_expanded,
                        on_toggle: Box::new(move || {
                            is_expanded.update(|v| *v = !*v);
                        }),
                        on_jump_to_file: None,
                    },
                    config,
                )
            )
            .style(|s| s.padding_left(24.0))
        } else {
            container(label(|| "")).style(|s| s)
        },
        
        // Result display
        if let Some(res) = result {
            container(
                code_accordion(
                    CodeAccordionProps {
                        path: Some("Result".to_string()),
                        code: res.clone(),
                        language: Some("json".to_string()),
                        is_expanded,
                        on_toggle: Box::new(move || {
                            is_expanded.update(|v| *v = !*v);
                        }),
                        on_jump_to_file: None,
                    },
                    config,
                )
            )
            .style(|s| s.padding_left(24.0))
        } else {
            container(label(|| "")).style(|s| s)
        },
    ))
    .style(|s| s.width_full())
}

/// MCP Resource Access renderer
pub struct McpResourceAccessProps {
    pub server_name: String,
    pub resource_uri: String,
    pub resource_name: Option<String>,
    pub mime_type: Option<String>,
}

pub fn mcp_resource_access(
    props: McpResourceAccessProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let server_name = props.server_name.clone();
    let resource_uri = props.resource_uri.clone();
    let resource_name = props.resource_name.clone();
    
    container(
        v_stack((
            h_stack((
                label(|| "📦".to_string())
                    .style(|s| s.margin_right(8.0)),
                label(move || format!("wants to access resource on {}", server_name.clone()))
                    .style(|s| s),
            )),
            label(move || {
                if let Some(name) = resource_name.clone() {
                    name
                } else {
                    resource_uri.clone()
                }
            })
            .style(move |s| {
                let cfg = config();
                s.margin_top(4.0)
                    .margin_left(28.0)
                    .color(cfg.color("editor.dim"))
            }),
        ))
    )
    .style(move |s| {
        let cfg = config();
        s.padding(8.0)
            .color(cfg.color("editor.foreground"))
            .width_full()
    })
}
