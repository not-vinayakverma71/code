// File Operation Tool Renderers
// Ported from ChatRow.tsx lines 551-747

use std::sync::Arc;

use floem::{
    event::EventListener,
    reactive::{SignalGet, SignalUpdate},
    views::{Decorators, container, h_stack, label, v_stack},
    IntoView, View,
};

use crate::{
    config::LapceConfig,
    panel::ai_chat::{
        shared::{
            code_accordion::{CodeAccordionProps, code_accordion},
            tool_use_block::{tool_use_block, tool_use_block_header},
        },
        utils::path_utils::remove_leading_non_alphanumeric,
    },
};

/// ReadFile tool renderer
/// Source: ChatRow.tsx lines 551-611
pub struct ReadFileToolProps {
    pub path: String,
    pub content: String,
    pub is_ask: bool,
    pub is_outside_workspace: bool,
    pub additional_file_count: Option<usize>,
    pub reason: Option<String>,
}

pub fn read_file_tool(
    props: ReadFileToolProps,
    is_expanded: floem::reactive::RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let path = props.path.clone();
    let path_display = remove_leading_non_alphanumeric(&path).to_string();
    let reason = props.reason.clone();
    
    v_stack((
        // Header
        container(
            h_stack((
                label(|| "📄".to_string())
                    .style(|s| s.margin_right(8.0)),
                label(move || {
                    if props.is_ask {
                        if props.is_outside_workspace {
                            "wants to read (outside workspace)".to_string()
                        } else if let Some(count) = props.additional_file_count {
                            if count > 0 {
                                format!("wants to read and {} more", count)
                            } else {
                                "wants to read".to_string()
                            }
                        } else {
                            "wants to read".to_string()
                        }
                    } else {
                        "did read".to_string()
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
        
        // File path display
        container(
            tool_use_block(
                tool_use_block_header(
                    h_stack((
                        label(move || path_display.clone())
                            .style(|s| s.margin_right(8.0)),
                        reason.clone().map(|r| {
                            label(move || r.clone())
                                .style(move |s| {
                                    let cfg = config();
                                    s.color(cfg.color("editor.dim"))
                                        .margin_left(8.0)
                                })
                                .into_any()
                        }).unwrap_or_else(|| container(label(|| "")).into_any()),
                        container(label(|| "↗".to_string()))
                            .style(|s| s.margin_left(8.0)),
                    )),
                    None::<fn()>,
                    config,
                ),
                config,
            )
        )
        .style(|s| s.padding_left(24.0)),
    ))
    .style(|s| s.width_full())
}

/// ListFilesTopLevel tool renderer
/// Source: ChatRow.tsx lines 630-655
pub struct ListFilesTopLevelToolProps {
    pub path: String,
    pub content: String,
    pub is_ask: bool,
    pub is_outside_workspace: bool,
}

pub fn list_files_top_level_tool(
    props: ListFilesTopLevelToolProps,
    is_expanded: floem::reactive::RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let path = props.path.clone();
    let content = props.content.clone();
    
    v_stack((
        // Header
        container(
            h_stack((
                label(|| "📋".to_string())
                    .style(|s| s.margin_right(8.0)),
                label(move || {
                    if props.is_ask {
                        if props.is_outside_workspace {
                            "wants to view top level (outside workspace)".to_string()
                        } else {
                            "wants to view top level".to_string()
                        }
                    } else if props.is_outside_workspace {
                        "did view top level (outside workspace)".to_string()
                    } else {
                        "did view top level".to_string()
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
        
        // File listing
        container(
            code_accordion(
                CodeAccordionProps {
                    path: Some(path),
                    code: content,
                    language: Some("shell-session".to_string()),
                    is_expanded,
                    on_toggle: Box::new(move || {
                        is_expanded.update(|v| *v = !*v);
                    }),
                    on_jump_to_file: None,
                },
                config,
            )
        )
        .style(|s| s.padding_left(24.0)),
    ))
    .style(|s| s.width_full())
}

/// ListFilesRecursive tool renderer
/// Source: ChatRow.tsx lines 656-681
pub struct ListFilesRecursiveToolProps {
    pub path: String,
    pub content: String,
    pub is_ask: bool,
    pub is_outside_workspace: bool,
}

pub fn list_files_recursive_tool(
    props: ListFilesRecursiveToolProps,
    is_expanded: floem::reactive::RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let path = props.path.clone();
    let content = props.content.clone();
    
    v_stack((
        // Header
        container(
            h_stack((
                label(|| "🗂".to_string())
                    .style(|s| s.margin_right(8.0)),
                label(move || {
                    if props.is_ask {
                        if props.is_outside_workspace {
                            "wants to view recursive (outside workspace)".to_string()
                        } else {
                            "wants to view recursive".to_string()
                        }
                    } else if props.is_outside_workspace {
                        "did view recursive (outside workspace)".to_string()
                    } else {
                        "did view recursive".to_string()
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
        
        // Recursive listing
        container(
            code_accordion(
                CodeAccordionProps {
                    path: Some(path),
                    code: content,
                    language: Some("shellsession".to_string()),
                    is_expanded,
                    on_toggle: Box::new(move || {
                        is_expanded.update(|v| *v = !*v);
                    }),
                    on_jump_to_file: None,
                },
                config,
            )
        )
        .style(|s| s.padding_left(24.0)),
    ))
    .style(|s| s.width_full())
}

/// SearchFiles tool renderer
/// Source: ChatRow.tsx lines 708-747
pub struct SearchFilesToolProps {
    pub regex: String,
    pub path: String,
    pub file_pattern: Option<String>,
    pub content: String,
    pub is_ask: bool,
    pub is_outside_workspace: bool,
}

pub fn search_files_tool(
    props: SearchFilesToolProps,
    is_expanded: floem::reactive::RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let regex = props.regex.clone();
    let path = props.path.clone();
    let file_pattern = props.file_pattern.clone();
    let content = props.content.clone();
    
    let display_path = if let Some(pattern) = file_pattern {
        format!("{}/({}))", path, pattern)
    } else {
        path
    };
    
    v_stack((
        // Header
        container(
            h_stack((
                label(|| "🔍".to_string())
                    .style(|s| s.margin_right(8.0)),
                label(move || {
                    if props.is_ask {
                        if props.is_outside_workspace {
                            format!("wants to search for '{}' (outside workspace)", regex)
                        } else {
                            format!("wants to search for '{}'", regex)
                        }
                    } else if props.is_outside_workspace {
                        format!("did search for '{}' (outside workspace)", regex)
                    } else {
                        format!("did search for '{}'", regex)
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
        
        // Search results
        container(
            code_accordion(
                CodeAccordionProps {
                    path: Some(display_path),
                    code: content,
                    language: Some("shellsession".to_string()),
                    is_expanded,
                    on_toggle: Box::new(move || {
                        is_expanded.update(|v| *v = !*v);
                    }),
                    on_jump_to_file: None,
                },
                config,
            )
        )
        .style(|s| s.padding_left(24.0)),
    ))
    .style(|s| s.width_full())
}
