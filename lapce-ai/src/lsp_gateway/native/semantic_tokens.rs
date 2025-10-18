/// Semantic Tokens (textDocument/semanticTokens/full)
/// LSP-013: Provide semantic highlighting using tree-sitter highlight queries

use anyhow::{Result, anyhow};
use std::sync::Arc;

/// Semantic tokens provider
pub struct SemanticTokensProvider {
    doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>,
}

impl SemanticTokensProvider {
    pub fn new(doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>) -> Self {
        Self { doc_sync }
    }

    /// Compute semantic tokens for document
    pub async fn compute_semantic_tokens(&self, uri: &str, _language_id: &str) -> Result<String> {
        #[cfg(feature = "cst_integration")]
        {
            let doc_sync = self.doc_sync.lock();
            
            let tree = doc_sync
                .get_tree(uri)
                .ok_or_else(|| anyhow!("Document tree not found: {}", uri))?;
            
            let text = doc_sync
                .get_text(uri)
                .ok_or_else(|| anyhow!("Document text not found: {}", uri))?;
            
            // Extract semantic tokens from tree
            let mut tokens = Vec::new();
            let mut cursor = tree.root_node().walk();
            
            self.extract_tokens(&mut cursor, text.as_bytes(), &mut tokens);
            
            // Sort by position (required by LSP spec)
            tokens.sort_by(|a, b| {
                a.line.cmp(&b.line).then(a.start_char.cmp(&b.start_char))
            });
            
            // Encode to LSP format (delta encoding)
            let encoded = self.encode_tokens(&tokens);
            
            let response = SemanticTokensResponse { data: encoded };
            
            serde_json::to_string(&response)
                .map_err(|e| anyhow!("Failed to serialize semantic tokens: {}", e))
        }
        
        #[cfg(not(feature = "cst_integration"))]
        {
            tracing::warn!("Semantic tokens not available without cst_integration feature");
            Ok(r#"{"data":[]}"#.to_string())
        }
    }
    
    #[cfg(feature = "cst_integration")]
    fn extract_tokens(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        source: &[u8],
        tokens: &mut Vec<Token>,
    ) {
        loop {
            let node = cursor.node();
            let kind = node.kind();
            
            // Map node kind to semantic token type
            if let Some(token_type) = self.kind_to_token_type(kind) {
                let start = node.start_position();
                let end = node.end_position();
                
                // Skip if multi-line (complex to handle correctly)
                if start.row == end.row {
                    tokens.push(Token {
                        line: start.row as u32,
                        start_char: start.column as u32,
                        length: (end.column - start.column) as u32,
                        token_type,
                        token_modifiers: 0,
                    });
                }
            }
            
            // Recurse into children
            if cursor.goto_first_child() {
                self.extract_tokens(cursor, source, tokens);
                cursor.goto_parent();
            }
            
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    
    #[cfg(feature = "cst_integration")]
    fn kind_to_token_type(&self, kind: &str) -> Option<u32> {
        // Map tree-sitter node kinds to LSP token types
        // Token types defined in get_legend()
        match kind {
            // Keywords (0)
            "if" | "else" | "for" | "while" | "return" | "break" | "continue" |
            "fn" | "function" | "class" | "struct" | "enum" | "interface" |
            "let" | "const" | "var" | "mut" | "pub" | "private" | "public" |
            "import" | "from" | "as" | "export" | "default" => Some(0),
            
            // Types (1)
            "type_identifier" | "primitive_type" | "type_annotation" => Some(1),
            
            // Functions (2)
            "function_declaration" | "function_definition" | "method_declaration" |
            "method_definition" | "function_item" => Some(2),
            
            // Variables (3)
            "identifier" => Some(3),
            
            // Strings (4)
            "string" | "string_literal" | "char_literal" | "raw_string_literal" => Some(4),
            
            // Numbers (5)
            "number" | "integer_literal" | "float_literal" => Some(5),
            
            // Comments (6)
            "comment" | "line_comment" | "block_comment" => Some(6),
            
            // Operators (7)
            "binary_operator" | "unary_operator" => Some(7),
            
            // Classes (8)
            "class_declaration" | "struct_declaration" | "enum_declaration" => Some(8),
            
            // Properties (9)
            "property_identifier" | "field_identifier" => Some(9),
            
            // Macros (10)
            "macro_invocation" | "attribute" => Some(10),
            
            _ => None,
        }
    }
    
    #[cfg(feature = "cst_integration")]
    fn encode_tokens(&self, tokens: &[Token]) -> Vec<u32> {
        // LSP semantic tokens use delta encoding:
        // Each token is encoded as 5 integers:
        // [deltaLine, deltaStartChar, length, tokenType, tokenModifiers]
        
        let mut encoded = Vec::with_capacity(tokens.len() * 5);
        let mut prev_line = 0u32;
        let mut prev_char = 0u32;
        
        for token in tokens {
            // Delta line
            let delta_line = token.line - prev_line;
            
            // Delta start character (reset to 0 on new line)
            let delta_char = if delta_line > 0 {
                token.start_char
            } else {
                token.start_char - prev_char
            };
            
            encoded.push(delta_line);
            encoded.push(delta_char);
            encoded.push(token.length);
            encoded.push(token.token_type);
            encoded.push(token.token_modifiers);
            
            prev_line = token.line;
            prev_char = token.start_char;
        }
        
        encoded
    }

    /// Get legend (token types and modifiers)
    pub fn get_legend(&self) -> String {
        let legend = SemanticTokensLegend {
            token_types: vec![
                "keyword".to_string(),
                "type".to_string(),
                "function".to_string(),
                "variable".to_string(),
                "string".to_string(),
                "number".to_string(),
                "comment".to_string(),
                "operator".to_string(),
                "class".to_string(),
                "property".to_string(),
                "macro".to_string(),
            ],
            token_modifiers: vec![
                "declaration".to_string(),
                "readonly".to_string(),
                "static".to_string(),
                "deprecated".to_string(),
                "abstract".to_string(),
                "async".to_string(),
                "modification".to_string(),
            ],
        };
        
        serde_json::to_string(&legend).unwrap_or_else(|_| r#"{"tokenTypes":[],"tokenModifiers":[]}"#.to_string())
    }
}

impl Default for SemanticTokensProvider {
    fn default() -> Self {
        panic!("Use SemanticTokensProvider::new(doc_sync) instead of Default")
    }
}

// ============================================================================
// Internal Types
// ============================================================================

#[cfg(feature = "cst_integration")]
struct Token {
    line: u32,
    start_char: u32,
    length: u32,
    token_type: u32,
    token_modifiers: u32,
}

// ============================================================================
// LSP Types
// ============================================================================

#[derive(Debug, serde::Serialize)]
struct SemanticTokensResponse {
    data: Vec<u32>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SemanticTokensLegend {
    token_types: Vec<String>,
    token_modifiers: Vec<String>,
}
