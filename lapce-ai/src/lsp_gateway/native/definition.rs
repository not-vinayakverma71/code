/// Go to Definition (textDocument/definition)
/// LSP-010: Navigate to symbol definitions using SymbolIndex

use anyhow::{Result, anyhow};
use std::sync::Arc;

/// Definition provider
pub struct DefinitionProvider {
    doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>,
    symbol_index: Arc<parking_lot::Mutex<super::SymbolIndex>>,
}

impl DefinitionProvider {
    pub fn new(
        doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>,
        symbol_index: Arc<parking_lot::Mutex<super::SymbolIndex>>,
    ) -> Self {
        Self {
            doc_sync,
            symbol_index,
        }
    }

    /// Find definition location for symbol at position
    pub async fn find_definition(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<String>> {
        // Get document text
        let doc_sync = self.doc_sync.lock();
        let text = doc_sync
            .get_text(uri)
            .ok_or_else(|| anyhow!("Document not found: {}", uri))?;
        
        // Find symbol at cursor position
        let symbol_name = {
            let index = self.symbol_index.lock();
            index.find_symbol_at_position(uri, line, character)
        };
        
        if let Some(name) = symbol_name {
            // Found a symbol, look up its definition
            let index = self.symbol_index.lock();
            
            if let Some(location) = index.find_definition(&name) {
                // Format as LSP Location
                let lsp_location = LspLocation {
                    uri: location.uri.clone(),
                    range: LspRange {
                        start: LspPosition {
                            line: location.line,
                            character: location.character,
                        },
                        end: LspPosition {
                            line: location.end_line,
                            character: location.end_character,
                        },
                    },
                };
                
                return serde_json::to_string(&lsp_location)
                    .map(Some)
                    .map_err(|e| anyhow!("Failed to serialize location: {}", e));
            }
        }
        
        // Try fallback: extract identifier at position and search by name
        let identifier = extract_identifier_at_position(text, line, character);
        
        if let Some(ident) = identifier {
            let index = self.symbol_index.lock();
            
            // Try exact match
            if let Some(location) = index.find_definition(&ident) {
                let lsp_location = LspLocation {
                    uri: location.uri.clone(),
                    range: LspRange {
                        start: LspPosition {
                            line: location.line,
                            character: location.character,
                        },
                        end: LspPosition {
                            line: location.end_line,
                            character: location.end_character,
                        },
                    },
                };
                
                return serde_json::to_string(&lsp_location)
                    .map(Some)
                    .map_err(|e| anyhow!("Failed to serialize location: {}", e));
            }
            
            // Try with common prefixes (class, function, etc.)
            for prefix in &["class ", "function ", "const ", "let ", "struct ", "enum "] {
                let full_name = format!("{}{}", prefix, ident);
                if let Some(location) = index.find_definition(&full_name) {
                    let lsp_location = LspLocation {
                        uri: location.uri.clone(),
                        range: LspRange {
                            start: LspPosition {
                                line: location.line,
                                character: location.character,
                            },
                            end: LspPosition {
                                line: location.end_line,
                                character: location.end_character,
                            },
                        },
                    };
                    
                    return serde_json::to_string(&lsp_location)
                        .map(Some)
                        .map_err(|e| anyhow!("Failed to serialize location: {}", e));
                }
            }
        }
        
        tracing::debug!("No definition found at {}:{}:{}", uri, line, character);
        Ok(None)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn extract_identifier_at_position(text: &str, line: u32, character: u32) -> Option<String> {
    // Convert position to byte offset
    let mut current_line = 0u32;
    let mut line_start = 0;
    
    for (i, ch) in text.char_indices() {
        if current_line == line {
            line_start = i;
            break;
        }
        if ch == '\n' {
            current_line += 1;
        }
    }
    
    if current_line != line {
        return None;
    }
    
    // Find character position in line
    let line_text = text[line_start..]
        .lines()
        .next()
        .unwrap_or("");
    
    let char_pos = character as usize;
    if char_pos >= line_text.len() {
        return None;
    }
    
    // Extract identifier at position
    let bytes = line_text.as_bytes();
    let mut start = char_pos;
    let mut end = char_pos;
    
    // Find start of identifier
    while start > 0 && is_identifier_char(bytes[start - 1]) {
        start -= 1;
    }
    
    // Find end of identifier
    while end < bytes.len() && is_identifier_char(bytes[end]) {
        end += 1;
    }
    
    if start < end {
        Some(String::from_utf8_lossy(&bytes[start..end]).to_string())
    } else {
        None
    }
}

fn is_identifier_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

// ============================================================================
// LSP Types
// ============================================================================

#[derive(Debug, serde::Serialize)]
struct LspLocation {
    uri: String,
    range: LspRange,
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
