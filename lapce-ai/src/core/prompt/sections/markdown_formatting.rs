//! Markdown Formatting Section
//!
//! 1:1 Translation from Codex `src/core/prompts/sections/markdown-formatting.ts`
//!
//! Reference: /home/verma/lapce/Codex/src/core/prompts/sections/markdown-formatting.ts (lines 1-8)

/// Generate markdown formatting rules section
///
/// Translation of markdownFormattingSection() from markdown-formatting.ts
pub fn markdown_formatting_section() -> String {
    r#"====

MARKDOWN RULES

ALL responses MUST show ANY `language construct` OR filename reference as clickable, exactly as [`filename OR language.declaration()`](relative/file/path.ext:line); line is required for `syntax` and optional for filename links. This applies to ALL markdown responses and ALSO those in <attempt_completion>"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_markdown_formatting_section() {
        let section = markdown_formatting_section();
        
        assert!(section.contains("===="));
        assert!(section.contains("MARKDOWN RULES"));
        assert!(section.contains("ALL responses MUST"));
        assert!(section.contains("clickable"));
        assert!(section.contains("<attempt_completion>"));
    }
}
