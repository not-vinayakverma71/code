//! SYNTAX HIGHLIGHTER - EFFICIENT HIGHLIGHTING FOR 32 LANGUAGES

use crate::native_parser_manager::{NativeParserManager, FileType};
use tree_sitter::{Query, QueryCursor, Node};
use std::sync::Arc;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct HighlightedRange {
    pub start: usize,
    pub end: usize,
    pub style: HighlightStyle,
}

#[derive(Debug, Clone)]
pub struct HighlightStyle {
    pub color: String,
    pub is_bold: bool,
    pub is_italic: bool,
}

pub struct Theme {
    styles: std::collections::HashMap<String, HighlightStyle>,
}

pub struct SyntaxHighlighter {
    parser_manager: Arc<NativeParserManager>,
    theme: Arc<Theme>,
}

impl SyntaxHighlighter {
    pub fn new(parser_manager: Arc<NativeParserManager>) -> Self {
        Self {
            parser_manager,
            theme: Arc::new(Theme::default()),
        }
    }
    
    pub async fn highlight(&self, path: &Path) -> Result<Vec<HighlightedRange>, Box<dyn std::error::Error>> {
        let parse_result = self.parser_manager.parse_file(path).await?;
        
        let mut highlights = Vec::new();
        
        // Basic highlighting based on node types
        let root = parse_result.tree.root_node();
        self.highlight_node(root, parse_result.source.as_ref(), &mut highlights);
        
        // Sort and merge overlapping ranges
        highlights.sort_by_key(|h| h.start);
        self.merge_overlapping(&mut highlights);
        
        Ok(highlights)
    }
    
    fn highlight_node(&self, node: Node, source: &[u8], highlights: &mut Vec<HighlightedRange>) {
        let style = self.get_style_for_node(&node);
        
        if let Some(style) = style {
            highlights.push(HighlightedRange {
                start: node.start_byte(),
                end: node.end_byte(),
                style,
            });
        }
        
        // Recurse through children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.highlight_node(child, source, highlights);
            }
        }
    }
    
    fn get_style_for_node(&self, node: &Node) -> Option<HighlightStyle> {
        match node.kind() {
            // Keywords
            "fn" | "function" | "def" | "class" | "struct" | "impl" | "trait" |
            "if" | "else" | "for" | "while" | "return" | "break" | "continue" |
            "import" | "export" | "package" | "module" | "use" | "pub" | "private" |
            "public" | "protected" | "static" | "const" | "let" | "var" => {
                Some(HighlightStyle {
                    color: "#FF7B72".to_string(), // Red
                    is_bold: true,
                    is_italic: false,
                })
            }
            
            // Types
            "type_identifier" | "primitive_type" | "type" => {
                Some(HighlightStyle {
                    color: "#79C0FF".to_string(), // Blue
                    is_bold: false,
                    is_italic: false,
                })
            }
            
            // Strings
            "string" | "string_literal" | "template_string" => {
                Some(HighlightStyle {
                    color: "#A5D6FF".to_string(), // Light blue
                    is_bold: false,
                    is_italic: false,
                })
            }
            
            // Numbers
            "number" | "integer" | "float" | "decimal_integer" => {
                Some(HighlightStyle {
                    color: "#79C0FF".to_string(), // Blue
                    is_bold: false,
                    is_italic: false,
                })
            }
            
            // Comments
            "comment" | "line_comment" | "block_comment" => {
                Some(HighlightStyle {
                    color: "#8B949E".to_string(), // Gray
                    is_bold: false,
                    is_italic: true,
                })
            }
            
            // Functions
            "function_declaration" | "method_definition" | "function_item" => {
                Some(HighlightStyle {
                    color: "#D2A8FF".to_string(), // Purple
                    is_bold: true,
                    is_italic: false,
                })
            }
            
            _ => None,
        }
    }
    
    fn merge_overlapping(&self, highlights: &mut Vec<HighlightedRange>) {
        if highlights.len() < 2 {
            return;
        }
        
        let mut write_idx = 0;
        for read_idx in 1..highlights.len() {
            if highlights[write_idx].end >= highlights[read_idx].start {
                // Merge overlapping ranges
                highlights[write_idx].end = highlights[write_idx].end
                    .max(highlights[read_idx].end);
            } else {
                write_idx += 1;
                highlights[write_idx] = highlights[read_idx].clone();
            }
        }
        
        highlights.truncate(write_idx + 1);
    }
}

impl Default for Theme {
    fn default() -> Self {
        let mut styles = std::collections::HashMap::new();
        
        // Default theme colors
        styles.insert("keyword".to_string(), HighlightStyle {
            color: "#FF7B72".to_string(),
            is_bold: true,
            is_italic: false,
        });
        
        styles.insert("function".to_string(), HighlightStyle {
            color: "#D2A8FF".to_string(),
            is_bold: false,
            is_italic: false,
        });
        
        styles.insert("type".to_string(), HighlightStyle {
            color: "#79C0FF".to_string(),
            is_bold: false,
            is_italic: false,
        });
        
        styles.insert("string".to_string(), HighlightStyle {
            color: "#A5D6FF".to_string(),
            is_bold: false,
            is_italic: false,
        });
        
        styles.insert("comment".to_string(), HighlightStyle {
            color: "#8B949E".to_string(),
            is_bold: false,
            is_italic: true,
        });
        
        Self { styles }
    }
}

impl Theme {
    pub fn get_style(&self, capture_name: &str) -> HighlightStyle {
        self.styles.get(capture_name)
            .cloned()
            .unwrap_or(HighlightStyle {
                color: "#C9D1D9".to_string(), // Default white
                is_bold: false,
                is_italic: false,
            })
    }
}
