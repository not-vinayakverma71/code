// Code Block - Syntax highlighted code with copy button
// Matches Windsurf's code display with header and actions

use std::sync::Arc;
use floem::{
    peniko::Color,
    views::{Decorators, container, h_stack, label, svg, v_stack},
    View,
};

use crate::{
    config::LapceConfig,
    panel::ai_chat::icons::*,
};

pub struct CodeBlockProps {
    pub code: String,
    pub language: Option<String>,
    pub filename: Option<String>,
    pub show_line_numbers: bool,
}

/// Complete code block with header and syntax highlighting
pub fn code_block(
    props: CodeBlockProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let code = props.code.clone();
    let language = props.language.clone();
    let filename = props.filename;
    
    v_stack((
        // Header bar
        code_header(language.clone(), filename, config),
        
        // Code content
        code_content(code, config),
    ))
    .style(|s| {
        s.width_full()
            .border_radius(6.0)
            .border(1.0)
            .border_color(Color::from_rgb8(0x45, 0x45, 0x45))
            .max_width_pct(90.0)
    })
}

/// Code block header with language and copy button
fn code_header(
    language: Option<String>,
    filename: Option<String>,
    _config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let code_lang = language.clone().or(filename).unwrap_or_else(|| "Code".to_string());
    
    h_stack((
        // Language/filename label
        label(move || code_lang.clone())
            .style(|s| {
                s.font_size(12.0)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.7))
            }),
        
        // Spacer
        container(floem::views::empty())
            .style(|s| s.flex_grow(1.0)),
        
        // Copy button
        copy_button(),
    ))
    .style(|s| {
        s.width_full()
            .padding(8.0)
            .items_center()
            .background(Color::from_rgb8(0x20, 0x20, 0x20))
            .border_bottom(1.0)
            .border_color(Color::from_rgb8(0x45, 0x45, 0x45))
    })
}

/// Code content area
fn code_content(
    code: String,
    _config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    // TODO: Add syntax highlighting using tree-sitter
    container(
        label(move || code.clone())
            .style(|s| {
                s.font_family("monospace".to_string())
                    .font_size(13.0)
                    .line_height(1.5)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
            })
    )
    .style(|s| {
        s.width_full()
            .padding(12.0)
            .background(Color::from_rgb8(0x20, 0x20, 0x20))
            .max_height(400.0)
    })
}

/// Copy button for code blocks
fn copy_button() -> impl View {
    container(
        label(|| "📋 Copy".to_string())
    )
    .on_click_stop(|_| {
        println!("[CodeBlock] Copy clicked!");
        // TODO: Wire to clipboard
    })
    .style(|s| {
        s.padding(4.0)
            .padding_horiz(8.0)
            .border_radius(4.0)
            .font_size(11.0)
            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.7))
            .cursor(floem::style::CursorStyle::Pointer)
            .hover(|s| {
                s.background(Color::from_rgb8(0xff, 0xff, 0xff).multiply_alpha(0.1))
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
            })
    })
}

/// Inline code (backtick style)
pub fn inline_code(
    code: String,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    label(move || code.clone())
        .style(move |s| {
            let cfg = config();
            s.font_family("monospace".to_string())
                .font_size(13.0)
                .padding_horiz(4.0)
                .padding_vert(2.0)
                .border_radius(3.0)
                .background(cfg.color("editor.foreground").multiply_alpha(0.1))
                .color(cfg.color("editor.foreground").multiply_alpha(0.9))
        })
}
