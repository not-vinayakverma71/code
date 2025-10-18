/// Hover Information (textDocument/hover)
/// LSP-009: Show signatures and doc comments at cursor position

use anyhow::{Result, anyhow};
use std::sync::Arc;

#[cfg(feature = "cst_integration")]
use lapce_tree_sitter::cst_api::CstApi;

/// Hover information provider
pub struct HoverProvider {
    doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>,
}

impl HoverProvider {
    pub fn new(doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>) -> Self {
        Self { doc_sync }
    }

    /// Get hover information at position
    pub async fn get_hover(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<String>> {
        #[cfg(feature = "cst_integration")]
        {
            let doc_sync = self.doc_sync.lock();
            
            // Get document text and tree
            let text = doc_sync
                .get_text(uri)
                .ok_or_else(|| anyhow!("Document not found: {}", uri))?;
            
            let tree = doc_sync
                .get_tree(uri)
                .ok_or_else(|| anyhow!("Document tree not found: {}", uri))?;
            
            // Convert LSP position to byte offset
            let byte_offset = position_to_byte(text, line, character);
            
            // Create CST API and find node at position
            let cst_api = create_cst_api_from_tree(tree, text.as_bytes())?;
            
            let node = match cst_api.find_node_at_position(byte_offset) {
                Some(n) => n,
                None => {
                    tracing::debug!("No node found at position {}:{}", line, character);
                    return Ok(None);
                }
            };
            
            // Extract hover information based on node kind
            let hover_text = extract_hover_info(&node, text.as_bytes(), &cst_api)?;
            
            if hover_text.is_empty() {
                return Ok(None);
            }
            
            // Format as LSP Hover response
            let hover = LspHover {
                contents: LspMarkupContent {
                    kind: "markdown".to_string(),
                    value: hover_text,
                },
                range: Some(LspRange {
                    start: LspPosition {
                        line: byte_to_position(text.as_bytes(), node.start_byte).0,
                        character: byte_to_position(text.as_bytes(), node.start_byte).1,
                    },
                    end: LspPosition {
                        line: byte_to_position(text.as_bytes(), node.end_byte).0,
                        character: byte_to_position(text.as_bytes(), node.end_byte).1,
                    },
                }),
            };
            
            serde_json::to_string(&hover)
                .map(Some)
                .map_err(|e| anyhow!("Failed to serialize hover: {}", e))
        }
        
        #[cfg(not(feature = "cst_integration"))]
        {
            tracing::warn!("Hover not available without cst_integration feature");
            Ok(None)
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

#[cfg(feature = "cst_integration")]
fn create_cst_api_from_tree(tree: &tree_sitter::Tree, source: &[u8]) -> Result<CstApi> {
    use lapce_tree_sitter::compact::bytecode::tree_sitter_encoder::TreeSitterBytecodeEncoder;
    
    // Encode tree to bytecode for CST API
    let mut encoder = TreeSitterBytecodeEncoder::new();
    let stream = encoder.encode_tree(tree, source);
    
    Ok(CstApi::from_bytecode(stream))
}

#[cfg(feature = "cst_integration")]
fn extract_hover_info(
    node: &lapce_tree_sitter::compact::bytecode::decoder::DecodedNode,
    source: &[u8],
    cst_api: &CstApi,
) -> Result<String> {
    use lapce_tree_sitter::ast::kinds::map_kind;
    
    // Determine language from context (we'll need to pass this through)
    // For now, use generic extraction
    let mut hover_parts = Vec::new();
    
    // Extract node signature
    let node_text = extract_node_text(source, node.start_byte, node.end_byte);
    let kind_name = &node.kind_name;
    
    // Add type/kind information
    hover_parts.push(format!("**{}**", kind_name));
    
    // Add node text (limited to 200 chars for preview)
    let preview = if node_text.len() > 200 {
        format!("{} ...", &node_text[..200])
    } else {
        node_text.clone()
    };
    
    hover_parts.push(format!("```\n{}\n```", preview));
    
    // Try to find preceding doc comment
    if let Some(doc_comment) = find_doc_comment(node, source, cst_api) {
        hover_parts.push(String::new()); // Empty line
        hover_parts.push(doc_comment);
    }
    
    // Add position information
    hover_parts.push(String::new());
    hover_parts.push(format!(
        "*Range: {}:{}-{}:{}*",
        byte_to_position(source, node.start_byte).0,
        byte_to_position(source, node.start_byte).1,
        byte_to_position(source, node.end_byte).0,
        byte_to_position(source, node.end_byte).1
    ));
    
    Ok(hover_parts.join("\n"))
}

#[cfg(feature = "cst_integration")]
fn find_doc_comment(
    node: &lapce_tree_sitter::compact::bytecode::decoder::DecodedNode,
    source: &[u8],
    cst_api: &CstApi,
) -> Option<String> {
    // Look for comment node immediately preceding this node
    let target_start = node.start_byte;
    
    // Search backwards for comment nodes
    if target_start < 10 {
        return None;
    }
    
    let search_start = target_start.saturating_sub(500); // Look back up to 500 bytes
    let nodes = cst_api.get_range_nodes(search_start..target_start);
    
    // Find last comment before our node
    for node in nodes.iter().rev() {
        if node.kind_name.contains("comment") && node.end_byte <= target_start {
            let comment_text = extract_node_text(source, node.start_byte, node.end_byte);
            // Clean up comment markers
            let cleaned = comment_text
                .lines()
                .map(|line| {
                    line.trim()
                        .trim_start_matches("//")
                        .trim_start_matches("///")
                        .trim_start_matches("#")
                        .trim_start_matches("/*")
                        .trim_end_matches("*/")
                        .trim()
                })
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    
    None
}

fn extract_node_text(source: &[u8], start_byte: usize, end_byte: usize) -> String {
    let slice = &source[start_byte..end_byte.min(source.len())];
    String::from_utf8_lossy(slice).to_string()
}

fn position_to_byte(text: &str, line: u32, character: u32) -> usize {
    let mut byte_offset = 0;
    let mut current_line = 0u32;
    
    for (i, ch) in text.char_indices() {
        if current_line == line {
            let line_start = byte_offset;
            let mut char_count = 0u32;
            
            for (j, _) in text[line_start..].char_indices() {
                if char_count == character {
                    return line_start + j;
                }
                char_count += 1;
            }
            return text.len();
        }
        
        if ch == '\n' {
            current_line += 1;
        }
        
        byte_offset = i + ch.len_utf8();
    }
    
    byte_offset
}

fn byte_to_position(source: &[u8], byte_offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut column = 0u32;
    
    for (i, &byte) in source.iter().enumerate() {
        if i >= byte_offset {
            break;
        }
        if byte == b'\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    
    (line, column)
}

// ============================================================================
// LSP Types
// ============================================================================

#[derive(Debug, serde::Serialize)]
struct LspHover {
    contents: LspMarkupContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<LspRange>,
}

#[derive(Debug, serde::Serialize)]
struct LspMarkupContent {
    kind: String,
    value: String,
}

#[derive(Debug, serde::Serialize)]
struct LspRange {
    start: LspPosition,
    end: LspPosition,
}

#[derive(Debug, serde::Serialize)]
struct LspPosition {
    line: u32,
    character: u32,
}
