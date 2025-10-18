/// Find References (textDocument/references)
/// LSP-011: Find all references to a symbol using index reverse map

use anyhow::{Result, anyhow};
use std::sync::Arc;

/// References provider
pub struct ReferencesProvider {
    doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>,
    symbol_index: Arc<parking_lot::Mutex<super::SymbolIndex>>,
}

impl ReferencesProvider {
    pub fn new(
        doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>,
        symbol_index: Arc<parking_lot::Mutex<super::SymbolIndex>>,
    ) -> Self {
        Self {
            doc_sync,
            symbol_index,
        }
    }

    /// Find all references to symbol at position
    pub async fn find_references(
        &self,
        uri: &str,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<String> {
        // Find symbol at cursor position
        let symbol_name = {
            let index = self.symbol_index.lock();
            index.find_symbol_at_position(uri, line, character)
        };
        
        if let Some(name) = symbol_name {
            let index = self.symbol_index.lock();
            
            // Get all references from index
            let mut locations = Vec::new();
            
            // Add references
            for ref_loc in index.find_references(&name) {
                locations.push(LspLocation {
                    uri: ref_loc.uri.clone(),
                    range: LspRange {
                        start: LspPosition {
                            line: ref_loc.line,
                            character: ref_loc.character,
                        },
                        end: LspPosition {
                            line: ref_loc.end_line,
                            character: ref_loc.end_character,
                        },
                    },
                });
            }
            
            // Optionally include the declaration
            if include_declaration {
                if let Some(def_loc) = index.find_definition(&name) {
                    locations.push(LspLocation {
                        uri: def_loc.uri.clone(),
                        range: LspRange {
                            start: LspPosition {
                                line: def_loc.line,
                                character: def_loc.character,
                            },
                            end: LspPosition {
                                line: def_loc.end_line,
                                character: def_loc.end_character,
                            },
                        },
                    });
                }
            }
            
            // Deduplicate by uri + range
            locations.sort_by(|a, b| {
                a.uri.cmp(&b.uri)
                    .then(a.range.start.line.cmp(&b.range.start.line))
                    .then(a.range.start.character.cmp(&b.range.start.character))
            });
            locations.dedup_by(|a, b| {
                a.uri == b.uri
                    && a.range.start.line == b.range.start.line
                    && a.range.start.character == b.range.start.character
            });
            
            return serde_json::to_string(&locations)
                .map_err(|e| anyhow!("Failed to serialize locations: {}", e));
        }
        
        // Fallback: try to extract identifier and search
        let doc_sync = self.doc_sync.lock();
        let text = doc_sync
            .get_text(uri)
            .ok_or_else(|| anyhow!("Document not found: {}", uri))?;
        
        if let Some(ident) = extract_identifier_at_position(text, line, character) {
            let index = self.symbol_index.lock();
            
            // Try with common prefixes
            for prefix in &["", "class ", "function ", "const ", "let ", "struct ", "enum "] {
                let full_name = if prefix.is_empty() {
                    ident.clone()
                } else {
                    format!("{}{}", prefix, ident)
                };
                
                let mut locations = Vec::new();
                
                for ref_loc in index.find_references(&full_name) {
                    locations.push(LspLocation {
                        uri: ref_loc.uri.clone(),
                        range: LspRange {
                            start: LspPosition {
                                line: ref_loc.line,
                                character: ref_loc.character,
                            },
                            end: LspPosition {
                                line: ref_loc.end_line,
                                character: ref_loc.end_character,
                            },
                        },
                    });
                }
                
                if include_declaration {
                    if let Some(def_loc) = index.find_definition(&full_name) {
                        locations.push(LspLocation {
                            uri: def_loc.uri.clone(),
                            range: LspRange {
                                start: LspPosition {
                                    line: def_loc.line,
                                    character: def_loc.character,
                                },
                                end: LspPosition {
                                    line: def_loc.end_line,
                                    character: def_loc.end_character,
                                },
                            },
                        });
                    }
                }
                
                if !locations.is_empty() {
                    return serde_json::to_string(&locations)
                        .map_err(|e| anyhow!("Failed to serialize locations: {}", e));
                }
            }
        }
        
        tracing::debug!("No references found at {}:{}:{}", uri, line, character);
        Ok("[]".to_string())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn extract_identifier_at_position(text: &str, line: u32, character: u32) -> Option<String> {
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
    
    let line_text = text[line_start..]
        .lines()
        .next()
        .unwrap_or("");
    
    let char_pos = character as usize;
    if char_pos >= line_text.len() {
        return None;
    }
    
    let bytes = line_text.as_bytes();
    let mut start = char_pos;
    let mut end = char_pos;
    
    while start > 0 && is_identifier_char(bytes[start - 1]) {
        start -= 1;
    }
    
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

#[derive(Debug, serde::Serialize, Clone)]
struct LspLocation {
    uri: String,
    range: LspRange,
}

#[derive(Debug, serde::Serialize, Clone)]
struct LspRange {
    start: LspPosition,
    end: LspPosition,
}

#[derive(Debug, serde::Serialize, Clone)]
struct LspPosition {
    line: u32,
    character: u32,
}
