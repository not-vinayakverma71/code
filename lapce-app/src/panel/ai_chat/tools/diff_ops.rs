// Diff Operation Tool Renderers
// Ported from ChatRow.tsx lines 353-477

use std::sync::Arc;

use floem::{
    reactive::SignalUpdate,
    views::{Decorators, container, h_stack, label, v_stack}, View,
};

use crate::{
    config::LapceConfig,
    panel::ai_chat::shared::code_accordion::{CodeAccordionProps, code_accordion},
};

/// ApplyDiff tool renderer (single file)
/// Source: ChatRow.tsx lines 354-407
pub struct ApplyDiffToolProps {
    pub path: String,
    pub diff: String,
    pub is_ask: bool,
    pub is_protected: bool,
    pub is_outside_workspace: bool,
    pub is_partial: bool,
}

pub fn apply_diff_tool(
    props: ApplyDiffToolProps,
    is_expanded: floem::reactive::RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let path = props.path.clone();
    let diff = props.diff.clone();
    
    v_stack((
        // Header
        container(
            h_stack((
                label(|| "📝".to_string())
                    .style(|s| s.margin_right(8.0)),
                label(move || {
                    if props.is_protected {
                        "wants to edit protected file".to_string()
                    } else if props.is_outside_workspace {
                        "wants to edit file outside workspace".to_string()
                    } else {
                        "wants to edit".to_string()
                    }
                })
                .style(|s| s),
            ))
        )
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .color(if props.is_protected {
                    cfg.color("editor.warnForeground")
                } else {
                    cfg.color("editor.foreground")
                })
        }),
        
        // Diff display
        container(
            code_accordion(
                CodeAccordionProps {
                    path: Some(path),
                    code: diff,
                    language: Some("diff".to_string()),
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

/// InsertContent tool renderer  
/// Source: ChatRow.tsx lines 408-444
pub struct InsertContentToolProps {
    pub path: String,
    pub diff: String,
    pub line_number: usize,
    pub is_protected: bool,
    pub is_outside_workspace: bool,
    pub is_partial: bool,
}

pub fn insert_content_tool(
    props: InsertContentToolProps,
    is_expanded: floem::reactive::RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let path = props.path.clone();
    let diff = props.diff.clone();
    let line_number = props.line_number;
    
    v_stack((
        // Header
        container(
            h_stack((
                label(|| "➕".to_string())
                    .style(|s| s.margin_right(8.0)),
                label(move || {
                    if props.is_protected {
                        "wants to edit protected file".to_string()
                    } else if props.is_outside_workspace {
                        "wants to edit file outside workspace".to_string()
                    } else if line_number == 0 {
                        "wants to insert at end".to_string()
                    } else {
                        format!("wants to insert at line {}", line_number)
                    }
                })
                .style(|s| s),
            ))
        )
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .color(if props.is_protected {
                    cfg.color("editor.warnForeground")
                } else {
                    cfg.color("editor.foreground")
                })
        }),
        
        // Diff display
        container(
            code_accordion(
                CodeAccordionProps {
                    path: Some(path),
                    code: diff,
                    language: Some("diff".to_string()),
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

/// SearchAndReplace tool renderer
/// Source: ChatRow.tsx lines 445-477
pub struct SearchAndReplaceToolProps {
    pub path: String,
    pub diff: String,
    pub is_ask: bool,
    pub is_protected: bool,
    pub is_partial: bool,
}

pub fn search_and_replace_tool(
    props: SearchAndReplaceToolProps,
    is_expanded: floem::reactive::RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let path = props.path.clone();
    let diff = props.diff.clone();
    
    v_stack((
        // Header
        container(
            h_stack((
                label(|| "🔄".to_string())
                    .style(|s| s.margin_right(8.0)),
                label(move || {
                    if props.is_protected && props.is_ask {
                        "wants to edit protected file".to_string()
                    } else if props.is_ask {
                        "wants to search and replace".to_string()
                    } else {
                        "did search and replace".to_string()
                    }
                })
                .style(|s| s),
            ))
        )
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .color(if props.is_protected {
                    cfg.color("editor.warnForeground")
                } else {
                    cfg.color("editor.foreground")
                })
        }),
        
        // Diff display
        container(
            code_accordion(
                CodeAccordionProps {
                    path: Some(path),
                    code: diff,
                    language: Some("diff".to_string()),
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

/// NewFileCreated tool renderer
/// Source: ChatRow.tsx lines 515-550
pub struct NewFileCreatedToolProps {
    pub path: String,
    pub content: String,
    pub is_protected: bool,
    pub is_partial: bool,
}

pub fn new_file_created_tool(
    props: NewFileCreatedToolProps,
    is_expanded: floem::reactive::RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let path = props.path.clone();
    let content = props.content.clone();
    
    // Detect language from path
    let language = crate::panel::ai_chat::utils::language_detection::get_language_from_path(&path)
        .unwrap_or("log")
        .to_string();
    
    v_stack((
        // Header
        container(
            h_stack((
                label(|| "📄".to_string())
                    .style(|s| s.margin_right(8.0)),
                label(move || {
                    if props.is_protected {
                        "wants to edit protected file".to_string()
                    } else {
                        "wants to create".to_string()
                    }
                })
                .style(|s| s),
            ))
        )
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .color(if props.is_protected {
                    cfg.color("editor.warnForeground")
                } else {
                    cfg.color("editor.foreground")
                })
        }),
        
        // File content display
        container(
            code_accordion(
                CodeAccordionProps {
                    path: Some(path),
                    code: content,
                    language: Some(language),
                    is_expanded,
                    on_toggle: Box::new(move || {
                        is_expanded.update(|v| *v = !*v);
                    }),
                    on_jump_to_file: None, // TODO: Wire to editor when available
                },
                config,
            )
        )
        .style(|s| s.padding_left(24.0)),
    ))
    .style(|s| s.width_full())
}
