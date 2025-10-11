// MarkdownBlock - Simplified for Phase 2
// Full markdown parsing (GFM, KaTeX, Mermaid) will be added in later phases
// For now: basic text display with monospace for code blocks

use std::sync::Arc;

use floem::{
    reactive::SignalGet,
    views::{Decorators, container, label, v_stack},
    View,
};

use crate::config::LapceConfig;

pub struct MarkdownBlockProps {
    pub markdown: String,
}

/// Simplified markdown block for Phase 2
/// TODO Phase 3+: Add full markdown parsing with:
/// - GFM tables (remark-gfm)
/// - Math equations (KaTeX)
/// - Mermaid diagrams
/// - Syntax highlighting
/// - Link handling (file:// protocol)
pub fn markdown_block(
    props: MarkdownBlockProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let markdown = props.markdown;
    
    // Phase 2: Simple text rendering
    // TODO: Parse markdown and render with proper formatting
    container(
        v_stack((
            label(move || markdown.clone())
                .style(move |s| {
                    let cfg = config();
                    s.padding(12.0)
                        .color(cfg.color("editor.foreground"))
                        .line_height(1.35)
                        .width_full()
                }),
        ))
    )
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .background(cfg.color("editor.background"))
    })
}

/// Detect if text contains code blocks
#[allow(dead_code)]
fn has_code_blocks(text: &str) -> bool {
    text.contains("```") || text.contains("~~~")
}

/// Extract language from code fence
#[allow(dead_code)]
fn extract_language(fence_line: &str) -> Option<String> {
    let trimmed = fence_line.trim_start_matches("```").trim_start_matches("~~~").trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.split_whitespace().next().unwrap_or("text").to_string())
    }
}
