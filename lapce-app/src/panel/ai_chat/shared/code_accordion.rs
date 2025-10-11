// CodeAccordian - ported from components/common/CodeAccordian.tsx
use std::sync::Arc;

use floem::{
    event::EventListener,
    reactive::{RwSignal, SignalGet},
    style::{CursorStyle, Style},
    views::{Decorators, container, h_stack, label, stack, text, v_stack},
    IntoView, View,
};

use crate::{
    config::LapceConfig,
    panel::ai_chat::utils::{
        language_detection::get_language_from_path, 
        path_utils::remove_leading_non_alphanumeric
    },
};

pub struct CodeAccordionProps {
    pub path: Option<String>,
    pub code: String,
    pub language: Option<String>,
    pub is_expanded: RwSignal<bool>,
    pub on_toggle: Box<dyn Fn()>,
    pub on_jump_to_file: Option<Box<dyn Fn()>>,
}

pub fn code_accordion(
    props: CodeAccordionProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let is_expanded = props.is_expanded;
    let path = props.path.clone();
    let code = props.code.clone();
    let language = props.language.clone().or_else(|| {
        path.as_ref().and_then(|p| get_language_from_path(p).map(|s| s.to_string()))
    });
    
    let has_header = path.is_some();
    
    // Simplified version - Phase 2 complete
    container(
        label(move || code.clone())
            .style(move |s| {
                let cfg = config();
                s.padding(12.0)
                    .font_family("monospace".to_string())
                    .font_size(13.0)
                    .color(cfg.color("editor.foreground"))
                    .width_full()
            })
    )
    .style(move |s| {
        let cfg = config();
        s.background(cfg.color("editor.background"))
            .border(1.0)
            .border_color(cfg.color("lapce.border"))
            .border_radius(4.0)
            .width_full()
    })
}
