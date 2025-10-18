// Command/Terminal Tool Renderers
// Ported from CommandExecution.tsx

use std::sync::Arc;

use floem::{
    reactive::SignalUpdate,
    views::{Decorators, container, h_stack, label, v_stack},
    View,
};

use crate::{
    config::LapceConfig,
    panel::ai_chat::shared::code_accordion::{CodeAccordionProps, code_accordion},
};

/// CommandExecution tool renderer
pub struct CommandExecutionToolProps {
    pub command: String,
    pub output: Option<String>,
    pub is_executing: bool,
    pub is_ask: bool,
    pub exit_code: Option<i32>,
}

pub fn command_execution_tool(
    props: CommandExecutionToolProps,
    is_expanded: floem::reactive::RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let command = props.command.clone();
    let output = props.output.clone();
    
    v_stack((
        // Header
        container(
            h_stack((
                label(|| "💻".to_string())
                    .style(|s| s.margin_right(8.0)),
                label(move || {
                    if props.is_ask {
                        format!("wants to execute: {}", command)
                    } else if props.is_executing {
                        "executing command...".to_string()
                    } else if let Some(code) = props.exit_code {
                        if code == 0 {
                            "command completed".to_string()
                        } else {
                            format!("command failed (exit {})", code)
                        }
                    } else {
                        "command executed".to_string()
                    }
                })
                .style(|s| s),
            ))
        )
        .style(move |s| {
            let cfg = config();
            let color = if let Some(code) = props.exit_code {
                if code == 0 {
                    cfg.color("editor.foreground")
                } else {
                    cfg.color("editor.errorForeground")
                }
            } else {
                cfg.color("editor.foreground")
            };
            s.padding(8.0).color(color)
        }),
        
        // Output display
        if let Some(out) = output {
            container(
                code_accordion(
                    CodeAccordionProps {
                        path: None,
                        code: out.clone(),
                        language: Some("shell".to_string()),
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
