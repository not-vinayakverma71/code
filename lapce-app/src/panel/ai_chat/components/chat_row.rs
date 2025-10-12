// ChatRow - Individual message renderer with tool routing
// Ported from ChatRow.tsx (Phase 5-6)

use std::sync::Arc;

use floem::{
    reactive::RwSignal,
    views::{Decorators, container, h_stack, label, v_stack},
    View,
};

use crate::{
    ai_bridge::messages::ToolPayload,
    config::LapceConfig,
    panel::ai_chat::tools::{
        file_ops::*,
        diff_ops::*,
        task_ops::*,
    },
};

#[derive(Clone, Debug)]
pub enum MessageType {
    Say(SayType),
    Ask(AskType),
}

#[derive(Clone, Debug)]
pub enum SayType {
    Text,
    User,
    Tool,
    ApiReqStarted,
    CompletionResult,
}

#[derive(Clone, Debug)]
pub enum AskType {
    Tool,
    Followup,
    Command,
    McpServer,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub ts: u64,
    pub message_type: MessageType,
    pub text: String,
    pub partial: bool,
}

pub struct ChatRowProps {
    pub message: ChatMessage,
    pub is_last: bool,
    pub is_expanded: RwSignal<bool>,
}

/// Main chat row renderer
/// Routes to appropriate renderer based on message type
pub fn chat_row(
    props: ChatRowProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let message_type = props.message.message_type.clone();
    let text = props.message.text.clone();
    let is_expanded = props.is_expanded;
    
    container(
        match message_type {
            MessageType::Say(say_type) => {
                say_message_simple(say_type, text, is_expanded, config)
            }
            MessageType::Ask(ask_type) => {
                ask_message_simple(ask_type, text, is_expanded, config)
            }
        }
    )
    .style(|s| s.width_full())
}

/// Render "say" messages (assistant responses)
fn say_message_simple(
    say_type: SayType,
    text: String,
    is_expanded: RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> Box<dyn View> {
    match say_type {
        SayType::Tool => {
            // Parse tool JSON and route to renderer
            if let Ok(tool) = serde_json::from_str::<ToolPayload>(&text) {
                return render_tool_payload(tool, false, is_expanded, config);
            }
            // Fallback if parse fails
            Box::new(container(
                label(move || format!("Tool: {}", text.clone()))
            ).style(|s| s.padding(12.0)))
        },
        SayType::Text => Box::new({
            // Regular assistant text - EXACT Windsurf prose styling from ui.json
            let text = text.clone();
            container(
                label(move || text.clone())
                    .style(move |s| {
                        let cfg = config();
                        s.padding(16.0)
                            .color(cfg.color("editor.foreground"))
                            .font_size(14.0)   // Exact from ui.json
                            .line_height(1.6)  // 22.4px / 14px = 1.6
                            .width_full()
                    })
            )
            .style(move |s| {
                let cfg = config();
                s.width_full()
                    .background(cfg.color("panel.background"))
                    .border_radius(6.0)
                    .margin_bottom(12.0)
            })
        }),
        SayType::User => Box::new({
            // User message - Windsurf style (more prominent)
            let text = text.clone();
            container(
                label(move || text.clone())
                    .style(move |s| {
                        let cfg = config();
                        s.padding(16.0)
                            .color(cfg.color("editor.foreground"))
                            .font_size(14.0)
                    })
            )
            .style(move |s| {
                let cfg = config();
                s.width_full()
                    .background(cfg.color("panel.background"))
                    .border_radius(8.0)
                    .margin_bottom(16.0)
                    .border(1.0)
                    .border_color(cfg.color("lapce.button_primary"))
            })
        }),
        SayType::ApiReqStarted => Box::new({
            // API request indicator - thinking state
            container(
                h_stack((
                    label(|| "🧠".to_string())
                        .style(|s| s.margin_right(8.0).font_size(14.0)),
                    label(|| "Thinking...".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.font_size(13.0).color(cfg.color("editor.dim"))
                        }),
                ))
            )
            .style(move |s| {
                let cfg = config();
                s.padding(12.0)
                    .margin_bottom(8.0)
                    .border_radius(6.0)
                    .background(cfg.color("panel.background"))
            })
        }),
        SayType::CompletionResult => Box::new({
            // Task completion - success indicator
            container(
                h_stack((
                    label(|| "✅".to_string())
                        .style(|s| s.margin_right(8.0).font_size(14.0)),
                    label(|| "Complete".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.font_size(13.0).color(cfg.color("terminal.ansiGreen"))
                        }),
                ))
            )
            .style(move |s| {
                let cfg = config();
                s.padding(12.0)
                    .margin_bottom(8.0)
                    .border_radius(6.0)
                    .background(cfg.color("panel.background"))
                    .border(1.0)
                    .border_color(cfg.color("terminal.ansiGreen"))
            })
        }),
    }
}

/// Route tool payload to specific renderer
fn render_tool_payload(
    tool: ToolPayload,
    is_ask: bool,
    is_expanded: RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> Box<dyn View> {
    match tool {
        ToolPayload::ReadFile { path, content, is_outside_workspace, additional_file_count, .. } => {
            Box::new(read_file_tool(
                ReadFileToolProps {
                    path,
                    content: content.unwrap_or_default(),
                    is_ask,
                    is_outside_workspace,
                    additional_file_count,
                    reason: None,
                },
                is_expanded,
                config,
            ))
        }
        
        ToolPayload::ListFilesTopLevel { path, content, is_outside_workspace } => {
            Box::new(list_files_top_level_tool(
                ListFilesTopLevelToolProps {
                    path,
                    content,
                    is_ask,
                    is_outside_workspace,
                },
                is_expanded,
                config,
            ))
        }
        
        ToolPayload::ListFilesRecursive { path, content, is_outside_workspace } => {
            Box::new(list_files_recursive_tool(
                ListFilesRecursiveToolProps {
                    path,
                    content,
                    is_ask,
                    is_outside_workspace,
                },
                is_expanded,
                config,
            ))
        }
        
        ToolPayload::SearchFiles { path, regex, file_pattern, content, is_outside_workspace } => {
            Box::new(search_files_tool(
                SearchFilesToolProps {
                    regex,
                    path,
                    file_pattern,
                    content,
                    is_ask,
                    is_outside_workspace,
                },
                is_expanded,
                config,
            ))
        }
        
        ToolPayload::AppliedDiff { path, diff, is_protected, is_outside_workspace, .. } => {
            Box::new(apply_diff_tool(
                ApplyDiffToolProps {
                    path,
                    diff: diff.unwrap_or_default(),
                    is_ask,
                    is_protected,
                    is_outside_workspace,
                    is_partial: false,
                },
                is_expanded,
                config,
            ))
        }
        
        ToolPayload::EditedExistingFile { path, diff, is_protected, is_outside_workspace } => {
            Box::new(apply_diff_tool(
                ApplyDiffToolProps {
                    path,
                    diff: diff.unwrap_or_default(),
                    is_ask,
                    is_protected,
                    is_outside_workspace,
                    is_partial: false,
                },
                is_expanded,
                config,
            ))
        }
        
        ToolPayload::InsertContent { path, diff, line_number, is_protected, is_outside_workspace } => {
            Box::new(insert_content_tool(
                InsertContentToolProps {
                    path,
                    diff,
                    line_number,
                    is_protected,
                    is_outside_workspace,
                    is_partial: false,
                },
                is_expanded,
                config,
            ))
        }
        
        ToolPayload::SearchAndReplace { path, diff, is_protected } => {
            Box::new(search_and_replace_tool(
                SearchAndReplaceToolProps {
                    path,
                    diff,
                    is_ask,
                    is_protected,
                    is_partial: false,
                },
                is_expanded,
                config,
            ))
        }
        
        ToolPayload::NewFileCreated { path, content, is_protected } => {
            Box::new(new_file_created_tool(
                NewFileCreatedToolProps {
                    path,
                    content,
                    is_protected,
                    is_partial: false,
                },
                is_expanded,
                config,
            ))
        }
        
        ToolPayload::UpdateTodoList { todos, content } => {
            // Map ToolPayload::TodoItem to task_ops::TodoItem
            let mapped_todos = todos.into_iter().map(|t| crate::panel::ai_chat::tools::task_ops::TodoItem {
                text: t.text,
                completed: t.completed,
            }).collect();
            Box::new(update_todo_list_tool(
                UpdateTodoListToolProps {
                    todos: mapped_todos,
                    content,
                },
                config,
            ))
        }
        
        _ => {
            // Fallback for unimplemented tools
            Box::new(container(
                label(|| "Tool (renderer not implemented)".to_string())
            ).style(|s| s.padding(12.0)))
        }
    }
}

/// Render "ask" messages (requests for approval)
fn ask_message_simple(
    ask_type: AskType,
    text: String,
    is_expanded: RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> Box<dyn View> {
    match ask_type {
        AskType::Tool => {
            // Parse tool JSON and route to renderer with is_ask=true
            if let Ok(tool) = serde_json::from_str::<ToolPayload>(&text) {
                return render_tool_payload(tool, true, is_expanded, config);
            }
            // Fallback if parse fails
            let text = text.clone();
            Box::new(
                container(
                    v_stack((
                        label(|| "🔧 Tool Request".to_string())
                            .style(|s| s),
                        label(move || format!("Tool: {}", text.clone()))
                            .style(move |s| {
                                let cfg = config();
                                s.margin_top(4.0)
                                    .color(cfg.color("editor.dim"))
                            }),
                    ))
                )
                .style(move |s| {
                    let cfg = config();
                    s.padding(12.0)
                        .background(cfg.color("editor.background"))
                        .border_left(2.0)
                        .border_color(cfg.color("lapce.warn"))
                        .margin_bottom(8.0)
                })
            )
        }
        AskType::Followup => Box::new({
            // Follow-up question
            let text = text.clone();
            container(
                label(move || text.clone())
                    .style(move |s| {
                        let cfg = config();
                        s.padding(12.0)
                            .color(cfg.color("editor.foreground"))
                    })
            )
            .style(move |s| {
                let cfg = config();
                s.width_full()
                    .background(cfg.color("editor.background"))
                    .border_left(2.0)
                    .border_color(cfg.color("lapce.button_primary"))
                    .margin_bottom(8.0)
            })
        }),
        AskType::Command => Box::new({
            // Command execution request
            let text = text.clone();
            container(
                h_stack((
                    label(|| "💻".to_string())
                        .style(|s| s.margin_right(8.0)),
                    label(move || format!("Execute: {}", text.clone()))
                        .style(|s| s),
                ))
            )
            .style(move |s| {
                let cfg = config();
                s.padding(12.0)
                    .background(cfg.color("editor.background"))
                    .border_left(2.0)
                    .border_color(cfg.color("lapce.error"))
                    .margin_bottom(8.0)
            })
        }),
        AskType::McpServer => Box::new({
            // MCP server request
            let text = text.clone();
            container(
                h_stack((
                    label(|| "🔧".to_string())
                        .style(|s| s.margin_right(8.0)),
                    label(move || format!("MCP: {}", text.clone()))
                        .style(|s| s),
                ))
            )
            .style(move |s| {
                let cfg = config();
                s.padding(12.0)
                    .background(cfg.color("editor.background"))
                    .margin_bottom(8.0)
            })
        }),
    }
}
