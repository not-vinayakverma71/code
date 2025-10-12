// Markdown Renderer - Render markdown content
// Simplified markdown display with basic formatting

use std::sync::Arc;
use floem::{
    views::{container, label, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone)]
pub struct MarkdownContent {
    pub raw: String,
}

impl MarkdownContent {
    /// Parse markdown into renderable sections
    /// Simplified parser - full implementation will use proper markdown library
    pub fn parse(&self) -> Vec<MarkdownSection> {
        let mut sections = Vec::new();
        let lines = self.raw.lines();
        
        for line in lines {
            if line.starts_with("```") {
                sections.push(MarkdownSection::CodeFence { language: line.trim_start_matches("```").to_string() });
            } else if line.starts_with("# ") {
                sections.push(MarkdownSection::Heading { level: 1, text: line[2..].to_string() });
            } else if line.starts_with("## ") {
                sections.push(MarkdownSection::Heading { level: 2, text: line[3..].to_string() });
            } else if line.starts_with("### ") {
                sections.push(MarkdownSection::Heading { level: 3, text: line[4..].to_string() });
            } else if line.starts_with("- ") || line.starts_with("* ") {
                sections.push(MarkdownSection::ListItem { text: line[2..].to_string() });
            } else if line.starts_with("> ") {
                sections.push(MarkdownSection::BlockQuote { text: line[2..].to_string() });
            } else if !line.is_empty() {
                sections.push(MarkdownSection::Paragraph { text: line.to_string() });
            }
        }
        
        sections
    }
}

#[derive(Debug, Clone)]
pub enum MarkdownSection {
    Heading { level: u8, text: String },
    Paragraph { text: String },
    CodeFence { language: String },
    ListItem { text: String },
    BlockQuote { text: String },
}

/// Markdown renderer component
/// Displays formatted markdown content (simplified)
pub fn markdown_renderer(
    content: MarkdownContent,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    // Simplified: render as plain text for now
    // Full implementation will use proper markdown library and dynamic view generation
    label(move || content.raw.clone())
        .style(move |s| {
            let cfg = config();
            s.color(cfg.color("editor.foreground"))
                .font_size(13.0)
                .line_height(1.6)
                .width_full()
        })
}

/// Simple text with inline formatting
/// Handles **bold**, *italic*, `code` inline styles
pub fn formatted_text(
    text: String,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    // Simplified - full implementation would parse inline markdown
    label(move || text.clone())
        .style(move |s| {
            let cfg = config();
            s.color(cfg.color("editor.foreground"))
                .font_size(13.0)
                .line_height(1.6)
        })
}
