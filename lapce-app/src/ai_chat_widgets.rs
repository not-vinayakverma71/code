/// AI Chat Widgets - Native Floem components matching Windsurf styling
///
/// Components:
/// - InlineCode: Styled code snippets
/// - ChatMessage: User/AI message with proper styling
/// - ChatPanel: Scrollable container for messages

use floem::{
    reactive::{RwSignal, SignalGet},
    unit::PxPctAuto,
    View, IntoView,
    views::{
        container, label, scroll, v_stack_from_iter, Decorators, dyn_stack,
    },
};

use crate::ai_theme::{AiTheme, font_size, spacing};

/// Role of a chat message
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageRole {
    User,
    Assistant,
}

/// A single chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub is_streaming: bool,
}

/// Inline code block widget - Windsurf styling
/// 
/// Uses: #313131 background, #cccccc foreground
pub fn inline_code(text: impl Into<String>, theme: AiTheme) -> impl View {
    let text = text.into();
    
    label(move || text.clone())
        .style(move |s| {
            s.font_family(theme.editor_font_family.to_string())
                .font_size(font_size::XS)
                .background(theme.inline_code_background)  // #313131
                .color(theme.inline_code_foreground)       // #cccccc
                .padding_horiz(spacing::SPACE_1)
                .padding_vert(spacing::SPACE_05)
                .border_radius(spacing::ROUNDED)
                .font_weight(floem::text::Weight::MEDIUM)
        })
}

/// Code block widget (multi-line) - Windsurf styling
///
/// Uses: #202020 background, rounded corners, padding
pub fn code_block(code: impl Into<String>, theme: AiTheme) -> impl View {
    let code = code.into();
    
    container(
        label(move || code.clone())
            .style(move |s| {
                s.font_family(theme.editor_font_family.to_string())
                    .font_size(font_size::BASE)
                    .color(theme.foreground)
            })
    )
    .style(move |s| {
        s.background(theme.code_block_background)  // #202020
            .padding(spacing::SPACE_3)              // 12px
            .border_radius(spacing::ROUNDED_MD)     // 6px
            .width(PxPctAuto::Pct(100.0))
    })
}

/// Chat message view - Windsurf exact colors
///
/// User messages: rgba(31,31,31,0.62) background
/// AI messages: #1f1f1f background, dimmed text (55% opacity)
pub fn chat_message_view(message: ChatMessage, theme: AiTheme) -> impl View {
    let is_ai = message.role == MessageRole::Assistant;
    let content = message.content.clone();
    let is_streaming = message.is_streaming;

    // Parse simple markdown: detect inline `code` and code blocks
    let parts = parse_markdown_simple(&content);
    
    // Text color based on role
    let text_color = if is_ai { 
        theme.message_bot_foreground  // Dimmed for AI
    } else { 
        theme.message_user_foreground // Normal for user
    };
    
    // Background based on role - exact Windsurf colors
    let bg_color = if is_ai {
        theme.message_bot_background   // #1f1f1f
    } else {
        theme.message_user_background  // rgba(31,31,31,0.62)
    };
    
    let font_family = theme.font_family.to_string();
    let font_size = theme.font_size;
    let theme_for_parts = theme.clone();

    container(
        v_stack_from_iter(
            parts.into_iter().map(move |part| {
                let theme_clone = theme_for_parts.clone();
                match part {
                    MarkdownPart::Text(text) => {
                        let font_family = font_family.clone();
                        label(move || text.clone())
                            .style(move |s| {
                                s.font_family(font_family.to_string())
                                    .font_size(font_size)
                                    .color(text_color)
                                    .line_height(1.5)
                            })
                            .into_any()
                    },
                    MarkdownPart::InlineCode(code) => {
                        inline_code(code, theme_clone.clone()).into_any()
                    },
                    MarkdownPart::CodeBlock(code) => {
                        code_block(code, theme_clone).into_any()
                    },
                }
            })
        )
        .style(move |s| {
            s.flex_col()
                .gap(spacing::SPACE_2)
                .width(PxPctAuto::Pct(100.0))
        })
    )
    .style(move |s| {
        s.background(bg_color)
            .border(1.0)
            .border_color(theme.message_border)    // rgba(255,255,255,0.1)
            .border_radius(spacing::ROUNDED_MD)    // 6px
            .padding(spacing::SPACE_3)              // 12px
            .width(PxPctAuto::Pct(100.0))
    })
}

/// Simple markdown parser
/// Detects:
/// - Inline code: `code`
/// - Code blocks: ```code```
#[derive(Debug, Clone)]
enum MarkdownPart {
    Text(String),
    InlineCode(String),
    CodeBlock(String),
}

fn parse_markdown_simple(content: &str) -> Vec<MarkdownPart> {
    let mut parts = Vec::new();
    let mut current_text = String::new();
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '`' {
            // Check for code block (```)
            if chars.peek() == Some(&'`') {
                chars.next(); // consume second `
                if chars.peek() == Some(&'`') {
                    chars.next(); // consume third `
                    
                    // Push accumulated text
                    if !current_text.is_empty() {
                        parts.push(MarkdownPart::Text(current_text.clone()));
                        current_text.clear();
                    }
                    
                    // Read until closing ```
                    let mut code = String::new();
                    let mut backtick_count = 0;
                    while let Some(c) = chars.next() {
                        if c == '`' {
                            backtick_count += 1;
                            if backtick_count == 3 {
                                break;
                            }
                        } else {
                            // Reset if not consecutive backticks
                            for _ in 0..backtick_count {
                                code.push('`');
                            }
                            backtick_count = 0;
                            code.push(c);
                        }
                    }
                    parts.push(MarkdownPart::CodeBlock(code.trim().to_string()));
                    continue;
                }
            }
            
            // Inline code
            if !current_text.is_empty() {
                parts.push(MarkdownPart::Text(current_text.clone()));
                current_text.clear();
            }
            
            let mut code = String::new();
            while let Some(c) = chars.next() {
                if c == '`' {
                    break;
                }
                code.push(c);
            }
            parts.push(MarkdownPart::InlineCode(code));
        } else {
            current_text.push(ch);
        }
    }

    if !current_text.is_empty() {
        parts.push(MarkdownPart::Text(current_text));
    }

    parts
}

/// Chat panel container - Windsurf styling
///
/// Background: #202020, border: #454545, shadow: rgba(0,0,0,0.36)
pub fn chat_panel(
    messages: RwSignal<Vec<ChatMessage>>,
    theme: AiTheme,
) -> impl View {
    let theme_for_stack = theme.clone();
    let theme_for_style = theme.clone();
    
    scroll(
        dyn_stack(
            move || messages.get(),
            move |msg| (msg.role, msg.content.clone()),
            move |msg| chat_message_view(msg, theme_for_stack.clone())
        )
        .style(move |s| {
            s.flex_col()
                .gap(spacing::SPACE_4)  // 16px between messages
                .padding(spacing::SPACE_4)  // 16px panel padding
                .width(PxPctAuto::Pct(100.0))
        })
    )
    .style(move |s| {
        s.background(theme_for_style.chat_background)  // #202020
            .border(1.0)
            .border_color(theme_for_style.chat_border)  // #454545
            .border_radius(spacing::ROUNDED_LG)         // 8px
            .box_shadow_blur(12.0)
            .box_shadow_color(theme_for_style.chat_shadow)  // rgba(0,0,0,0.36)
            .width(PxPctAuto::Pct(100.0))
            .height(PxPctAuto::Pct(100.0))
    })
}

/// Streaming cursor animation
///
/// Blinking cursor that appears during AI streaming
pub fn streaming_cursor(theme: AiTheme) -> impl View {
    label(|| "▊")
        .style(move |s| {
            s.color(theme.ai_text_medium)
                .font_size(font_size::BASE)
                // TODO: Add animation when Floem animation API is stable
                // .animation(
                //     Animation::new(500.ms())
                //         .repeat()
                //         .keyframe(0, |s| s.opacity(1.0))
                //         .keyframe(50, |s| s.opacity(0.0))
                // )
        })
}

/// Copy button for messages
///
/// Small button that appears on hover
pub fn copy_button(content: String, theme: AiTheme) -> impl View {
    let dim_color = theme.foreground.multiply_alpha(0.7);
    let hover_bg = theme.active_background;
    let fg = theme.foreground;
    
    container(
        label(|| "Copy".to_string())
    )
    .on_click_stop(move |_| {
        // TODO: Wire to clipboard when available
        println!("Copy: {}", content);
    })
    .style(move |s| {
        s.font_size(font_size::XS)
            .padding_horiz(spacing::SPACE_2)
            .padding_vert(spacing::SPACE_1)
            .background(theme.hover_background)
            .color(dim_color)
            .border_radius(spacing::ROUNDED)
            .cursor(floem::style::CursorStyle::Pointer)
            .hover(move |s| {
                s.background(hover_bg)
                    .color(fg)
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_parser_inline_code() {
        let content = "Use `cargo build` to compile";
        let parts = parse_markdown_simple(content);
        assert_eq!(parts.len(), 3);
        matches!(parts[0], MarkdownPart::Text(_));
        matches!(parts[1], MarkdownPart::InlineCode(_));
        matches!(parts[2], MarkdownPart::Text(_));
    }

    #[test]
    fn test_markdown_parser_code_block() {
        let content = "Example:\n```\nfn main() {}\n```\nDone";
        let parts = parse_markdown_simple(content);
        assert!(parts.iter().any(|p| matches!(p, MarkdownPart::CodeBlock(_))));
    }

    #[test]
    fn test_markdown_parser_multiple() {
        let content = "Run `npm install` then:\n```\nnpm start\n```";
        let parts = parse_markdown_simple(content);
        assert!(parts.len() >= 3);
    }
}
